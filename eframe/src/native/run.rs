use super::epi_integration;
use crate::epi;
use egui_winit::winit;

struct RequestRepaintEvent;

#[cfg(feature = "glow")]
#[allow(unsafe_code)]
fn create_display(
    native_options: &NativeOptions,
    window_builder: winit::window::WindowBuilder,
    event_loop: &winit::event_loop::EventLoop<RequestRepaintEvent>,
) -> (
    glutin::WindowedContext<glutin::PossiblyCurrent>,
    glow::Context,
) {
    crate::profile_function!();

    use crate::HardwareAcceleration;

    let hardware_acceleration = match native_options.hardware_acceleration {
        HardwareAcceleration::Required => Some(true),
        HardwareAcceleration::Preferred => None,
        HardwareAcceleration::Off => Some(false),
    };

    let gl_window = unsafe {
        glutin::ContextBuilder::new()
            .with_hardware_acceleration(hardware_acceleration)
            .with_depth_buffer(native_options.depth_buffer)
            .with_multisampling(native_options.multisampling)
            .with_srgb(true)
            .with_stencil_buffer(native_options.stencil_buffer)
            .with_vsync(native_options.vsync)
            .build_windowed(window_builder, event_loop)
            .unwrap()
            .make_current()
            .unwrap()
    };

    let gl = unsafe { glow::Context::from_loader_function(|s| gl_window.get_proc_address(s)) };

    (gl_window, gl)
}

// ----------------------------------------------------------------------------

pub use epi::NativeOptions;

