//! [`egui`] bindings for [`winit`](https://github.com/rust-windowing/winit).
//!
//! The library translates winit events to egui, handled copy/paste,
//! updates the cursor, open links clicked in egui, etc.

#![forbid(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::if_let_mutex,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_errors_doc,
    clippy::missing_safety_doc,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values,
    future_incompatible,
    missing_crate_level_docs,
    nonstandard_style,
    rust_2018_idioms
)]
#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

pub use winit;

pub mod screen_reader;

pub fn native_pixels_per_point(window: &winit::window::Window) -> f32 {
    window.scale_factor() as f32
}

pub fn screen_size_in_pixels(window: &winit::window::Window) -> egui::Vec2 {
    // let (width_in_pixels, height_in_pixels) = display.get_framebuffer_dimensions();
    // egui::vec2(width_in_pixels as f32, height_in_pixels as f32)
    let size = window.inner_size().to_logical(window.scale_factor());
    egui::vec2(size.width, size.height)
}

/// Handles the integration between egui and winit.
pub struct State {
    start_time: std::time::Instant,
    egui_input: egui::RawInput,
    pointer_pos_in_points: Option<egui::Pos2>,
    any_pointer_button_down: bool,
    current_cursor_icon: egui::CursorIcon,
    /// What egui uses.
    current_pixels_per_point: f32,

    screen_reader: crate::screen_reader::ScreenReader,

    #[cfg(feature = "copypasta")]
    clipboard: Option<copypasta::ClipboardContext>,
}

impl State {
    /// Initialize with the native pixels_per_point (dpi scaling).
    pub fn new(window: &winit::window::Window) -> Self {
        Self::from_pixels_per_point(crate::native_pixels_per_point(window))
    }

    /// Initialize with a given dpi scaling.
    pub fn from_pixels_per_point(pixels_per_point: f32) -> Self {
        Self {
            start_time: std::time::Instant::now(),
            egui_input: egui::RawInput {
                pixels_per_point: Some(pixels_per_point),
                ..Default::default()
            },
            pointer_pos_in_points: None,
            any_pointer_button_down: false,
            current_cursor_icon: egui::CursorIcon::Default,
            current_pixels_per_point: pixels_per_point,

            screen_reader: crate::screen_reader::ScreenReader::default(),

            #[cfg(feature = "copypasta")]
            clipboard: init_clipboard(),
        }
    }

    /// The same as what egui uses.
    pub fn pixels_per_point(&self) -> f32 {
        self.current_pixels_per_point
    }

    /// The current input state.
    /// This is changed by [`Self::on_event`] and cleared by [`Self::take_egui_input`].
    pub fn egui_input(&self) -> &egui::RawInput {
        &self.egui_input
    }

    /// Prepare for a new frame by extracting the accumulated input.
    pub fn take_egui_input(&mut self, display: &winit::window::Window) -> egui::RawInput {
        let pixels_per_point = self.pixels_per_point();

        self.egui_input.time = Some(self.start_time.elapsed().as_secs_f64());

        // On Windows, a minimized window will have 0 width and height.
        // See: https://github.com/rust-windowing/winit/issues/208
        // This solves an issue where egui window positions would be changed when minimizing on Windows.
        let screen_size = screen_size_in_pixels(display);
        self.egui_input.screen_rect = if screen_size.x > 0.0 && screen_size.y > 0.0 {
            Some(egui::Rect::from_min_size(
                Default::default(),
                screen_size / pixels_per_point,
            ))
        } else {
            None
        };

        self.egui_input.take()
    }

