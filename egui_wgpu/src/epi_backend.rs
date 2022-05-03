#![allow(unsafe_code)]

use crate::renderer::{wgpu, RenderPass, ScreenDescriptor};
use egui::FullOutput;
use egui_winit::winit;

/// A custom event type for the winit app.
enum Event {
    RequestRedraw,
}

/// This is the repaint signal type that egui needs for requesting a repaint from another thread.
/// It sends the custom RequestRedraw event to the winit event loop.
struct WgpuRepaintSignal(std::sync::Mutex<winit::event_loop::EventLoopProxy<Event>>);

impl epi::backend::RepaintSignal for WgpuRepaintSignal {
    fn request_repaint(&self) {
        self.0.lock().unwrap().send_event(Event::RequestRedraw).ok();
    }
}

/// Run an egui app
pub fn run(app_name: &str, native_options: &epi::NativeOptions, app_creator: epi::AppCreator) -> ! {
    let storage = epi_integration::create_storage(app_name);
    let window_settings = epi_integration::load_window_settings(storage.as_deref());
    let event_loop = winit::event_loop::EventLoop::with_user_event();
    let window = epi_integration::window_builder(native_options, &window_settings)
        .with_title(app_name)
        .build(&event_loop)
        .unwrap();

    // TODO: Unclear what to do with this. It used to be an argument to `EpiIntegration::new`.
    let repaint_signal = std::sync::Arc::new(WgpuRepaintSignal(std::sync::Mutex::new(
        event_loop.create_proxy(),
    )));

    // GL is "unsupported" as backend, but it is required to run in a remote desktop environment.
    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY | wgpu::Backends::GL);
    let surface = unsafe { instance.create_surface(&window) };

    // WGPU 0.11+ support force fallback (if HW implementation not supported), set it to true or false (optional).
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::default(),
            limits: wgpu::Limits::default(),
            label: None,
        },
        None,
    ))
    .unwrap();

    let size = window.inner_size();
    let surface_format = surface.get_preferred_format(&adapter).unwrap();
    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width as u32,
        height: size.height as u32,
        present_mode: wgpu::PresentMode::Fifo,
    };
    surface.configure(&device, &surface_config);

    let mut integration = egui_winit::EpiIntegration::new(
        "egui_wgpu",
        device.limits().max_texture_dimension_2d as usize,
        &window,
        storage,
    );
    // We use the egui_wgpu_backend crate as the render backend.
    let mut egui_rpass = RenderPass::new(&device, surface_format, 1);

    let mut is_focused = true;

    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            if !is_focused {
                // On Mac, a minimized Window uses up all CPU: https://github.com/emilk/egui/issues/325
                // We can't know if we are minimized: https://github.com/rust-windowing/winit/issues/208
                // But we know if we are focused (in foreground). When minimized, we are not focused.
                // However, a user may want an egui with an animation in the background,
                // so we still need to repaint quite fast.
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            let output_frame = match surface.get_current_texture() {
                Ok(frame) => frame,
                Err(wgpu::SurfaceError::Outdated) => {
                    // This error occurs when the app is minimized on Windows.
                    // Silently return here to prevent spamming the console with:
                    // "The underlying surface has changed, and therefore the swap chain must be updated"
                    return;
                }
                Err(e) => {
                    eprintln!("Dropped frame with error: {}", e);
                    return;
                }
            };
            let output_view = output_frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());

            let FullOutput {
                platform_output,
                needs_repaint,
                textures_delta,
                shapes,
            } = integration.update(&window);

            integration.handle_platform_output(&window, platform_output);

            for (id, image_delta) in textures_delta.set {
                egui_rpass.update_texture(id, image_delta)
            }
            for id in textures_delta.free {
                egui_rpass.free_texture(id);
            }
            let clipped_meshes = integration.egui_ctx.tessellate(shapes);

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });

            // Upload all resources for the GPU.
            let screen_descriptor = ScreenDescriptor {
                physical_width: surface_config.width,
                physical_height: surface_config.height,
                scale_factor: integration.egui_ctx.pixels_per_point(),
            };

            egui_rpass.update_textures(&device, &queue);
            egui_rpass.update_buffers(&device, &queue, &clipped_meshes, &screen_descriptor);

            let clear_color = integration.app.clear_color();

            // Record all render passes.
            egui_rpass
                .execute(
                    &mut encoder,
                    &output_view,
                    &clipped_meshes,
                    &screen_descriptor,
                    Some(wgpu::Color {
                        r: clear_color.r() as f64,
                        g: clear_color.g() as f64,
                        b: clear_color.b() as f64,
                        a: clear_color.a() as f64,
                    }),
                )
                .unwrap();

            // Submit the commands.
            queue.submit(std::iter::once(encoder.finish()));

            // Redraw egui
            output_frame.present();

            integration.maybe_autosave(&window);

            *control_flow = if integration.should_quit() {
                winit::event_loop::ControlFlow::Exit
            } else if needs_repaint {
                window.request_redraw();
                winit::event_loop::ControlFlow::Poll
            } else {
                winit::event_loop::ControlFlow::Wait
            };
        };

        match event {
            winit::event::Event::WindowEvent { event, .. } => {
                match event {
                    winit::event::WindowEvent::Focused(new_focused) => {
                        is_focused = new_focused;
                    }
                    winit::event::WindowEvent::Resized(size) => {
                        // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                        // See: https://github.com/rust-windowing/winit/issues/208
                        // This solves an issue where the app would panic when minimizing on Windows.
                        if size.width > 0 && size.height > 0 {
                            surface_config.width = size.width;
                            surface_config.height = size.height;
                            surface.configure(&device, &surface_config);
                        }
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                    _ => {}
                };
                integration.on_event(&event);
                if integration.should_quit() {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                }
                window.request_redraw();
            }
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            winit::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            winit::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),
            winit::event::Event::UserEvent(Event::RequestRedraw) => window.request_redraw(),
            winit::event::Event::LoopDestroyed => {
                integration.on_exit(&window);
            }
            _ => (),
        }
    });
}
