//! Example how to use pure `egui_glow` without [`epi`].

fn create_display(
    event_loop: &glutin::event_loop::EventLoop<()>,
) -> (
    glutin::WindowedContext<glutin::PossiblyCurrent>,
    glow::Context,
) {
    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize {
            width: 800.0,
            height: 600.0,
        })
        .with_title("egui_glow example");

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

fn main() {
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let (gl_window, gl) = create_display(&event_loop);

    let mut egui = egui_glow::EguiGlow::new(&gl_window, &gl);

    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            egui.begin_frame(gl_window.window());

            let mut quit = false;

            egui::SidePanel::left("my_side_panel").show(egui.ctx(), |ui| {
                ui.heading("Hello World!");
                if ui.button("Quit").clicked() {
                    quit = true;
                }

                egui::ComboBox::from_label("Version")
                    .width(150.0)
                    .selected_text("foo")
                    .show_ui(ui, |ui| {
                        egui::CollapsingHeader::new("Dev")
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.label("contents");
                            });
                    });
            });

            let (needs_repaint, shapes) = egui.end_frame(gl_window.window());

            *control_flow = if quit {
                glutin::event_loop::ControlFlow::Exit
            } else if needs_repaint {
                gl_window.window().request_redraw();
                glutin::event_loop::ControlFlow::Poll
            } else {
                glutin::event_loop::ControlFlow::Wait
            };

            {
                let clear_color = egui::Rgba::from_rgb(0.1, 0.3, 0.2);
                unsafe {
                    use glow::HasContext as _;
                    gl.clear_color(
                        clear_color[0],
                        clear_color[1],
                        clear_color[2],
                        clear_color[3],
                    );
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }

                // draw things behind egui here

                egui.paint(&gl_window, &gl, shapes);

                // draw things on top of egui here

                gl_window.swap_buffers().unwrap();
            }
        };

        match event {
            // Platform-dependent event handlers to workaround a winit bug
            // See: https://github.com/rust-windowing/winit/issues/987
            // See: https://github.com/rust-windowing/winit/issues/1619
            glutin::event::Event::RedrawEventsCleared if cfg!(windows) => redraw(),
            glutin::event::Event::RedrawRequested(_) if !cfg!(windows) => redraw(),

            glutin::event::Event::WindowEvent { event, .. } => {
                if egui.is_quit_event(&event) {
                    *control_flow = glutin::event_loop::ControlFlow::Exit;
                }

                if let glutin::event::WindowEvent::Resized(physical_size) = event {
                    gl_window.resize(physical_size);
                }

                egui.on_event(&event);

                gl_window.window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
            }
            glutin::event::Event::LoopDestroyed => {
                egui.destroy(&gl);
            }

            _ => (),
        }
    });
}
