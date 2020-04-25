#![deny(warnings)]
#[allow(clippy::single_match)]
use std::time::{Duration, Instant};

use {
    emigui::{
        example_app::ExampleApp,
        label,
        math::*,
        widgets::{Button, Label},
        Align, CursorIcon, Emigui, Window,
    },
    emigui_glium::Painter,
    glium::glutin,
};

fn main() {
    let mut events_loop = glutin::EventsLoop::new();
    let window = glutin::WindowBuilder::new().with_title("Emigui example");
    let context = glutin::ContextBuilder::new();
    let display = glium::Display::new(window, context, &events_loop).unwrap();

    display
        .gl_window()
        .set_inner_size(glutin::dpi::LogicalSize {
            width: 1024.0,
            height: 800.0,
        });
    display.gl_window().set_position((0, 24).into()); // Useful when ddeveloping and constantly restarting it

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

    // used to keep track of time for animations
    let start_time = Instant::now();

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

        raw_input.time = start_time.elapsed().as_nanos() as f64 * 1e-9;
        raw_input.scroll_delta = vec2(0.0, 0.0);

        events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => quit = true,

                glutin::WindowEvent::Resized(glutin::dpi::LogicalSize { width, height }) => {
                    raw_input.screen_size = vec2(width as f32, height as f32);
                }
                glutin::WindowEvent::MouseInput { state, .. } => {
                    raw_input.mouse_down = state == glutin::ElementState::Pressed;
                }
                glutin::WindowEvent::CursorMoved { position, .. } => {
                    raw_input.mouse_pos = Some(pos2(position.x as f32, position.y as f32));
                }
                glutin::WindowEvent::KeyboardInput { input, .. } => {
                    if input.virtual_keycode == Some(glutin::VirtualKeyCode::Q)
                        && input.modifiers.logo
                    {
                        quit = true;
                    }
                }
                glutin::WindowEvent::MouseWheel { delta, .. } => {
                    match delta {
                        glutin::MouseScrollDelta::LineDelta(x, y) => {
                            raw_input.scroll_delta = vec2(x, y) * 24.0;
                        }
                        glutin::MouseScrollDelta::PixelDelta(delta) => {
                            // Actually point delta
                            raw_input.scroll_delta = vec2(delta.x as f32, delta.y as f32);
                        }
                    }
                }
                _ => {
                    // dbg!(event);
                }
            },
            _ => (),
        });

        emigui.begin_frame(raw_input);
        let mut region = emigui.background_region();
        let mut region = region.centered_column(region.available_width().min(480.0));
        region.set_align(Align::Min);
        region.add(label!("Emigui running inside of Glium").text_style(emigui::TextStyle::Heading));
        if region.add(Button::new("Quit")).clicked {
            quit = true;
        }

        // TODO: Make it even simpler to show a window

        Window::new("Examples")
            .default_pos(pos2(50.0, 100.0))
            .default_size(vec2(300.0, 400.0))
            .show(region.ctx(), |region| {
                example_app.ui(region);
            });

        Window::new("Emigui settings")
            .default_pos(pos2(450.0, 100.0))
            .default_size(vec2(450.0, 500.0))
            .show(region.ctx(), |region| {
                emigui.ui(region);
            });

        let (output, paint_batches) = emigui.end_frame();
        painter.paint_batches(&display, paint_batches, emigui.texture());

        let cursor = match output.cursor_icon {
            CursorIcon::Default => glutin::MouseCursor::Default,
            CursorIcon::PointingHand => glutin::MouseCursor::Hand,
            CursorIcon::ResizeNwSe => glutin::MouseCursor::NwseResize,
        };

        if let Some(url) = output.open_url {
            if let Err(err) = webbrowser::open(&url) {
                eprintln!("Failed to open url: {}", err); // TODO show error in imgui
            }
        }

        display.gl_window().set_cursor(cursor);
    }
}
