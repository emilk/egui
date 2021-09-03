//! Example how to use [epi::NativeTexture] with glium.
use epi::NativeTexture;
use glium::glutin;
use std::io::Cursor;

fn create_display(event_loop: &glutin::event_loop::EventLoop<()>) -> glium::Display {
    let window_builder = glutin::window::WindowBuilder::new()
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize {
            width: 800.0,
            height: 600.0,
        })
        .with_title("egui_glium example");

    let context_builder = glutin::ContextBuilder::new()
        .with_depth_buffer(0)
        .with_srgb(true)
        .with_stencil_buffer(0)
        .with_vsync(true);

    glium::Display::new(window_builder, context_builder, event_loop).unwrap()
}

fn main() {
    let event_loop = glutin::event_loop::EventLoop::with_user_event();
    let display = create_display(&event_loop);

    let mut egui = egui_glium::EguiGlium::new(&display);
    // load image by image crate
    let image = image::load(
        Cursor::new(&include_bytes!("rust-logo-blk.png")[..]),
        image::ImageFormat::Png,
    )
    .unwrap()
    .to_rgba8();
    let image_dimensions = image.dimensions();
    // mark as image
    let image =
        glium::texture::RawImage2d::from_raw_rgba(image.into_raw(), image_dimensions);
    //load to gpu memory
    let native_texture = glium::texture::SrgbTexture2d::new(&display, image).unwrap();
    // allocate egui's texture id for GL texture
    let texture_id = egui
        .ctx_and_painter_mut()
        .1
        .bind_native_texture(native_texture);
    event_loop.run(move |event, _, control_flow| {
        let mut redraw = || {
            egui.begin_frame(&display);

            let mut quit = false;

            egui::SidePanel::left("my_side_panel").show(egui.ctx(), |ui| {
                ui.heading("");
                if ui.button("Quit").clicked() {
                    quit = true;
                }

            });
            egui::Window::new("NativeTextureDisplay").show(egui.ctx(), |ui| {
                ui.image(texture_id, egui::Vec2::new(128.0, 128.0));
            });
            let (needs_repaint, shapes) = egui.end_frame(&display);

            *control_flow = if quit {
                glutin::event_loop::ControlFlow::Exit
            } else if needs_repaint {
                display.gl_window().window().request_redraw();
                glutin::event_loop::ControlFlow::Poll
            } else {
                glutin::event_loop::ControlFlow::Wait
            };

            {
                use glium::Surface as _;
                let mut target = display.draw();

                let clear_color = egui::Rgba::from_rgb(0.1, 0.3, 0.2);
                target.clear_color(
                    clear_color[0],
                    clear_color[1],
                    clear_color[2],
                    clear_color[3],
                );

                // draw things behind egui here

                egui.paint(&display, &mut target, shapes);

                // draw things on top of egui here

                target.finish().unwrap();
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
                    *control_flow = glium::glutin::event_loop::ControlFlow::Exit;
                }

                egui.on_event(&event);

                display.gl_window().window().request_redraw(); // TODO: ask egui if the events warrants a repaint instead
            }

            _ => (),
        }
    });
}
