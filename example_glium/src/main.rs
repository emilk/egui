#![deny(warnings)]
#[allow(clippy::single_match)]
use std::time::{Duration, Instant};

use {
    clipboard::{ClipboardContext, ClipboardProvider},
    emigui::{containers::*, example_app::ExampleApp, widgets::*, *},
    emigui_glium::Painter,
    glium::glutin::{self, VirtualKeyCode},
};

// TODO: move more code into emigui_glium care

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

    let mut running = true;

    // used to keep track of time for animations
    let start_time = Instant::now();

    let mut frame_start = Instant::now();

    let mut frame_times = std::collections::VecDeque::new();

    let mut example_app = ExampleApp::default();

    let mut clipboard: Option<ClipboardContext> = match ClipboardContext::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            eprintln!("Failed to initialize clipboard: {}", err);
            None
        }
    };

    while running {
        {
            // Keep smooth frame rate. TODO: proper vsync
            let frame_duration = frame_start.elapsed();
            if frame_duration < Duration::from_millis(16) {
                std::thread::sleep(Duration::from_millis(16) - frame_duration);
            }
            frame_start = Instant::now();
        }

        {
            raw_input.time = start_time.elapsed().as_nanos() as f64 * 1e-9;
            raw_input.scroll_delta = vec2(0.0, 0.0);
            raw_input.dropped_files.clear();
            raw_input.hovered_files.clear();
            raw_input.events.clear();
            events_loop.poll_events(|event| {
                input_event(event, clipboard.as_mut(), &mut raw_input, &mut running)
            });
        }

        let emigui_start = Instant::now();
        emigui.begin_frame(raw_input.clone()); // TODO: avoid clone
        let mut region = emigui.background_region();
        let mut region = region.centered_column(region.available_width().min(480.0));
        region.set_align(Align::Min);
        region.add(label!("Emigui running inside of Glium").text_style(emigui::TextStyle::Heading));
        if region.add(Button::new("Quit")).clicked {
            running = false;
        }

        let mean_frame_time = if frame_times.is_empty() {
            0.0
        } else {
            frame_times.iter().sum::<f64>() / (frame_times.len() as f64)
        };
        region.add(
            label!(
                "Frame time: {:.1} ms (excludes painting)",
                1e3 * mean_frame_time
            )
            .text_style(TextStyle::Monospace),
        );

        // TODO: Make it even simpler to show a window

        Window::new("Examples")
            .default_pos(pos2(50.0, 100.0))
            .default_size(vec2(300.0, 600.0))
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

        frame_times.push_back((Instant::now() - emigui_start).as_secs_f64());
        while frame_times.len() > 30 {
            frame_times.pop_front();
        }

        painter.paint_batches(&display, paint_batches, emigui.texture());

        let cursor = match output.cursor_icon {
            CursorIcon::Default => glutin::MouseCursor::Default,
            CursorIcon::PointingHand => glutin::MouseCursor::Hand,
            CursorIcon::ResizeNwSe => glutin::MouseCursor::NwseResize,
            CursorIcon::Text => glutin::MouseCursor::Text,
        };

        if let Some(url) = output.open_url {
            if let Err(err) = webbrowser::open(&url) {
                eprintln!("Failed to open url: {}", err); // TODO show error in imgui
            }
        }

        if !output.copied_text.is_empty() {
            if let Some(clipboard) = clipboard.as_mut() {
                if let Err(err) = clipboard.set_contents(output.copied_text) {
                    eprintln!("Copy/Cut error: {}", err);
                }
            }
        }

        display.gl_window().set_cursor(cursor);
    }
}

fn input_event(
    event: glutin::Event,
    clipboard: Option<&mut ClipboardContext>,
    raw_input: &mut RawInput,
    running: &mut bool,
) {
    use glutin::WindowEvent::*;
    match event {
        glutin::Event::WindowEvent { event, .. } => match event {
            CloseRequested | Destroyed => *running = false,

            DroppedFile(path) => raw_input.dropped_files.push(path),
            HoveredFile(path) => raw_input.hovered_files.push(path),

            Resized(glutin::dpi::LogicalSize { width, height }) => {
                raw_input.screen_size = vec2(width as f32, height as f32);
            }
            MouseInput { state, .. } => {
                raw_input.mouse_down = state == glutin::ElementState::Pressed;
            }
            CursorMoved { position, .. } => {
                raw_input.mouse_pos = Some(pos2(position.x as f32, position.y as f32));
            }
            CursorLeft { .. } => {
                raw_input.mouse_pos = None;
            }
            ReceivedCharacter(ch) => {
                raw_input.events.push(Event::Text(ch.to_string()));
            }
            KeyboardInput { input, .. } => {
                if let Some(virtual_keycode) = input.virtual_keycode {
                    // TODO: If mac
                    if input.modifiers.logo && virtual_keycode == VirtualKeyCode::Q {
                        *running = false;
                    }

                    match virtual_keycode {
                        VirtualKeyCode::Paste => {
                            if let Some(clipboard) = clipboard {
                                match clipboard.get_contents() {
                                    Ok(contents) => {
                                        raw_input.events.push(Event::Text(contents));
                                    }
                                    Err(err) => {
                                        eprintln!("Paste error: {}", err);
                                    }
                                }
                            }
                        }
                        VirtualKeyCode::Copy => raw_input.events.push(Event::Copy),
                        VirtualKeyCode::Cut => raw_input.events.push(Event::Cut),
                        _ => {
                            if let Some(key) = translate_virtual_key_code(virtual_keycode) {
                                raw_input.events.push(Event::Key {
                                    key,
                                    pressed: input.state == glutin::ElementState::Pressed,
                                });
                            }
                        }
                    }
                }
            }
            MouseWheel { delta, .. } => {
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
            // TODO: HiDpiFactorChanged
            _ => {
                // dbg!(event);
            }
        },
        _ => (),
    }
}

fn translate_virtual_key_code(key: glutin::VirtualKeyCode) -> Option<emigui::Key> {
    use VirtualKeyCode::*;

    Some(match key {
        Escape => Key::Escape,
        Insert => Key::Insert,
        Home => Key::Home,
        Delete => Key::Delete,
        End => Key::End,
        PageDown => Key::PageDown,
        PageUp => Key::PageUp,
        Left => Key::Left,
        Up => Key::Up,
        Right => Key::Right,
        Down => Key::Down,
        Back => Key::Backspace,
        Return => Key::Return,
        // Space => Key::Space,
        Tab => Key::Tab,

        LAlt | RAlt => Key::Alt,
        LShift | RShift => Key::Shift,
        LControl | RControl => Key::Control,
        LWin | RWin => Key::Logo,

        _ => {
            return None;
        }
    })
}