    /// Call this when there is a new event.
    ///
    /// The result can be found in [`Self::egui_input`] and be extracted with [`Self::take_egui_input`].
    pub fn on_event(&mut self, event: &winit::event::WindowEvent<'_>) {
        // Useful for debugging egui touch support on non-touch devices.
        let simulate_touches = false;

        use winit::event::WindowEvent;
        match event {
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let pixels_per_point = *scale_factor as f32;
                self.egui_input.pixels_per_point = Some(pixels_per_point);
                self.current_pixels_per_point = pixels_per_point;
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(pos) = self.pointer_pos_in_points {
                    if let Some(button) = translate_mouse_button(*button) {
                        let pressed = *state == winit::event::ElementState::Pressed;

                        self.egui_input.events.push(egui::Event::PointerButton {
                            pos,
                            button,
                            pressed,
                            modifiers: self.egui_input.modifiers,
                        });

                        if simulate_touches {
                            if pressed {
                                self.any_pointer_button_down = true;

                                self.egui_input.events.push(egui::Event::Touch {
                                    device_id: egui::TouchDeviceId(0),
                                    id: egui::TouchId(0),
                                    phase: egui::TouchPhase::Start,
                                    pos,
                                    force: 0.0,
                                });
                            } else {
                                self.any_pointer_button_down = false;

                                self.egui_input.events.push(egui::Event::PointerGone);

                                self.egui_input.events.push(egui::Event::Touch {
                                    device_id: egui::TouchDeviceId(0),
                                    id: egui::TouchId(0),
                                    phase: egui::TouchPhase::End,
                                    pos,
                                    force: 0.0,
                                });
                            };
                        }
                    }
                }
            }
            WindowEvent::CursorMoved {
                position: pos_in_pixels,
                ..
            } => {
                let pos_in_points = egui::pos2(
                    pos_in_pixels.x as f32 / self.pixels_per_point(),
                    pos_in_pixels.y as f32 / self.pixels_per_point(),
                );
                self.pointer_pos_in_points = Some(pos_in_points);

                if simulate_touches {
                    if self.any_pointer_button_down {
                        self.egui_input
                            .events
                            .push(egui::Event::PointerMoved(pos_in_points));

                        self.egui_input.events.push(egui::Event::Touch {
                            device_id: egui::TouchDeviceId(0),
                            id: egui::TouchId(0),
                            phase: egui::TouchPhase::Move,
                            pos: pos_in_points,
                            force: 0.0,
                        });
                    }
                } else {
                    self.egui_input
                        .events
                        .push(egui::Event::PointerMoved(pos_in_points));
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer_pos_in_points = None;
                self.egui_input.events.push(egui::Event::PointerGone);
            }
            WindowEvent::ReceivedCharacter(ch) => {
                if is_printable_char(*ch)
                    && !self.egui_input.modifiers.ctrl
                    && !self.egui_input.modifiers.mac_cmd
                {
                    self.egui_input
                        .events
                        .push(egui::Event::Text(ch.to_string()));
                }
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(keycode) = input.virtual_keycode {
                    use winit::event::VirtualKeyCode;

                    let pressed = input.state == winit::event::ElementState::Pressed;

                    // We could also use `WindowEvent::ModifiersChanged` instead, I guess.
                    if matches!(keycode, VirtualKeyCode::LAlt | VirtualKeyCode::RAlt) {
                        self.egui_input.modifiers.alt = pressed;
                    }
                    if matches!(keycode, VirtualKeyCode::LControl | VirtualKeyCode::RControl) {
                        self.egui_input.modifiers.ctrl = pressed;
                        if !cfg!(target_os = "macos") {
                            self.egui_input.modifiers.command = pressed;
                        }
                    }
                    if matches!(keycode, VirtualKeyCode::LShift | VirtualKeyCode::RShift) {
                        self.egui_input.modifiers.shift = pressed;
                    }
                    if cfg!(target_os = "macos")
                        && matches!(keycode, VirtualKeyCode::LWin | VirtualKeyCode::RWin)
                    {
                        self.egui_input.modifiers.mac_cmd = pressed;
                        self.egui_input.modifiers.command = pressed;
                    }

                    if pressed {
                        // VirtualKeyCode::Paste etc in winit are broken/untrustworthy,
                        // so we detect these things manually:
                        if is_cut_command(self.egui_input.modifiers, keycode) {
                            self.egui_input.events.push(egui::Event::Cut);
                        } else if is_copy_command(self.egui_input.modifiers, keycode) {
                            self.egui_input.events.push(egui::Event::Copy);
                        } else if is_paste_command(self.egui_input.modifiers, keycode) {
                            #[cfg(feature = "copypasta")]
                            if let Some(clipboard) = &mut self.clipboard {
                                use copypasta::ClipboardProvider as _;
                                match clipboard.get_contents() {
                                    Ok(contents) => {
                                        self.egui_input.events.push(egui::Event::Text(contents));
                                    }
                                    Err(err) => {
                                        eprintln!("Paste error: {}", err);
                                    }
                                }
                            }
                        }
                    }

                    if let Some(key) = translate_virtual_key_code(keycode) {
                        self.egui_input.events.push(egui::Event::Key {
                            key,
                            pressed,
                            modifiers: self.egui_input.modifiers,
                        });
                    }
                }
            }
            WindowEvent::Focused(_) => {
                // We will not be given a KeyboardInput event when the modifiers are released while
                // the window does not have focus. Unset all modifier state to be safe.
                self.egui_input.modifiers = egui::Modifiers::default();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let mut delta = match *delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        let points_per_scroll_line = 50.0; // Scroll speed decided by consensus: https://github.com/emilk/egui/issues/461
                        egui::vec2(x, y) * points_per_scroll_line
                    }
                    winit::event::MouseScrollDelta::PixelDelta(delta) => {
                        egui::vec2(delta.x as f32, delta.y as f32) / self.pixels_per_point()
                    }
                };
                if cfg!(target_os = "macos") {
                    // This is still buggy in winit despite
                    // https://github.com/rust-windowing/winit/issues/1695 being closed
                    delta.x *= -1.0;
                }

                if self.egui_input.modifiers.ctrl || self.egui_input.modifiers.command {
                    // Treat as zoom instead:
                    self.egui_input.zoom_delta *= (delta.y / 200.0).exp();
                } else {
                    self.egui_input.scroll_delta += delta;
                }
            }
            // WindowEvent::TouchpadPressure {
            //     device_id,
            //     pressure,
            //     stage,
            //     ..
            // } => {
            //     // TODO
            // }
            WindowEvent::Touch(touch) => {
                use std::hash::{Hash as _, Hasher as _};
                let pixels_per_point_recip = 1. / self.pixels_per_point();
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                touch.device_id.hash(&mut hasher);
                self.egui_input.events.push(egui::Event::Touch {
                    device_id: egui::TouchDeviceId(hasher.finish()),
                    id: egui::TouchId::from(touch.id),
                    phase: match touch.phase {
                        winit::event::TouchPhase::Started => egui::TouchPhase::Start,
                        winit::event::TouchPhase::Moved => egui::TouchPhase::Move,
                        winit::event::TouchPhase::Ended => egui::TouchPhase::End,
                        winit::event::TouchPhase::Cancelled => egui::TouchPhase::Cancel,
                    },
                    pos: egui::pos2(
                        touch.location.x as f32 * pixels_per_point_recip,
                        touch.location.y as f32 * pixels_per_point_recip,
                    ),
                    force: match touch.force {
                        Some(winit::event::Force::Normalized(force)) => force as f32,
                        Some(winit::event::Force::Calibrated {
                            force,
                            max_possible_force,
                            ..
                        }) => (force / max_possible_force) as f32,
                        None => 0_f32,
                    },
                });
            }
            WindowEvent::HoveredFile(path) => {
                self.egui_input.hovered_files.push(egui::HoveredFile {
                    path: Some(path.clone()),
                    ..Default::default()
                });
            }
            WindowEvent::HoveredFileCancelled => {
                self.egui_input.hovered_files.clear();
            }
            WindowEvent::DroppedFile(path) => {
                self.egui_input.hovered_files.clear();
                self.egui_input.dropped_files.push(egui::DroppedFile {
                    path: Some(path.clone()),
                    ..Default::default()
                });
            }
            _ => {
                // dbg!(event);
            }
        }
    }

    /// Call with the output given by `egui`.
    ///
    /// This will update the cursor, copy text to the clipboard, etc.
    pub fn handle_output(
        &mut self,
        window: &winit::window::Window,
        egui_ctx: &egui::Context,
        output: egui::Output,
    ) {
        self.current_pixels_per_point = egui_ctx.pixels_per_point(); // someone can have changed it to scale the UI

        if egui_ctx.memory().options.screen_reader {
            self.screen_reader.speak(&output.events_description());
        }

        self.set_cursor_icon(window, output.cursor_icon);

        if let Some(open) = output.open_url {
            if let Err(err) = webbrowser::open(&open.url) {
                eprintln!("Failed to open url: {}", err);
            }
        }

        #[cfg(feature = "copypasta")]
        if !output.copied_text.is_empty() {
            if let Some(clipboard) = &mut self.clipboard {
                use copypasta::ClipboardProvider as _;
                if let Err(err) = clipboard.set_contents(output.copied_text) {
                    eprintln!("Copy/Cut error: {}", err);
                }
            }
        }

        if let Some(egui::Pos2 { x, y }) = output.text_cursor_pos {
            window.set_ime_position(winit::dpi::LogicalPosition { x, y })
        }
    }

    /// Helper: checks for Alt-F4 (windows/linux) or Cmd-Q (Mac)
    pub fn is_quit_shortcut(&self, input: &winit::event::KeyboardInput) -> bool {
        if cfg!(target_os = "macos") {
            input.state == winit::event::ElementState::Pressed
                && self.egui_input.modifiers.mac_cmd
                && input.virtual_keycode == Some(winit::event::VirtualKeyCode::Q)
        } else {
            input.state == winit::event::ElementState::Pressed
                && self.egui_input.modifiers.alt
                && input.virtual_keycode == Some(winit::event::VirtualKeyCode::F4)
        }
    }

    /// Is this a close event or a Cmd-Q/Alt-F4 keyboard command?
    pub fn is_quit_event(&self, event: &winit::event::WindowEvent<'_>) -> bool {
        use winit::event::WindowEvent;
        match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => true,
            WindowEvent::KeyboardInput { input, .. } => self.is_quit_shortcut(input),
            _ => false,
        }
    }

    fn set_cursor_icon(&mut self, window: &winit::window::Window, cursor_icon: egui::CursorIcon) {
        // prevent flickering near frame boundary when Windows OS tries to control cursor icon for window resizing
        if self.current_cursor_icon == cursor_icon {
            return;
        }
        self.current_cursor_icon = cursor_icon;

        if let Some(cursor_icon) = translate_cursor(cursor_icon) {
            window.set_cursor_visible(true);

            let is_pointer_in_window = self.pointer_pos_in_points.is_some();
            if is_pointer_in_window {
                window.set_cursor_icon(cursor_icon);
            }
        } else {
            window.set_cursor_visible(false);
        }
    }
}

