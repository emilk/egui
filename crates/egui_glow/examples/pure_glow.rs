//! Example how to use pure `egui_glow`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(unsafe_code)]
#![allow(clippy::arc_with_non_send_sync)] // glow::Context was accidentally non-Sync in glow 0.13, but that will be fixed in future releases of glow: https://github.com/grovesNL/glow/commit/c4a5f7151b9b4bbb380faa06ec27415235d1bf7e

use std::num::NonZeroU32;

use egui_winit::winit;

/// The majority of `GlutinWindowContext` is taken from `eframe`
struct GlutinWindowContext {
    window: winit::window::Window,
    gl_context: glutin::context::PossiblyCurrentContext,
    gl_display: glutin::display::Display,
    gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

impl GlutinWindowContext {
    // refactor this function to use `glutin-winit` crate eventually.
    // preferably add android support at the same time.
    #[allow(unsafe_code)]
    unsafe fn new(event_loop: &winit::event_loop::EventLoopWindowTarget<UserEvent>) -> Self {
        use glutin::context::NotCurrentGlContext;
        use glutin::display::GetGlDisplay;
        use glutin::display::GlDisplay;
        use glutin::prelude::GlSurface;
        use rwh_05::HasRawWindowHandle;
        let winit_window_builder = winit::window::WindowBuilder::new()
            .with_resizable(true)
            .with_inner_size(winit::dpi::LogicalSize {
                width: 800.0,
                height: 600.0,
            })
            .with_title("egui_glow example") // Keep hidden until we've painted something. See https://github.com/emilk/egui/pull/2279
            .with_visible(false);

        let config_template_builder = glutin::config::ConfigTemplateBuilder::new()
            .prefer_hardware_accelerated(None)
            .with_depth_size(0)
            .with_stencil_size(0)
            .with_transparency(false);

        log::debug!("trying to get gl_config");
        let (mut window, gl_config) =
            glutin_winit::DisplayBuilder::new() // let glutin-winit helper crate handle the complex parts of opengl context creation
                .with_preference(glutin_winit::ApiPreference::FallbackEgl) // https://github.com/emilk/egui/issues/2520#issuecomment-1367841150
                .with_window_builder(Some(winit_window_builder.clone()))
                .build(
                    event_loop,
                    config_template_builder,
                    |mut config_iterator| {
                        config_iterator.next().expect(
                            "failed to find a matching configuration for creating glutin config",
                        )
                    },
                )
                .expect("failed to create gl_config");
        let gl_display = gl_config.display();
        log::debug!("found gl_config: {:?}", &gl_config);

        let raw_window_handle = window.as_ref().map(|w| w.raw_window_handle());
        log::debug!("raw window handle: {:?}", raw_window_handle);
        let context_attributes =
            glutin::context::ContextAttributesBuilder::new().build(raw_window_handle);
        // by default, glutin will try to create a core opengl context. but, if it is not available, try to create a gl-es context using this fallback attributes
        let fallback_context_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(None))
            .build(raw_window_handle);
        let not_current_gl_context = unsafe {
            gl_display
                    .create_context(&gl_config, &context_attributes)
                    .unwrap_or_else(|_| {
                        log::debug!("failed to create gl_context with attributes: {:?}. retrying with fallback context attributes: {:?}",
                            &context_attributes,
                            &fallback_context_attributes);
                        gl_config
                            .display()
                            .create_context(&gl_config, &fallback_context_attributes)
                            .expect("failed to create context even with fallback attributes")
                    })
        };

        // this is where the window is created, if it has not been created while searching for suitable gl_config
        let window = window.take().unwrap_or_else(|| {
            log::debug!("window doesn't exist yet. creating one now with finalize_window");
            glutin_winit::finalize_window(event_loop, winit_window_builder.clone(), &gl_config)
                .expect("failed to finalize glutin window")
        });
        let (width, height): (u32, u32) = window.inner_size().into();
        let width = NonZeroU32::new(width).unwrap_or(NonZeroU32::MIN);
        let height = NonZeroU32::new(height).unwrap_or(NonZeroU32::MIN);
        let surface_attributes =
            glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
                .build(window.raw_window_handle(), width, height);
        log::debug!(
            "creating surface with attributes: {:?}",
            &surface_attributes
        );
        let gl_surface = unsafe {
            gl_display
                .create_window_surface(&gl_config, &surface_attributes)
                .unwrap()
        };
        log::debug!("surface created successfully: {gl_surface:?}.making context current");
        let gl_context = not_current_gl_context.make_current(&gl_surface).unwrap();

        gl_surface
            .set_swap_interval(
                &gl_context,
                glutin::surface::SwapInterval::Wait(NonZeroU32::MIN),
            )
            .unwrap();

