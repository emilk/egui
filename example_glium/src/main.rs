#![deny(warnings)]

use std::time::{Duration, Instant};

use {
    emigui::{
        example_app::ExampleApp,
        label,
        math::vec2,
        widgets::{Button, Label},
        Align, Emigui, Window,
    },
    emigui_glium::Painter,
    glium::glutin,
};

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_title("Emigui example");
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    let pixels_per_point = display.gl_window().get_hidpi_factor() as f32;

    let mut emigui = Emigui::new(pixels_per_point);
    let mut painter = Painter::new(&display);

    let mut raw_input = emigui::RawInput {
        screen_size: {
            let (width, height) = display.get_framebuffer_dimensions();
            vec2(width as f32, height as f32) / pixels_per_point
        },
        pixels_per_point,
        ..Default::default()
    };

    let mut quit = false;

    let mut frame_start = Instant::now();

    let mut example_app = ExampleApp::default();

    while !quit {
        {
            // Keep smooth frame rate. TODO: proper vsync
            let frame_duration = frame_start.elapsed();
            if frame_duration < Duration::from_millis(16) {
                std::thread::sleep(Duration::from_millis(16) - frame_duration);
            }
            frame_start = Instant::now();
        }

        events_loop.poll_events(|event| {
            match event {
                glutin::Event::WindowEvent { event, .. } => match event {
                    glutin::WindowEvent::CloseRequested => quit = true,

                    glutin::WindowEvent::Resized(glutin::dpi::LogicalSize { width, height }) => {
                        raw_input.screen_size =
                            vec2(width as f32, height as f32) / pixels_per_point;
                    }
                    glutin::WindowEvent::MouseInput { state, .. } => {
                        raw_input.mouse_down = state == glutin::ElementState::Pressed;
                    }
                    glutin::WindowEvent::CursorMoved { position, .. } => {
                        raw_input.mouse_pos = Some(vec2(position.x as f32, position.y as f32));
                    }
                    glutin::WindowEvent::KeyboardInput { input, .. } => {
                        if input.virtual_keycode == Some(glutin::VirtualKeyCode::Q)
                            && input.modifiers.logo
                        {
                            quit = true;
                        }
                    }
                    _ => {
                        // dbg!(event);
                    }
                },
                _ => (),
            }
        });

        emigui.new_frame(raw_input);
        let mut region = emigui.whole_screen_region();
        let mut region = region.left_column(region.width().min(480.0));
        region.set_align(Align::Min);
        region.add(label!("Emigui running inside of Glium").text_style(emigui::TextStyle::Heading));
        if region.add(Button::new("Quit")).clicked {
            quit = true;
        }
        example_app.ui(&mut region);
        emigui.ui(&mut region);

        // TODO: Make it simpler to show a window
        Window::new("Test window").show(region.data().clone(), |region| {
            region.add(label!("Grab the window and move it around!"));
        });

        let mesh = emigui.paint();
        painter.paint(&display, mesh, emigui.texture());
    }
}
