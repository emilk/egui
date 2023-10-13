use std::time::Instant;

use egui::{Modifiers, Pos2};
use sdl2::mouse::SystemCursor;

use crate::DpiScaling;

use super::conversions::{ToEguiKey, ToEguiModifiers, ToEguiPointerButton, ToSdl2SystemCursor};

pub struct FusedCursor {
    pub cursor: sdl2::mouse::Cursor,
    pub icon: sdl2::mouse::SystemCursor,
}

impl FusedCursor {
    pub fn new() -> Self {
        Self {
            cursor: sdl2::mouse::Cursor::from_system(SystemCursor::Arrow).unwrap(),
            icon: SystemCursor::Arrow,
        }
    }
}

impl Default for FusedCursor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Platform {
    start_time: Instant,
    pointer_pos: Option<Pos2>,
    fused_cursor: FusedCursor,
    modifiers: Modifiers,
    raw_input: egui::RawInput,

    current_pixels_per_point: f32,

    egui_ctx: egui::Context,
}

impl Platform {
    pub fn new(window: &sdl2::video::Window, scale: DpiScaling) -> Self {
        //log::info!("window size: {size:?}, drawable: {drawable_size:?}");
        let ratio = Self::get_dpi(window);
        let scale = match scale {
            DpiScaling::Default => ratio,
            DpiScaling::Custom(custom) => ratio * custom,
        };

        Self {
            start_time: Instant::now(),
            pointer_pos: None,
            raw_input: egui::RawInput {
                screen_rect: None,
                pixels_per_point: Some(scale),
                ..Default::default()
            },
            current_pixels_per_point: scale,
            fused_cursor: FusedCursor::default(),
            modifiers: Modifiers::default(),
            egui_ctx: egui::Context::default(),
        }
    }

    pub fn screen_size_in_pixels(window: &sdl2::video::Window) -> egui::Vec2 {
        let size = window.drawable_size();
        egui::vec2(size.0 as f32, size.1 as f32)
    }

    fn get_dpi(window: &sdl2::video::Window) -> f32 {
        window.drawable_size().0 as f32 / window.size().0 as f32
    }