        Self {
            window,
            gl_context,
            gl_display,
            gl_surface,
        }
    }

    fn window(&self) -> &winit::window::Window {
        &self.window
    }

    fn resize(&self, physical_size: winit::dpi::PhysicalSize<u32>) {
        use glutin::surface::GlSurface;
        self.gl_surface.resize(
            &self.gl_context,
            physical_size.width.try_into().unwrap(),
            physical_size.height.try_into().unwrap(),
        );
    }

    fn swap_buffers(&self) -> glutin::error::Result<()> {
        use glutin::surface::GlSurface;
        self.gl_surface.swap_buffers(&self.gl_context)
    }

    fn get_proc_address(&self, addr: &std::ffi::CStr) -> *const std::ffi::c_void {
        use glutin::display::GlDisplay;
        self.gl_display.get_proc_address(addr)
    }
}

#[derive(Debug)]
pub enum UserEvent {
    Redraw(std::time::Duration),
}

fn main() {
    let mut clear_color = [0.1, 0.1, 0.1];

    let event_loop = winit::event_loop::EventLoopBuilder::<UserEvent>::with_user_event()
        .build()
        .unwrap();
    let (gl_window, gl) = create_display(&event_loop);
    let gl = std::sync::Arc::new(gl);

    let mut egui_glow = egui_glow::EguiGlow::new(&event_loop, gl.clone(), None, None);

    let event_loop_proxy = egui::mutex::Mutex::new(event_loop.create_proxy());
    egui_glow
        .egui_ctx
        .set_request_repaint_callback(move |info| {
            event_loop_proxy
                .lock()
                .send_event(UserEvent::Redraw(info.delay))
                .expect("Cannot send event");
        });

    let mut repaint_delay = std::time::Duration::MAX;

    let _ = event_loop.run(move |event, event_loop_window_target| {
        let mut redraw = || {
            let mut quit = false;

            egui_glow.run(gl_window.window(), |egui_ctx| {
                egui::SidePanel::left("my_side_panel").show(egui_ctx, |ui| {
                    ui.heading("Hello World!");
                    if ui.button("Quit").clicked() {
                        quit = true;
                    }
                    ui.color_edit_button_rgb(&mut clear_color);
                });
            });

            if quit {
                event_loop_window_target.exit();
            } else {
                event_loop_window_target.set_control_flow(if repaint_delay.is_zero() {
                    gl_window.window().request_redraw();
                    winit::event_loop::ControlFlow::Poll
                } else if let Some(repaint_after_instant) =
                    std::time::Instant::now().checked_add(repaint_delay)
                {
                    winit::event_loop::ControlFlow::WaitUntil(repaint_after_instant)
                } else {
                    winit::event_loop::ControlFlow::Wait
                });
            }

            {
                unsafe {
                    use glow::HasContext as _;
                    gl.clear_color(clear_color[0], clear_color[1], clear_color[2], 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }

                // draw things behind egui here

                egui_glow.paint(gl_window.window());

                // draw things on top of egui here

                gl_window.swap_buffers().unwrap();
                gl_window.window().set_visible(true);
            }
        };

        match event {
            winit::event::Event::WindowEvent { event, .. } => {
                use winit::event::WindowEvent;
                if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed) {
                    event_loop_window_target.exit();
                    return;
                }

                if matches!(event, WindowEvent::RedrawRequested) {
                    redraw();
                    return;
                }

                if let winit::event::WindowEvent::Resized(physical_size) = &event {
                    gl_window.resize(*physical_size);
                }

                let event_response = egui_glow.on_window_event(gl_window.window(), &event);

                if event_response.repaint {
                    gl_window.window().request_redraw();
                }
            }

            winit::event::Event::UserEvent(UserEvent::Redraw(delay)) => {
                repaint_delay = delay;
            }
            winit::event::Event::LoopExiting => {
                egui_glow.destroy();
            }
            winit::event::Event::NewEvents(winit::event::StartCause::ResumeTimeReached {
                ..
            }) => {
                gl_window.window().request_redraw();
            }

            _ => (),
        }
    });
}

fn create_display(
    event_loop: &winit::event_loop::EventLoopWindowTarget<UserEvent>,
) -> (GlutinWindowContext, glow::Context) {
    let glutin_window_context = unsafe { GlutinWindowContext::new(event_loop) };
    let gl = unsafe {
        glow::Context::from_loader_function(|s| {
            let s = std::ffi::CString::new(s)
                .expect("failed to construct C string from string for gl proc address");

            glutin_window_context.get_proc_address(&s)
        })
    };

    (glutin_window_context, gl)
}