#[cfg(feature = "copypasta")]
fn init_clipboard() -> Option<copypasta::ClipboardContext> {
    match copypasta::ClipboardContext::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            eprintln!("Failed to initialize clipboard: {}", err);
            None
        }
    }
}

/// Glium sends special keys (backspace, delete, F1, ...) as characters.
/// Ignore those.
/// We also ignore '\r', '\n', '\t'.
/// Newlines are handled by the `Key::Enter` event.
fn is_printable_char(chr: char) -> bool {
    let is_in_private_use_area = '\u{e000}' <= chr && chr <= '\u{f8ff}'
        || '\u{f0000}' <= chr && chr <= '\u{ffffd}'
        || '\u{100000}' <= chr && chr <= '\u{10fffd}';

    !is_in_private_use_area && !chr.is_ascii_control()
}

fn is_cut_command(modifiers: egui::Modifiers, keycode: winit::event::VirtualKeyCode) -> bool {
    (modifiers.command && keycode == winit::event::VirtualKeyCode::X)
        || (cfg!(target_os = "windows")
            && modifiers.shift
            && keycode == winit::event::VirtualKeyCode::Delete)
}

fn is_copy_command(modifiers: egui::Modifiers, keycode: winit::event::VirtualKeyCode) -> bool {
    (modifiers.command && keycode == winit::event::VirtualKeyCode::C)
        || (cfg!(target_os = "windows")
            && modifiers.ctrl
            && keycode == winit::event::VirtualKeyCode::Insert)
}

