use crate::*;

struct RequestRepaintEvent;

struct GlowRepaintSignal(std::sync::Mutex<glutin::event_loop::EventLoopProxy<RequestRepaintEvent>>);

impl epi::RepaintSignal for GlowRepaintSignal {
    fn request_repaint(&self) {
        self.0.lock().unwrap().send_event(RequestRepaintEvent).ok();
    }
}

#[allow(unsafe_code)]
fn create_display(
    window_builder: glutin::window::WindowBuilder,
    event_loop: &glutin::event_loop::EventLoop<RequestRepaintEvent>,
) -> (
    glutin::WindowedContext<glutin::PossiblyCurrent>,
    glow::Context,
) {
    let gl_window = unsafe {
        glutin::ContextBuilder::new()
            .with_depth_buffer(0)
            .with_srgb(true)
            .with_stencil_buffer(0)
            .with_vsync(true)
            .build_windowed(window_builder, event_loop)
            .unwrap()
            .make_current()
            .unwrap()
    };

    let gl = unsafe { glow::Context::from_loader_function(|s| gl_window.get_proc_address(s)) };

    unsafe {
        use glow::HasContext as _;
        gl.enable(glow::FRAMEBUFFER_SRGB);
    }

    (gl_window, gl)
}

// ----------------------------------------------------------------------------

pub use epi::NativeOptions;

/// Run an egui app
#[allow(unsafe_code)]
pub fn run(app: Box<dyn epi::App>, native_options: &epi::NativeOptions) -> ! {
    let persistence = egui_winit::epi::Persistence::from_app_name(app.name());
    let window_settings = persistence.load_window_settings();
    let window_builder =
        egui_winit::epi::window_builder(native_options, &window_settings).with_title(app.name());
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let (gl_window, gl) = create_display(window_builder, &event_loop);

    let repaint_signal = std::sync::Arc::new(GlowRepaintSignal(std::sync::Mutex::new(
        event_loop.create_proxy(),
    )));

    let mut painter = crate::Painter::new(&gl, None, "")
        .map_err(|error| eprintln!("some OpenGL error occurred {}\n", error))
        .unwrap();
    let mut integration = egui_winit::epi::EpiIntegration::new(
        "egui_glow",
        gl_window.window(),
        &mut painter,
        repaint_signal,
        persistence,
        app,
    );

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

            let (needs_repaint, shapes) = integration.update(gl_window.window(), &mut painter);
            let clipped_meshes = integration.egui_ctx.tessellate(shapes);

            {
                let color = integration.app.clear_color();
                unsafe {
                    use glow::HasContext as _;
                    gl.disable(glow::SCISSOR_TEST);
                    gl.clear_color(color[0], color[1], color[2], color[3]);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }
                painter.upload_egui_texture(&gl, &integration.egui_ctx.texture());
                painter.paint_meshes(
                    gl_window.window().inner_size().into(),
                    &gl,
                    integration.egui_ctx.pixels_per_point(),
                    clipped_meshes,
                );

                gl_window.swap_buffers().unwrap();
            }

            {
                *control_flow = if integration.should_quit() {
                    glutin::event_loop::ControlFlow::Exit
                } else if needs_repaint {
                    gl_window.window().request_redraw();
                    glutin::event_loop::ControlFlow::Poll
                } else {
                    glutin::event_loop::ControlFlow::Wait
                };
            }

            integration.maybe_autosave(gl_window.window());
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            glutin::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            glutin::event::Event::WindowEvent { event, .. } => {
                if let glutin::event::WindowEvent::Focused(new_focused) = event {
                    is_focused = new_focused;
                }

                if let glutin::event::WindowEvent::Resized(physical_size) = event {
                    gl_window.resize(physical_size);
                }

                integration.on_event(&event);
                if integration.should_quit() {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                }

                gl_window.window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
            }
            glutin::event::Event::LoopDestroyed => {
                integration.on_exit(gl_window.window());
                painter.destroy(&gl);
            }
            glutin::event::Event::UserEvent(RequestRepaintEvent) => {
                gl_window.window().request_redraw();
            }
            _ => (),
        }
    });
}