/// Run an egui app
#[cfg(feature = "glow")]
pub fn run_glow(
    app_name: &str,
    native_options: &epi::NativeOptions,
    app_creator: epi::AppCreator,
) -> ! {
    let storage = epi_integration::create_storage(app_name);
    let window_settings = epi_integration::load_window_settings(storage.as_deref());
    let event_loop = winit::event_loop::EventLoop::with_user_event();

    let window_builder =
        epi_integration::window_builder(native_options, &window_settings).with_title(app_name);
    let (gl_window, gl) = create_display(native_options, window_builder, &event_loop);
    let gl = std::sync::Arc::new(gl);

    let mut painter = egui_glow::Painter::new(gl.clone(), None, "")
        .unwrap_or_else(|error| panic!("some OpenGL error occurred {}\n", error));

    let mut integration = epi_integration::EpiIntegration::new(
        &event_loop,
        painter.max_texture_side(),
        gl_window.window(),
        storage,
        Some(gl.clone()),
        #[cfg(feature = "wgpu")]
        None,
    );

    {
        let event_loop_proxy = egui::mutex::Mutex::new(event_loop.create_proxy());
        integration.egui_ctx.set_request_repaint_callback(move || {
            event_loop_proxy.lock().send_event(RequestRepaintEvent).ok();
        });
    }

    let mut app = app_creator(&epi::CreationContext {
        egui_ctx: integration.egui_ctx.clone(),
        integration_info: integration.frame.info(),
        storage: integration.frame.storage(),
        gl: Some(gl.clone()),
        #[cfg(feature = "wgpu")]
        render_state: None,
    });

    if app.warm_up_enabled() {
        integration.warm_up(app.as_mut(), gl_window.window());
    }

    let mut is_focused = true;

    event_loop.run(move |event, _, control_flow| {
        let window = gl_window.window();

        let mut redraw = || {
            #[cfg(feature = "puffin")]
            puffin::GlobalProfiler::lock().new_frame();
            crate::profile_scope!("frame");

            let screen_size_in_pixels: [u32; 2] = window.inner_size().into();

            egui_glow::painter::clear(
                &gl,
                screen_size_in_pixels,
                app.clear_color(&integration.egui_ctx.style().visuals),
            );

            let egui::FullOutput {
                platform_output,
                needs_repaint,
                textures_delta,
                shapes,
            } = integration.update(app.as_mut(), window);

            integration.handle_platform_output(window, platform_output);

            let clipped_primitives = {
                crate::profile_scope!("tessellate");
                integration.egui_ctx.tessellate(shapes)
            };

            painter.paint_and_update_textures(
                screen_size_in_pixels,
                integration.egui_ctx.pixels_per_point(),
                &clipped_primitives,
                &textures_delta,
            );

            integration.post_rendering(app.as_mut(), window);

            {
                crate::profile_scope!("swap_buffers");
                gl_window.swap_buffers().unwrap();
            }

            *control_flow = if integration.should_quit() {
                winit::event_loop::ControlFlow::Exit
            } else if needs_repaint {
                window.request_redraw();
                winit::event_loop::ControlFlow::Poll
            } else {
                winit::event_loop::ControlFlow::Wait
            };

            integration.maybe_autosave(app.as_mut(), window);

            if !is_focused {
                // On Mac, a minimized Window uses up all CPU: https://github.com/emilk/egui/issues/325
                // We can't know if we are minimized: https://github.com/rust-windowing/winit/issues/208
                // But we know if we are focused (in foreground). When minimized, we are not focused.
                // However, a user may want an egui with an animation in the background,
                // so we still need to repaint quite fast.
                crate::profile_scope!("bg_sleep");
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            winit::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            winit::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            winit::event::Event::WindowEvent { event, .. } => {
                match &event {
                    winit::event::WindowEvent::Focused(new_focused) => {
                        is_focused = *new_focused;
                    }
                    winit::event::WindowEvent::Resized(physical_size) => {
                        // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                        // See: https://github.com/rust-windowing/winit/issues/208
                        // This solves an issue where the app would panic when minimizing on Windows.
                        if physical_size.width > 0 && physical_size.height > 0 {
                            gl_window.resize(*physical_size);
                        }
                    }
                    winit::event::WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        gl_window.resize(**new_inner_size);
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                    _ => {}
                }

                integration.on_event(app.as_mut(), &event);
                if integration.should_quit() {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                }
                window.request_redraw(); // TODO(emilk): ask egui if the events warrants a repaint instead
            }
            winit::event::Event::LoopDestroyed => {
                integration.save(&mut *app, window);
                app.on_exit(Some(&gl));
                painter.destroy();
            }
            winit::event::Event::UserEvent(RequestRepaintEvent) => window.request_redraw(),
            _ => (),
        }
    });
}

// TODO(emilk): merge with with the clone above
/// Run an egui app
#[cfg(feature = "wgpu")]
pub fn run_wgpu(
    app_name: &str,
    native_options: &epi::NativeOptions,
    app_creator: epi::AppCreator,
) -> ! {
    let storage = epi_integration::create_storage(app_name);
    let window_settings = epi_integration::load_window_settings(storage.as_deref());
    let event_loop = winit::event_loop::EventLoop::with_user_event();

    let window = epi_integration::window_builder(native_options, &window_settings)
        .with_title(app_name)
        .build(&event_loop)
        .unwrap();

    // SAFETY: `window` must outlive `painter`.
    #[allow(unsafe_code)]
    let mut painter = unsafe {
        let mut painter = egui_wgpu::winit::Painter::new(
            wgpu::Backends::PRIMARY | wgpu::Backends::GL,
            wgpu::PowerPreference::HighPerformance,
            wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::default(),
                limits: wgpu::Limits::default(),
            },
            wgpu::PresentMode::Fifo,
            native_options.multisampling.max(1) as _,
        );
        #[cfg(not(target_os = "android"))]
        painter.set_window(Some(&window));
        painter
    };

    let render_state = painter.get_render_state().expect("Uninitialized");

    let mut integration = epi_integration::EpiIntegration::new(
        &event_loop,
        painter.max_texture_side().unwrap_or(2048),
        &window,
        storage,
        #[cfg(feature = "glow")]
        None,
        Some(render_state.clone()),
    );

    {
        let event_loop_proxy = egui::mutex::Mutex::new(event_loop.create_proxy());
        integration.egui_ctx.set_request_repaint_callback(move || {
            event_loop_proxy.lock().send_event(RequestRepaintEvent).ok();
        });
    }

    let mut app = app_creator(&epi::CreationContext {
        egui_ctx: integration.egui_ctx.clone(),
        integration_info: integration.frame.info(),
        storage: integration.frame.storage(),
        #[cfg(feature = "glow")]
        gl: None,
        render_state: Some(render_state),
    });

    if app.warm_up_enabled() {
        integration.warm_up(app.as_mut(), &window);
    }

    let mut is_focused = true;

    event_loop.run(move |event, _, control_flow| {
        let window = &window;

        let mut redraw = || {
            #[cfg(feature = "puffin")]
            puffin::GlobalProfiler::lock().new_frame();
            crate::profile_scope!("frame");

            let egui::FullOutput {
                platform_output,
                needs_repaint,
                textures_delta,
                shapes,
            } = integration.update(app.as_mut(), window);

            integration.handle_platform_output(window, platform_output);

            let clipped_primitives = {
                crate::profile_scope!("tessellate");
                integration.egui_ctx.tessellate(shapes)
            };

            painter.paint_and_update_textures(
                integration.egui_ctx.pixels_per_point(),
                app.clear_color(&integration.egui_ctx.style().visuals),
                &clipped_primitives,
                &textures_delta,
            );

            *control_flow = if integration.should_quit() {
                winit::event_loop::ControlFlow::Exit
            } else if needs_repaint {
                window.request_redraw();
                winit::event_loop::ControlFlow::Poll
            } else {
                winit::event_loop::ControlFlow::Wait
            };

            integration.maybe_autosave(app.as_mut(), window);

            if !is_focused {
                // On Mac, a minimized Window uses up all CPU: https://github.com/emilk/egui/issues/325
                // We can't know if we are minimized: https://github.com/rust-windowing/winit/issues/208
                // But we know if we are focused (in foreground). When minimized, we are not focused.
                // However, a user may want an egui with an animation in the background,
                // so we still need to repaint quite fast.
                crate::profile_scope!("bg_sleep");
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            winit::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            winit::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            #[cfg(target_os = "android")]
            winit::event::Event::Resumed => unsafe {
                painter.set_window(Some(&window));
            },
            #[cfg(target_os = "android")]
            winit::event::Event::Paused => unsafe {
                painter.set_window(None);
            },

            winit::event::Event::WindowEvent { event, .. } => {
                match &event {
                    winit::event::WindowEvent::Focused(new_focused) => {
                        is_focused = *new_focused;
                    }
                    winit::event::WindowEvent::Resized(physical_size) => {
                        // Resize with 0 width and height is used by winit to signal a minimize event on Windows.
                        // See: https://github.com/rust-windowing/winit/issues/208
                        // This solves an issue where the app would panic when minimizing on Windows.
                        if physical_size.width > 0 && physical_size.height > 0 {
                            painter.on_window_resized(physical_size.width, physical_size.height);
                        }
                    }
                    winit::event::WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        painter.on_window_resized(new_inner_size.width, new_inner_size.height);
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    }
                    _ => {}
                };

                integration.on_event(app.as_mut(), &event);
                if integration.should_quit() {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                }
                window.request_redraw(); // TODO(emilk): ask egui if the events warrants a repaint instead
            }
            winit::event::Event::LoopDestroyed => {
                integration.save(&mut *app, window);

                #[cfg(feature = "glow")]
                app.on_exit(None);

                #[cfg(not(feature = "glow"))]
                app.on_exit();

                painter.destroy();
            }
            winit::event::Event::UserEvent(RequestRepaintEvent) => window.request_redraw(),
            _ => (),
        }
    });
}