fn is_paste_command(modifiers: egui::Modifiers, keycode: winit::event::VirtualKeyCode) -> bool {
    (modifiers.command && keycode == winit::event::VirtualKeyCode::V)
        || (cfg!(target_os = "windows")
            && modifiers.shift
            && keycode == winit::event::VirtualKeyCode::Insert)
}

fn translate_mouse_button(button: winit::event::MouseButton) -> Option<egui::PointerButton> {
    match button {
        winit::event::MouseButton::Left => Some(egui::PointerButton::Primary),
        winit::event::MouseButton::Right => Some(egui::PointerButton::Secondary),
        winit::event::MouseButton::Middle => Some(egui::PointerButton::Middle),
        winit::event::MouseButton::Other(_) => None,
    }
}

fn translate_virtual_key_code(key: winit::event::VirtualKeyCode) -> Option<egui::Key> {
    use egui::Key;
    use winit::event::VirtualKeyCode;

    Some(match key {
        VirtualKeyCode::Down => Key::ArrowDown,
        VirtualKeyCode::Left => Key::ArrowLeft,
        VirtualKeyCode::Right => Key::ArrowRight,
        VirtualKeyCode::Up => Key::ArrowUp,

        VirtualKeyCode::Escape => Key::Escape,
        VirtualKeyCode::Tab => Key::Tab,
        VirtualKeyCode::Back => Key::Backspace,
        VirtualKeyCode::Return => Key::Enter,
        VirtualKeyCode::Space => Key::Space,

        VirtualKeyCode::Insert => Key::Insert,
        VirtualKeyCode::Delete => Key::Delete,
        VirtualKeyCode::Home => Key::Home,
        VirtualKeyCode::End => Key::End,
        VirtualKeyCode::PageUp => Key::PageUp,
        VirtualKeyCode::PageDown => Key::PageDown,

        VirtualKeyCode::Key0 | VirtualKeyCode::Numpad0 => Key::Num0,
        VirtualKeyCode::Key1 | VirtualKeyCode::Numpad1 => Key::Num1,
        VirtualKeyCode::Key2 | VirtualKeyCode::Numpad2 => Key::Num2,
        VirtualKeyCode::Key3 | VirtualKeyCode::Numpad3 => Key::Num3,
        VirtualKeyCode::Key4 | VirtualKeyCode::Numpad4 => Key::Num4,
        VirtualKeyCode::Key5 | VirtualKeyCode::Numpad5 => Key::Num5,
        VirtualKeyCode::Key6 | VirtualKeyCode::Numpad6 => Key::Num6,
        VirtualKeyCode::Key7 | VirtualKeyCode::Numpad7 => Key::Num7,
        VirtualKeyCode::Key8 | VirtualKeyCode::Numpad8 => Key::Num8,
        VirtualKeyCode::Key9 | VirtualKeyCode::Numpad9 => Key::Num9,

        VirtualKeyCode::A => Key::A,
        VirtualKeyCode::B => Key::B,
        VirtualKeyCode::C => Key::C,
        VirtualKeyCode::D => Key::D,
        VirtualKeyCode::E => Key::E,
        VirtualKeyCode::F => Key::F,
        VirtualKeyCode::G => Key::G,
        VirtualKeyCode::H => Key::H,
        VirtualKeyCode::I => Key::I,
        VirtualKeyCode::J => Key::J,
        VirtualKeyCode::K => Key::K,
        VirtualKeyCode::L => Key::L,
        VirtualKeyCode::M => Key::M,
        VirtualKeyCode::N => Key::N,
        VirtualKeyCode::O => Key::O,
        VirtualKeyCode::P => Key::P,
        VirtualKeyCode::Q => Key::Q,
        VirtualKeyCode::R => Key::R,
        VirtualKeyCode::S => Key::S,
        VirtualKeyCode::T => Key::T,
        VirtualKeyCode::U => Key::U,
        VirtualKeyCode::V => Key::V,
        VirtualKeyCode::W => Key::W,
        VirtualKeyCode::X => Key::X,
        VirtualKeyCode::Y => Key::Y,
        VirtualKeyCode::Z => Key::Z,

        _ => {
            return None;
        }
    })
}

