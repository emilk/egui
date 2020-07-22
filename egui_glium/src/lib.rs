#![deny(warnings)]
#![warn(clippy::all)]
#![allow(clippy::single_match)]
#![allow(deprecated)] // TODO: remove
mod painter;

pub use painter::Painter;

use glutin::event_loop::ControlFlow;

pub fn input_to_egui(
    event: glutin::event::WindowEvent,
    clipboard: Option<&mut ClipboardContext>,
    raw_input: &mut RawInput,
    control_flow: &mut ControlFlow,
) {
    use glutin::event::WindowEvent::*;
    match event {
        CloseRequested | Destroyed => *control_flow = ControlFlow::Exit,

        Resized(physical_size) => {
            raw_input.screen_size =
                egui::vec2(physical_size.width as f32, physical_size.height as f32)
                    / raw_input.pixels_per_point.unwrap();
        }

        ScaleFactorChanged {
            scale_factor,
            new_inner_size,
        } => {
            raw_input.pixels_per_point = Some(scale_factor as f32);
            raw_input.screen_size =
                egui::vec2(new_inner_size.width as f32, new_inner_size.height as f32)
                    / (scale_factor as f32);
        }

        MouseInput { state, .. } => {
            raw_input.mouse_down = state == glutin::event::ElementState::Pressed;
        }
        CursorMoved { position, .. } => {
            raw_input.mouse_pos = Some(pos2(
                position.x as f32 / raw_input.pixels_per_point.unwrap(),
                position.y as f32 / raw_input.pixels_per_point.unwrap(),
            ));
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
                if input.modifiers.logo() && virtual_keycode == VirtualKeyCode::Q {
                    *control_flow = ControlFlow::Exit;
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
                                pressed: input.state == glutin::event::ElementState::Pressed,
                            });
                        }
                    }
                }
            }
        }
        MouseWheel { delta, .. } => {
            match delta {
                glutin::event::MouseScrollDelta::LineDelta(x, y) => {
                    let line_height = 24.0; // TODO
                    raw_input.scroll_delta = vec2(x, y) * line_height;
                }
                glutin::event::MouseScrollDelta::PixelDelta(delta) => {
                    // Actually point delta
                    raw_input.scroll_delta = vec2(delta.x as f32, delta.y as f32);
                }
            }
        }
        _ => {
            // dbg!(event);
        }
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

pub fn translate_virtual_key_code(key: VirtualKeyCode) -> Option<egui::Key> {
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

pub fn translate_cursor(cursor_icon: egui::CursorIcon) -> glutin::window::CursorIcon {
    match cursor_icon {
        CursorIcon::Default => glutin::window::CursorIcon::Default,
        CursorIcon::PointingHand => glutin::window::CursorIcon::Hand,
        CursorIcon::ResizeHorizontal => glutin::window::CursorIcon::EwResize,
        CursorIcon::ResizeNeSw => glutin::window::CursorIcon::NeswResize,
        CursorIcon::ResizeNwSe => glutin::window::CursorIcon::NwseResize,
        CursorIcon::ResizeVertical => glutin::window::CursorIcon::NsResize,
        CursorIcon::Text => glutin::window::CursorIcon::Text,
    }
}

pub fn handle_output(
    output: egui::Output,
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
        .window()
        .set_cursor_icon(translate_cursor(output.cursor_icon));
}

use {
    clipboard::{ClipboardContext, ClipboardProvider},
    egui::*,
    glium::glutin::{self, event::VirtualKeyCode},
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

// ----------------------------------------------------------------------------

pub fn read_json<T>(memory_json_path: impl AsRef<std::path::Path>) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    match std::fs::File::open(memory_json_path) {
        Ok(file) => {
            let reader = std::io::BufReader::new(file);
            match serde_json::from_reader(reader) {
                Ok(value) => Some(value),
                Err(err) => {
                    eprintln!("ERROR: Failed to parse json: {}", err);
                    None
                }
            }
        }
        Err(_err) => {
            // File probably doesn't exist. That's fine.
            None
        }
    }
}

pub fn read_memory(ctx: &Context, memory_json_path: impl AsRef<std::path::Path>) {
    let memory: Option<Memory> = read_json(memory_json_path);
    if let Some(memory) = memory {
        *ctx.memory() = memory;
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

// ----------------------------------------------------------------------------

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct WindowSettings {
    pos: Option<Pos2>,
    size: Option<Vec2>,
}

impl WindowSettings {
    pub fn from_json_file(
        settings_json_path: impl AsRef<std::path::Path>,
    ) -> Option<WindowSettings> {
        read_json(settings_json_path)
    }

    pub fn from_display(display: &glium::Display) -> Self {
        Self {
            pos: display
                .gl_window()
                .window()
                .outer_position()
                .ok()
                .map(|p| pos2(p.x as f32, p.y as f32)),

            size: Some(vec2(
                display.gl_window().window().inner_size().width as f32,
                display.gl_window().window().inner_size().height as f32,
            )),
        }
    }

    pub fn initialize_size(
        &self,
        window: glutin::window::WindowBuilder,
    ) -> glutin::window::WindowBuilder {
        if let Some(size) = self.size {
            window.with_inner_size(glutin::dpi::PhysicalSize {
                width: size.x as f64,
                height: size.y as f64,
            })
        } else {
            window
        }

        // Not yet available in winit: https://github.com/rust-windowing/winit/issues/1190
        // if let Some(pos) = self.pos {
        //     *window = window.with_outer_pos(glutin::dpi::PhysicalPosition {
        //         x: pos.x as f64,
        //         y: pos.y as f64,
        //     });
        // }
    }

    pub fn restore_positions(&self, display: &glium::Display) {
        // not needed, done by `initialize_size`
        // let size = self.size.unwrap_or_else(|| vec2(1024.0, 800.0));
        // display
        //     .gl_window()
        //     .window()
        //     .set_inner_size(glutin::dpi::PhysicalSize {
        //         width: size.x as f64,
        //         height: size.y as f64,
        //     });

        if let Some(pos) = self.pos {
            display
                .gl_window()
                .window()
                .set_outer_position(glutin::dpi::PhysicalPosition::new(
                    pos.x as f64,
                    pos.y as f64,
                ));
        }
    }
}

pub fn make_raw_input(display: &glium::Display) -> egui::RawInput {
    let pixels_per_point = display.gl_window().window().scale_factor() as f32;
    egui::RawInput {
        screen_size: {
            let (width, height) = display.get_framebuffer_dimensions();
            vec2(width as f32, height as f32) / pixels_per_point
        },
        pixels_per_point: Some(pixels_per_point),
        ..Default::default()
    }
}
