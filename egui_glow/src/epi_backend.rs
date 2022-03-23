use crate::*;
use egui_winit::winit;

struct RequestRepaintEvent;

#[allow(unsafe_code)]
fn create_display(
    native_options: &NativeOptions,
    window_builder: winit::window::WindowBuilder,
    event_loop: &winit::event_loop::EventLoop<RequestRepaintEvent>,
) -> (
    glutin::WindowedContext<glutin::PossiblyCurrent>,
    glow::Context,
) {
    let gl_window = unsafe {
        glutin::ContextBuilder::new()
            .with_depth_buffer(native_options.depth_buffer)
            .with_multisampling(native_options.multisampling)
            .with_srgb(native_options.vsync)
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
#[allow(unsafe_code)]
pub fn run(app_name: &str, native_options: &epi::NativeOptions, app_creator: epi::AppCreator) -> ! {
    let persistence = egui_winit::epi::Persistence::from_app_name(app_name);
    let window_settings = persistence.load_window_settings();
    let window_builder =
        egui_winit::epi::window_builder(native_options, &window_settings).with_title(app_name);
    let event_loop = winit::event_loop::EventLoop::with_user_event();
    let (gl_window, gl) = create_display(native_options, window_builder, &event_loop);
    let gl = std::rc::Rc::new(gl);

    let mut painter = crate::Painter::new(gl.clone(), None, "")
        .unwrap_or_else(|error| panic!("some OpenGL error occurred {}\n", error));

    let mut integration = egui_winit::epi::EpiIntegration::new(
        "egui_glow",
        painter.max_texture_side(),
        gl_window.window(),
        persistence,
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
        storage: integration.persistence.storage(),
        gl: gl.clone(),
    });

    if app.warm_up_enabled() {
        integration.warm_up(app.as_mut(), gl_window.window());
    }

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

            let egui::FullOutput {
                platform_output,
                needs_repaint,
                textures_delta,
                shapes,
            } = integration.update(app.as_mut(), gl_window.window());

            integration.handle_platform_output(gl_window.window(), platform_output);

            let clipped_primitives = integration.egui_ctx.tessellate(shapes);

            // paint:
            {
                let color = app.clear_color();
                unsafe {
                    use glow::HasContext as _;
                    gl.disable(glow::SCISSOR_TEST);
                    gl.clear_color(color[0], color[1], color[2], color[3]);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }
                painter.paint_and_update_textures(
                    gl_window.window().inner_size().into(),
                    integration.egui_ctx.pixels_per_point(),
                    &clipped_primitives,
                    &textures_delta,
                );

                gl_window.swap_buffers().unwrap();
            }

            {
                *control_flow = if integration.should_quit() {
                    winit::event_loop::ControlFlow::Exit
                } else if needs_repaint {
                    gl_window.window().request_redraw();
                    winit::event_loop::ControlFlow::Poll
                } else {
                    winit::event_loop::ControlFlow::Wait
                };
            }

            integration.maybe_autosave(app.as_mut(), gl_window.window());
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            winit::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            winit::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            winit::event::Event::WindowEvent { event, .. } => {
                if let winit::event::WindowEvent::Focused(new_focused) = event {
                    is_focused = new_focused;
                }

                if let winit::event::WindowEvent::Resized(physical_size) = event {
                    gl_window.resize(physical_size);
                }

                integration.on_event(app.as_mut(), &event);
                if integration.should_quit() {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                }

                gl_window.window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
            }
            winit::event::Event::LoopDestroyed => {
                integration
                    .persistence
                    .save(&mut *app, &integration.egui_ctx, gl_window.window());
                app.on_exit(&gl);
                painter.destroy();
            }
            winit::event::Event::UserEvent(RequestRepaintEvent) => {
                gl_window.window().request_redraw();
            }
            _ => (),
        }
    });
}