fn translate_cursor(cursor_icon: egui::CursorIcon) -> Option<winit::window::CursorIcon> {
    match cursor_icon {
        egui::CursorIcon::None => None,

        egui::CursorIcon::Alias => Some(winit::window::CursorIcon::Alias),
        egui::CursorIcon::AllScroll => Some(winit::window::CursorIcon::AllScroll),
        egui::CursorIcon::Cell => Some(winit::window::CursorIcon::Cell),
        egui::CursorIcon::ContextMenu => Some(winit::window::CursorIcon::ContextMenu),
        egui::CursorIcon::Copy => Some(winit::window::CursorIcon::Copy),
        egui::CursorIcon::Crosshair => Some(winit::window::CursorIcon::Crosshair),
        egui::CursorIcon::Default => Some(winit::window::CursorIcon::Default),
        egui::CursorIcon::Grab => Some(winit::window::CursorIcon::Grab),
        egui::CursorIcon::Grabbing => Some(winit::window::CursorIcon::Grabbing),
        egui::CursorIcon::Help => Some(winit::window::CursorIcon::Help),
        egui::CursorIcon::Move => Some(winit::window::CursorIcon::Move),
        egui::CursorIcon::NoDrop => Some(winit::window::CursorIcon::NoDrop),
        egui::CursorIcon::NotAllowed => Some(winit::window::CursorIcon::NotAllowed),
        egui::CursorIcon::PointingHand => Some(winit::window::CursorIcon::Hand),
        egui::CursorIcon::Progress => Some(winit::window::CursorIcon::Progress),
        egui::CursorIcon::ResizeHorizontal => Some(winit::window::CursorIcon::EwResize),
        egui::CursorIcon::ResizeNeSw => Some(winit::window::CursorIcon::NeswResize),
        egui::CursorIcon::ResizeNwSe => Some(winit::window::CursorIcon::NwseResize),
        egui::CursorIcon::ResizeVertical => Some(winit::window::CursorIcon::NsResize),
        egui::CursorIcon::Text => Some(winit::window::CursorIcon::Text),
        egui::CursorIcon::VerticalText => Some(winit::window::CursorIcon::VerticalText),
        egui::CursorIcon::Wait => Some(winit::window::CursorIcon::Wait),
        egui::CursorIcon::ZoomIn => Some(winit::window::CursorIcon::ZoomIn),
        egui::CursorIcon::ZoomOut => Some(winit::window::CursorIcon::ZoomOut),
    }
}