    pub fn handle_event(&mut self, event: &sdl2::event::Event, window: &sdl2::video::Window) {
        //log::trace!("Event {event:?}");
        use sdl2::event::{Event, WindowEvent};
        match event {
            Event::Window {
                win_event: WindowEvent::Leave,
                ..
            } => {
                self.pointer_pos = None;
                self.raw_input.events.push(egui::Event::PointerGone);
            }

            Event::MouseButtonDown { mouse_btn, .. } => {
                if let Some(pos) = self.pointer_pos {
                    if let Some(btn) = mouse_btn.to_egui_pointer_button() {
                        self.raw_input.events.push(egui::Event::PointerButton {
                            pos,
                            button: btn,
                            pressed: true,
                            modifiers: self.modifiers,
                        });
                    }
                }
            }
            Event::MouseButtonUp { mouse_btn, .. } => {
                if let Some(pos) = self.pointer_pos {
                    if let Some(btn) = mouse_btn.to_egui_pointer_button() {
                        self.raw_input.events.push(egui::Event::PointerButton {
                            pos,
                            button: btn,
                            pressed: false,
                            modifiers: self.modifiers,
                        });
                    }
                }
            }
            Event::MouseMotion { x, y, .. } => {
                let native_ratio = Self::get_dpi(window);
                let pos = egui::Pos2::new(
                    *x as f32 / (self.current_pixels_per_point / native_ratio),
                    *y as f32 / (self.current_pixels_per_point / native_ratio),
                );

                self.pointer_pos = Some(pos);
                self.raw_input.events.push(egui::Event::PointerMoved(pos));
            }
            Event::MouseWheel { x, y, .. } => {
                let delta = egui::Vec2::new(*x as f32 * 8.0, *y as f32 * 8.0);

                self.raw_input.events.push(if self.modifiers.ctrl {
                    egui::Event::Zoom((delta.y / 125.0).exp())
                } else {
                    egui::Event::Scroll(delta)
                });
            }
            Event::KeyDown {
                keycode: Some(keycode),
                keymod,
                ..
            } => {
                if let Some(key) = keycode.to_egui_key() {
                    self.modifiers = keymod.to_egui_modifier();
                    self.raw_input.events.push(egui::Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers: self.modifiers,
                    });

                    // Handle Cut Copy and paste
                    use egui::Key;
                    let clipboard = window.subsystem().clipboard();
                    if self.modifiers.command && key == Key::C {
                        self.raw_input.events.push(egui::Event::Copy);
                    } else if self.modifiers.command && key == Key::X {
                        self.raw_input.events.push(egui::Event::Cut);
                    } else if self.modifiers.command
                        && key == Key::V
                        && clipboard.has_clipboard_text()
                    {
                        let contents = clipboard.clipboard_text().unwrap().replace("\r\n", "\n");
                        if !contents.is_empty() {
                            self.raw_input.events.push(egui::Event::Paste(contents));
                        }
                    }
                }
            }
            Event::KeyUp {
                keycode, keymod, ..
            } => {
                if let Some(keycode) = keycode {
                    if let Some(key) = keycode.to_egui_key() {
                        self.modifiers = keymod.to_egui_modifier();
                        self.raw_input.modifiers = self.modifiers;
                        self.raw_input.events.push(egui::Event::Key {
                            key,
                            pressed: false,
                            repeat: false,
                            modifiers: self.modifiers,
                        });
                    }
                }
                self.egui_ctx.wants_keyboard_input();
            }
            Event::TextInput { text, .. } => {
                self.raw_input
                    .events
                    .push(egui::Event::Text(text.replace("\r\n", "\n")));
            }

            _ => {}
        }
    }

    /// Prepare for a new frame by extracting the accumulated input,
    /// as well as setting [the time](egui::RawInput::time) and [screen rectangle](egui::RawInput::screen_rect).
    pub fn take_egui_input(&mut self, window: &sdl2::video::Window) -> egui::RawInput {
        let pixels_per_point = self.current_pixels_per_point;
        self.raw_input.time = Some(self.start_time.elapsed().as_secs_f64());

        let screen_size_in_pixels = Self::screen_size_in_pixels(window);
        let screen_size_in_points = screen_size_in_pixels / pixels_per_point;
        self.raw_input.screen_rect =
            if screen_size_in_points.x > 0.0 && screen_size_in_points.y > 0.0 {
                Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    screen_size_in_points,
                ))
            } else {
                None
            };
        self.raw_input.pixels_per_point = Some(pixels_per_point);
        self.raw_input.take()
    }

    /// Call with the output given by `egui`.
    ///
    /// This will, if needed:
    /// * update the cursor
    /// * copy text to the clipboard
    /// * open any clicked urls
    /// *
    pub fn handle_platform_output(
        &mut self,
        clipboard: &mut sdl2::clipboard::ClipboardUtil,
        egui_ctx: &egui::Context,
        platform_output: egui::PlatformOutput,
    ) {
        let egui::PlatformOutput {
            cursor_icon,
            open_url,
            copied_text,
            events: _,
            mutable_text_under_cursor: _,
            text_cursor_pos: _,
        } = platform_output;
        self.current_pixels_per_point = egui_ctx.pixels_per_point(); // someone can have changed it to scale the UI

        if let Some(system_cursor) = cursor_icon.to_sdl2_cursor() {
            if system_cursor != self.fused_cursor.icon {
                self.fused_cursor.cursor = sdl2::mouse::Cursor::from_system(system_cursor).unwrap();
                self.fused_cursor.icon = system_cursor;
                self.fused_cursor.cursor.set();
            }
        }

        if let Some(open_url) = open_url {
            Self::open_url_in_browser(&open_url.url);
        }
        if !copied_text.is_empty() && clipboard.set_clipboard_text(&copied_text).is_err() {
            log::warn!("Failed to paste clipboard text");
        }
    }

    fn open_url_in_browser(url: &str) {
        #[cfg(feature = "webbrowser")]
        if let Err(err) = webbrowser::open(url) {
            log::warn!("Failed to open url: {}", err);
        }

        #[cfg(not(feature = "webbrowser"))]
        {
            log::warn!("Cannot open url ({url}) - feature \"links\" not enabled.");
        }
    }
}
