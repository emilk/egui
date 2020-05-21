#![deny(warnings)]
#![warn(clippy::all)]
#![allow(clippy::single_match)]
mod painter;

pub use painter::Painter;

use {
    clipboard::{ClipboardContext, ClipboardProvider},
    emigui::*,
    glium::glutin::{self, VirtualKeyCode},
};

pub fn init_clipboard() -> Option<ClipboardContext> {
    match ClipboardContext::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            eprintln!("Failed to initialize clipboard: {}", err);
            None
        }
    }
}

pub fn input_event(
    event: glutin::Event,
    clipboard: Option<&mut ClipboardContext>,
    raw_input: &mut RawInput,
    running: &mut bool,
) {
    use glutin::WindowEvent::*;
    match event {
        glutin::Event::WindowEvent { event, .. } => match event {
            CloseRequested | Destroyed => *running = false,

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
                if !should_ignore_char(ch) {
                    if ch == '\r' {
                        raw_input.events.push(Event::Text("\n".to_owned()));
                    } else {
                        raw_input.events.push(Event::Text(ch.to_string()));
                    }
                }
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

fn should_ignore_char(chr: char) -> bool {
    // Glium sends some keys as chars:
    match chr {
        '\u{7f}' |    // backspace
        '\u{f728}' |  // delete
        '\u{f700}' |  // up
        '\u{f701}' |  // down
        '\u{f702}' |  // left
        '\u{f703}' |  // right
        '\u{f729}' |  // home
        '\u{f72b}' |  // end
        '\u{f72c}' |  // page up
        '\u{f72d}' |  // page down
        '\u{f710}' |  // print screen
        '\u{f704}' | '\u{f705}'  // F1, F2, ...
        => true,
        _ => false,
    }
}

pub fn translate_virtual_key_code(key: glutin::VirtualKeyCode) -> Option<emigui::Key> {
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

pub fn translate_cursor(cursor_icon: emigui::CursorIcon) -> glutin::MouseCursor {
    match cursor_icon {
        CursorIcon::Default => glutin::MouseCursor::Default,
        CursorIcon::PointingHand => glutin::MouseCursor::Hand,
        CursorIcon::ResizeHorizontal => glutin::MouseCursor::EwResize,
        CursorIcon::ResizeNeSw => glutin::MouseCursor::NeswResize,
        CursorIcon::ResizeNwSe => glutin::MouseCursor::NwseResize,
        CursorIcon::ResizeVertical => glutin::MouseCursor::NsResize,
        CursorIcon::Text => glutin::MouseCursor::Text,
    }
}

pub fn handle_output(
    output: emigui::Output,
    display: &glium::backend::glutin::Display,
    clipboard: Option<&mut ClipboardContext>,
) {
    if let Some(url) = output.open_url {
        if let Err(err) = webbrowser::open(&url) {
            eprintln!("Failed to open url: {}", err); // TODO show error in imgui
        }
    }

    if !output.copied_text.is_empty() {
        if let Some(clipboard) = clipboard {
            if let Err(err) = clipboard.set_contents(output.copied_text) {
                eprintln!("Copy/Cut error: {}", err);
            }
        }
    }

    display
        .gl_window()
        .set_cursor(translate_cursor(output.cursor_icon));
}

// ----------------------------------------------------------------------------

pub fn read_memory(ctx: &Context, memory_json_path: impl AsRef<std::path::Path>) {
    match std::fs::File::open(memory_json_path) {
        Ok(file) => {
            let reader = std::io::BufReader::new(file);
            match serde_json::from_reader(reader) {
                Ok(memory) => {
                    *ctx.memory() = memory;
                }
                Err(err) => {
                    eprintln!("ERROR: Failed to parse memory json: {}", err);
                }
            }
        }
        Err(_err) => {
            // File probably doesn't exist. That's fine.
        }
    }
}

pub fn write_memory(
    ctx: &Context,
    memory_json_path: impl AsRef<std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    serde_json::to_writer_pretty(std::fs::File::create(memory_json_path)?, &*ctx.memory())?;
    Ok(())
}

// ----------------------------------------------------------------------------

/// Time of day as seconds since midnight. Used for clock in example app.
pub fn local_time_of_day() -> f64 {
    use chrono::Timelike;
    let time = chrono::Local::now().time();
    time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64)
}
