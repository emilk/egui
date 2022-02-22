use glium::glutin;

use crate::*;

struct RequestRepaintEvent;

struct GliumRepaintSignal(
    std::sync::Mutex<glutin::event_loop::EventLoopProxy<RequestRepaintEvent>>,
);

impl epi::backend::RepaintSignal for GliumRepaintSignal {
    fn request_repaint(&self) {
        self.0.lock().unwrap().send_event(RequestRepaintEvent).ok();
    }
}

fn create_display(
    window_builder: glutin::window::WindowBuilder,
    event_loop: &glutin::event_loop::EventLoop<RequestRepaintEvent>,
) -> glium::Display {
    let context_builder = glutin::ContextBuilder::new()
        .with_depth_buffer(0)
        .with_srgb(true)
        .with_stencil_buffer(0)
        .with_vsync(true);

    glium::Display::new(window_builder, context_builder, event_loop).unwrap()
}

// ----------------------------------------------------------------------------

pub use epi::NativeOptions;

/// Run an egui app
pub fn run(app: Box<dyn epi::App>, native_options: &epi::NativeOptions) -> ! {
    let persistence = egui_winit::epi::Persistence::from_app_name(app.name());
    let window_settings = persistence.load_window_settings();
    let window_builder =
        egui_winit::epi::window_builder(native_options, &window_settings).with_title(app.name());
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let display = create_display(window_builder, &event_loop);

    let repaint_signal = std::sync::Arc::new(GliumRepaintSignal(std::sync::Mutex::new(
        event_loop.create_proxy(),
    )));

    let mut painter = crate::Painter::new(&display);
    let mut integration = egui_winit::epi::EpiIntegration::new(
        "egui_glium",
        painter.max_texture_side(),
        display.gl_window().window(),
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

            let egui::FullOutput {
                platform_output,
                needs_repaint,
                textures_delta,
                shapes,
            } = integration.update(display.gl_window().window());

            integration.handle_platform_output(display.gl_window().window(), platform_output);

            let clipped_meshes = integration.egui_ctx.tessellate(shapes);

            // paint:
            {
                use glium::Surface as _;
                let mut target = display.draw();
                let color = integration.app.clear_color();
                target.clear_color(color[0], color[1], color[2], color[3]);

                painter.paint_and_update_textures(
                    &display,
                    &mut target,
                    integration.egui_ctx.pixels_per_point(),
                    clipped_meshes,
                    &textures_delta,
                );

                target.finish().unwrap();
            }

            {
                *control_flow = if integration.should_quit() {
                    glutin::event_loop::ControlFlow::Exit
                } else if needs_repaint {
                    display.gl_window().window().request_redraw();
                    glutin::event_loop::ControlFlow::Poll
                } else {
                    glutin::event_loop::ControlFlow::Wait
                };
            }

            integration.maybe_autosave(display.gl_window().window());
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

                integration.on_event(&event);
                if integration.should_quit() {
                    *control_flow = glium::glutin::event_loop::ControlFlow::Exit;
                }

                display.gl_window().window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
            }
            glutin::event::Event::LoopDestroyed => {
                integration.on_exit(display.gl_window().window());
            }
            glutin::event::Event::UserEvent(RequestRepaintEvent) => {
                display.gl_window().window().request_redraw();
            }
            _ => (),
        }
    });
}
