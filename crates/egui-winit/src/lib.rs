//! [`egui`] bindings for [`winit`](https://github.com/rust-windowing/winit).
//!
//! The library translates winit events to egui, handled copy/paste,
//! updates the cursor, open links clicked in egui, etc.
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![allow(clippy::manual_range_contains)]

#[cfg(feature = "accesskit")]
pub use accesskit_winit;
pub use egui;
#[cfg(feature = "accesskit")]
use egui::accesskit;
use egui::{
    Pos2, Rect, Vec2, ViewportBuilder, ViewportCommand, ViewportId, ViewportIdPair, ViewportInfo,
};
pub use winit;

pub mod clipboard;
mod window_settings;

pub use window_settings::WindowSettings;

use raw_window_handle::HasRawDisplayHandle;

pub fn native_pixels_per_point(window: &Window) -> f32 {
    window.scale_factor() as f32
}

pub fn screen_size_in_pixels(window: &Window) -> egui::Vec2 {
    let size = window.inner_size();
    egui::vec2(size.width as f32, size.height as f32)
}

// ----------------------------------------------------------------------------

#[must_use]
pub struct EventResponse {
    /// If true, egui consumed this event, i.e. wants exclusive use of this event
    /// (e.g. a mouse click on an egui window, or entering text into a text field).
    ///
    /// For instance, if you use egui for a game, you should only
    /// pass on the events to your game when [`Self::consumed`] is `false.
    ///
    /// Note that egui uses `tab` to move focus between elements, so this will always be `true` for tabs.
    pub consumed: bool,

    /// Do we need an egui refresh because of this event?
    pub repaint: bool,
}

// ----------------------------------------------------------------------------

/// Handles the integration between egui and a winit Window.
///
/// Instantiate one of these per viewport/window.
pub struct State {
    start_time: web_time::Instant,
    egui_input: egui::RawInput,
    pointer_pos_in_points: Option<egui::Pos2>,
    any_pointer_button_down: bool,
    current_cursor_icon: Option<egui::CursorIcon>,

    /// What egui uses.
    current_pixels_per_point: f32,

    clipboard: clipboard::Clipboard,

    /// If `true`, mouse inputs will be treated as touches.
    /// Useful for debugging touch support in egui.
    ///
    /// Creates duplicate touches, if real touch inputs are coming.
    simulate_touch_screen: bool,

    /// Is Some(…) when a touch is being translated to a pointer.
    ///
    /// Only one touch will be interpreted as pointer at any time.
    pointer_touch_id: Option<u64>,

    /// track ime state
    input_method_editor_started: bool,

    #[cfg(feature = "accesskit")]
    accesskit: Option<accesskit_winit::Adapter>,

    allow_ime: bool,
}

impl State {
    /// Construct a new instance
    pub fn new(
        display_target: &dyn HasRawDisplayHandle,
        native_pixels_per_point: Option<f32>,
        max_texture_side: Option<usize>,
    ) -> Self {
        crate::profile_function!();

        let egui_input = egui::RawInput {
            focused: false, // winit will tell us when we have focus
            ..Default::default()
        };

        let mut slf = Self {
            start_time: web_time::Instant::now(),
            egui_input,
            pointer_pos_in_points: None,
            any_pointer_button_down: false,
            current_cursor_icon: None,
            current_pixels_per_point: 1.0,

            clipboard: clipboard::Clipboard::new(display_target),

            simulate_touch_screen: false,
            pointer_touch_id: None,

            input_method_editor_started: false,

            #[cfg(feature = "accesskit")]
            accesskit: None,

            allow_ime: false,
        };
        if let Some(native_pixels_per_point) = native_pixels_per_point {
            slf.set_pixels_per_point(native_pixels_per_point);
        }
        if let Some(max_texture_side) = max_texture_side {
            slf.set_max_texture_side(max_texture_side);
        }
        slf
    }

    #[cfg(feature = "accesskit")]
    pub fn init_accesskit<T: From<accesskit_winit::ActionRequestEvent> + Send>(
        &mut self,
        window: &Window,
        event_loop_proxy: winit::event_loop::EventLoopProxy<T>,
        initial_tree_update_factory: impl 'static + FnOnce() -> accesskit::TreeUpdate + Send,
    ) {
        crate::profile_function!();
        self.accesskit = Some(accesskit_winit::Adapter::new(
            window,
            initial_tree_update_factory,
            event_loop_proxy,
        ));
    }

    /// Call this once a graphics context has been created to update the maximum texture dimensions
    /// that egui will use.
    pub fn set_max_texture_side(&mut self, max_texture_side: usize) {
        self.egui_input.max_texture_side = Some(max_texture_side);
    }

    /// Call this when a new native Window is created for rendering to initialize the `pixels_per_point`
    /// for that window.
    ///
    /// In particular, on Android it is necessary to call this after each `Resumed` lifecycle
    /// event, each time a new native window is created.
    ///
    /// Once this has been initialized for a new window then this state will be maintained by handling
    /// [`winit::event::WindowEvent::ScaleFactorChanged`] events.
    pub fn set_pixels_per_point(&mut self, pixels_per_point: f32) {
        self.egui_input.pixels_per_point = Some(pixels_per_point);
        self.current_pixels_per_point = pixels_per_point;
    }

    /// The number of physical pixels per logical point,
    /// as configured on the current egui context (see [`egui::Context::pixels_per_point`]).
    #[inline]
    pub fn pixels_per_point(&self) -> f32 {
        self.current_pixels_per_point
    }

    /// The current input state.
    /// This is changed by [`Self::on_event`] and cleared by [`Self::take_egui_input`].
    #[inline]
    pub fn egui_input(&self) -> &egui::RawInput {
        &self.egui_input
    }

    /// The current input state.
    /// This is changed by [`Self::on_event`] and cleared by [`Self::take_egui_input`].
    #[inline]
    pub fn egui_input_mut(&mut self) -> &mut egui::RawInput {
        &mut self.egui_input
    }

    /// Update the given viewport info with the current state of the window.
    ///
    /// Call before [`Self::update_viewport_info`]
    pub fn update_viewport_info(&self, info: &mut ViewportInfo, window: &Window) {
        update_viewport_info(info, window, self.pixels_per_point());
    }

    /// Prepare for a new frame by extracting the accumulated input,
    ///
    /// as well as setting [the time](egui::RawInput::time) and [screen rectangle](egui::RawInput::screen_rect).
    ///
    /// You need to set [`egui::RawInput::viewports`] yourself though.
    /// Use [`Self::update_viewport_info`] to update the info for each
    /// viewport.
    pub fn take_egui_input(&mut self, window: &Window, ids: ViewportIdPair) -> egui::RawInput {
        crate::profile_function!();

        let pixels_per_point = self.pixels_per_point();

        self.egui_input.time = Some(self.start_time.elapsed().as_secs_f64());

        // TODO remove this in winit 0.29
        // This hack make the window outer_position and size to be valid, X11 Only
        // That was happending because winit get the window state before the compositor adds decorations!
        #[cfg(all(feature = "x11", target_os = "linux"))]
        window.set_maximized(window.is_maximized());

        // On Windows, a minimized window will have 0 width and height.
        // See: https://github.com/rust-windowing/winit/issues/208
        // This solves an issue where egui window positions would be changed when minimizing on Windows.
        let screen_size_in_pixels = screen_size_in_pixels(window);
        let screen_size_in_points = screen_size_in_pixels / pixels_per_point;

        self.egui_input.screen_rect = (screen_size_in_points.x > 0.0
            && screen_size_in_points.y > 0.0)
            .then(|| Rect::from_min_size(Pos2::ZERO, screen_size_in_points));

        // Tell egui which viewport is now active:
        self.egui_input.viewport_ids = ids;
        self.egui_input.native_pixels_per_point = Some(native_pixels_per_point(window));
        self.egui_input.take()
    }

    /// Call this when there is a new event.
    ///
    /// The result can be found in [`Self::egui_input`] and be extracted with [`Self::take_egui_input`].
    pub fn on_event(
        &mut self,
        egui_ctx: &egui::Context,
        event: &winit::event::WindowEvent<'_>,
        viewport_id: ViewportId,
    ) -> EventResponse {
        crate::profile_function!();

        use winit::event::WindowEvent;
        match event {
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let pixels_per_point = *scale_factor as f32;
                self.egui_input.pixels_per_point = Some(pixels_per_point);
                self.current_pixels_per_point = pixels_per_point;
                EventResponse {
                    repaint: true,
                    consumed: false,
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.on_mouse_button_input(*state, *button);
                EventResponse {
                    repaint: true,
                    consumed: egui_ctx.wants_pointer_input(),
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.on_mouse_wheel(*delta);
                EventResponse {
                    repaint: true,
                    consumed: egui_ctx.wants_pointer_input(),
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.on_cursor_moved(*position);
                EventResponse {
                    repaint: true,
                    consumed: egui_ctx.is_using_pointer(),
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer_pos_in_points = None;
                self.egui_input.events.push(egui::Event::PointerGone);
                EventResponse {
                    repaint: true,
                    consumed: false,
                }
            }
            // WindowEvent::TouchpadPressure {device_id, pressure, stage, ..  } => {} // TODO
            WindowEvent::Touch(touch) => {
                self.on_touch(touch);
                let consumed = match touch.phase {
                    winit::event::TouchPhase::Started
                    | winit::event::TouchPhase::Ended
                    | winit::event::TouchPhase::Cancelled => egui_ctx.wants_pointer_input(),
                    winit::event::TouchPhase::Moved => egui_ctx.is_using_pointer(),
                };
                EventResponse {
                    repaint: true,
                    consumed,
                }
            }
            WindowEvent::ReceivedCharacter(ch) => {
                // On Mac we get here when the user presses Cmd-C (copy), ctrl-W, etc.
                // We need to ignore these characters that are side-effects of commands.
                let is_mac_cmd = cfg!(target_os = "macos")
                    && (self.egui_input.modifiers.ctrl || self.egui_input.modifiers.mac_cmd);

                let consumed = if is_printable_char(*ch) && !is_mac_cmd {
                    self.egui_input
                        .events
                        .push(egui::Event::Text(ch.to_string()));
                    egui_ctx.wants_keyboard_input()
                } else {
                    false
                };
                EventResponse {
                    repaint: true,
                    consumed,
                }
            }
            WindowEvent::Ime(ime) => {
                // on Mac even Cmd-C is pressed during ime, a `c` is pushed to Preedit.
                // So no need to check is_mac_cmd.
                //
                // How winit produce `Ime::Enabled` and `Ime::Disabled` differs in MacOS
                // and Windows.
                //
                // - On Windows, before and after each Commit will produce an Enable/Disabled
                // event.
                // - On MacOS, only when user explicit enable/disable ime. No Disabled
                // after Commit.
                //
                // We use input_method_editor_started to manually insert CompositionStart
                // between Commits.
                match ime {
                    winit::event::Ime::Enabled | winit::event::Ime::Disabled => (),
                    winit::event::Ime::Commit(text) => {
                        self.input_method_editor_started = false;
                        self.egui_input
                            .events
                            .push(egui::Event::CompositionEnd(text.clone()));
                    }
                    winit::event::Ime::Preedit(text, Some(_)) => {
                        if !self.input_method_editor_started {
                            self.input_method_editor_started = true;
                            self.egui_input.events.push(egui::Event::CompositionStart);
                        }
                        self.egui_input
                            .events
                            .push(egui::Event::CompositionUpdate(text.clone()));
                    }
                    winit::event::Ime::Preedit(_, None) => {}
                };

                EventResponse {
                    repaint: true,
                    consumed: egui_ctx.wants_keyboard_input(),
                }
            }
            WindowEvent::KeyboardInput { input, .. } => {
                self.on_keyboard_input(input);
                // When pressing the Tab key, egui focuses the first focusable element, hence Tab always consumes.
                let consumed = egui_ctx.wants_keyboard_input()
                    || input.virtual_keycode == Some(winit::event::VirtualKeyCode::Tab);
                EventResponse {
                    repaint: true,
                    consumed,
                }
            }
            WindowEvent::Focused(focused) => {
                self.egui_input.focused = *focused;
                // We will not be given a KeyboardInput event when the modifiers are released while
                // the window does not have focus. Unset all modifier state to be safe.
                self.egui_input.modifiers = egui::Modifiers::default();
                self.egui_input
                    .events
                    .push(egui::Event::WindowFocused(*focused));
                EventResponse {
                    repaint: true,
                    consumed: false,
                }
            }
            WindowEvent::HoveredFile(path) => {
                self.egui_input.hovered_files.push(egui::HoveredFile {
                    path: Some(path.clone()),
                    ..Default::default()
                });
                EventResponse {
                    repaint: true,
                    consumed: false,
                }
            }
            WindowEvent::HoveredFileCancelled => {
                self.egui_input.hovered_files.clear();
                EventResponse {
                    repaint: true,
                    consumed: false,
                }
            }
            WindowEvent::DroppedFile(path) => {
                self.egui_input.hovered_files.clear();
                self.egui_input.dropped_files.push(egui::DroppedFile {
                    path: Some(path.clone()),
                    ..Default::default()
                });
                EventResponse {
                    repaint: true,
                    consumed: false,
                }
            }
            WindowEvent::ModifiersChanged(state) => {
                self.egui_input.modifiers.alt = state.alt();
                self.egui_input.modifiers.ctrl = state.ctrl();
                self.egui_input.modifiers.shift = state.shift();
                self.egui_input.modifiers.mac_cmd = cfg!(target_os = "macos") && state.logo();
                self.egui_input.modifiers.command = if cfg!(target_os = "macos") {
                    state.logo()
                } else {
                    state.ctrl()
                };
                EventResponse {
                    repaint: true,
                    consumed: false,
                }
            }

            // Things that may require repaint:
            WindowEvent::CloseRequested => {
                if let Some(viewport_info) = self.egui_input.viewports.get_mut(&viewport_id) {
                    viewport_info.close_requested = true;
                }
                EventResponse {
                    consumed: true,
                    repaint: true,
                }
            }
            WindowEvent::CursorEntered { .. }
            | WindowEvent::Destroyed
            | WindowEvent::Occluded(_)
            | WindowEvent::Resized(_)
            | WindowEvent::Moved(_)
            | WindowEvent::ThemeChanged(_)
            | WindowEvent::TouchpadPressure { .. } => EventResponse {
                repaint: true,
                consumed: false,
            },

            // Things we completely ignore:
            WindowEvent::AxisMotion { .. }
            | WindowEvent::SmartMagnify { .. }
            | WindowEvent::TouchpadRotate { .. } => EventResponse {
                repaint: false,
                consumed: false,
            },

            WindowEvent::TouchpadMagnify { delta, .. } => {
                // Positive delta values indicate magnification (zooming in).
                // Negative delta values indicate shrinking (zooming out).
                let zoom_factor = (*delta as f32).exp();
                self.egui_input.events.push(egui::Event::Zoom(zoom_factor));
                EventResponse {
                    repaint: true,
                    consumed: egui_ctx.wants_pointer_input(),
                }
            }
        }
    }

    /// Call this when there is a new [`accesskit::ActionRequest`].
    ///
    /// The result can be found in [`Self::egui_input`] and be extracted with [`Self::take_egui_input`].
    #[cfg(feature = "accesskit")]
    pub fn on_accesskit_action_request(&mut self, request: accesskit::ActionRequest) {
        self.egui_input
            .events
            .push(egui::Event::AccessKitActionRequest(request));
    }

    fn on_mouse_button_input(
        &mut self,
        state: winit::event::ElementState,
        button: winit::event::MouseButton,
    ) {
        if let Some(pos) = self.pointer_pos_in_points {
            if let Some(button) = translate_mouse_button(button) {
                let pressed = state == winit::event::ElementState::Pressed;

                self.egui_input.events.push(egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers: self.egui_input.modifiers,
                });

                if self.simulate_touch_screen {
                    if pressed {
                        self.any_pointer_button_down = true;

                        self.egui_input.events.push(egui::Event::Touch {
                            device_id: egui::TouchDeviceId(0),
                            id: egui::TouchId(0),
                            phase: egui::TouchPhase::Start,
                            pos,
                            force: None,
                        });
                    } else {
                        self.any_pointer_button_down = false;

                        self.egui_input.events.push(egui::Event::PointerGone);

                        self.egui_input.events.push(egui::Event::Touch {
                            device_id: egui::TouchDeviceId(0),
                            id: egui::TouchId(0),
                            phase: egui::TouchPhase::End,
                            pos,
                            force: None,
                        });
                    };
                }
            }
        }
    }

    fn on_cursor_moved(&mut self, pos_in_pixels: winit::dpi::PhysicalPosition<f64>) {
        let pos_in_points = egui::pos2(
            pos_in_pixels.x as f32 / self.pixels_per_point(),
            pos_in_pixels.y as f32 / self.pixels_per_point(),
        );
        self.pointer_pos_in_points = Some(pos_in_points);

        if self.simulate_touch_screen {
            if self.any_pointer_button_down {
                self.egui_input
                    .events
                    .push(egui::Event::PointerMoved(pos_in_points));

                self.egui_input.events.push(egui::Event::Touch {
                    device_id: egui::TouchDeviceId(0),
                    id: egui::TouchId(0),
                    phase: egui::TouchPhase::Move,
                    pos: pos_in_points,
                    force: None,
                });
            }
        } else {
            self.egui_input
                .events
                .push(egui::Event::PointerMoved(pos_in_points));
        }
    }

    fn on_touch(&mut self, touch: &winit::event::Touch) {
        // Emit touch event
        self.egui_input.events.push(egui::Event::Touch {
            device_id: egui::TouchDeviceId(egui::epaint::util::hash(touch.device_id)),
            id: egui::TouchId::from(touch.id),
            phase: match touch.phase {
                winit::event::TouchPhase::Started => egui::TouchPhase::Start,
                winit::event::TouchPhase::Moved => egui::TouchPhase::Move,
                winit::event::TouchPhase::Ended => egui::TouchPhase::End,
                winit::event::TouchPhase::Cancelled => egui::TouchPhase::Cancel,
            },
            pos: egui::pos2(
                touch.location.x as f32 / self.pixels_per_point(),
                touch.location.y as f32 / self.pixels_per_point(),
            ),
            force: match touch.force {
                Some(winit::event::Force::Normalized(force)) => Some(force as f32),
                Some(winit::event::Force::Calibrated {
                    force,
                    max_possible_force,
                    ..
                }) => Some((force / max_possible_force) as f32),
                None => None,
            },
        });
        // If we're not yet translating a touch or we're translating this very
        // touch …
        if self.pointer_touch_id.is_none() || self.pointer_touch_id.unwrap() == touch.id {
            // … emit PointerButton resp. PointerMoved events to emulate mouse
            match touch.phase {
                winit::event::TouchPhase::Started => {
                    self.pointer_touch_id = Some(touch.id);
                    // First move the pointer to the right location
                    self.on_cursor_moved(touch.location);
                    self.on_mouse_button_input(
                        winit::event::ElementState::Pressed,
                        winit::event::MouseButton::Left,
                    );
                }
                winit::event::TouchPhase::Moved => {
                    self.on_cursor_moved(touch.location);
                }
                winit::event::TouchPhase::Ended => {
                    self.pointer_touch_id = None;
                    self.on_mouse_button_input(
                        winit::event::ElementState::Released,
                        winit::event::MouseButton::Left,
                    );
                    // The pointer should vanish completely to not get any
                    // hover effects
                    self.pointer_pos_in_points = None;
                    self.egui_input.events.push(egui::Event::PointerGone);
                }
                winit::event::TouchPhase::Cancelled => {
                    self.pointer_touch_id = None;
                    self.pointer_pos_in_points = None;
                    self.egui_input.events.push(egui::Event::PointerGone);
                }
            }
        }
    }

    fn on_mouse_wheel(&mut self, delta: winit::event::MouseScrollDelta) {
        {
            let (unit, delta) = match delta {
                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                    (egui::MouseWheelUnit::Line, egui::vec2(x, y))
                }
                winit::event::MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition {
                    x,
                    y,
                }) => (
                    egui::MouseWheelUnit::Point,
                    egui::vec2(x as f32, y as f32) / self.pixels_per_point(),
                ),
            };
            let modifiers = self.egui_input.modifiers;
            self.egui_input.events.push(egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
            });
        }
        let delta = match delta {
            winit::event::MouseScrollDelta::LineDelta(x, y) => {
                let points_per_scroll_line = 50.0; // Scroll speed decided by consensus: https://github.com/emilk/egui/issues/461
                egui::vec2(x, y) * points_per_scroll_line
            }
            winit::event::MouseScrollDelta::PixelDelta(delta) => {
                egui::vec2(delta.x as f32, delta.y as f32) / self.pixels_per_point()
            }
        };

        if self.egui_input.modifiers.ctrl || self.egui_input.modifiers.command {
            // Treat as zoom instead:
            let factor = (delta.y / 200.0).exp();
            self.egui_input.events.push(egui::Event::Zoom(factor));
        } else if self.egui_input.modifiers.shift {
            // Treat as horizontal scrolling.
            // Note: one Mac we already get horizontal scroll events when shift is down.
            self.egui_input
                .events
                .push(egui::Event::Scroll(egui::vec2(delta.x + delta.y, 0.0)));
        } else {
            self.egui_input.events.push(egui::Event::Scroll(delta));
        }
    }

    fn on_keyboard_input(&mut self, input: &winit::event::KeyboardInput) {
        if let Some(keycode) = input.virtual_keycode {
            let pressed = input.state == winit::event::ElementState::Pressed;

            if pressed {
                // VirtualKeyCode::Paste etc in winit are broken/untrustworthy,
                // so we detect these things manually:
                if is_cut_command(self.egui_input.modifiers, keycode) {
                    self.egui_input.events.push(egui::Event::Cut);
                } else if is_copy_command(self.egui_input.modifiers, keycode) {
                    self.egui_input.events.push(egui::Event::Copy);
                } else if is_paste_command(self.egui_input.modifiers, keycode) {
                    if let Some(contents) = self.clipboard.get() {
                        let contents = contents.replace("\r\n", "\n");
                        if !contents.is_empty() {
                            self.egui_input.events.push(egui::Event::Paste(contents));
                        }
                    }
                }
            }

            if let Some(key) = translate_virtual_key_code(keycode) {
                self.egui_input.events.push(egui::Event::Key {
                    key,
                    pressed,
                    repeat: false, // egui will fill this in for us!
                    modifiers: self.egui_input.modifiers,
                });
            }
        }
    }

    /// Call with the output given by `egui`.
    ///
    /// This will, if needed:
    /// * update the cursor
    /// * copy text to the clipboard
    /// * open any clicked urls
    /// * update the IME
    /// *
    pub fn handle_platform_output(
        &mut self,
        window: &Window,
        viewport_id: ViewportId,
        egui_ctx: &egui::Context,
        platform_output: egui::PlatformOutput,
    ) {
        crate::profile_function!();

        let egui::PlatformOutput {
            cursor_icon,
            open_url,
            copied_text,
            events: _,                    // handled elsewhere
            mutable_text_under_cursor: _, // only used in eframe web
            text_cursor_pos,
            #[cfg(feature = "accesskit")]
            accesskit_update,
        } = platform_output;

        self.current_pixels_per_point = egui_ctx.input_for(viewport_id, |i| i.pixels_per_point); // someone can have changed it to scale the UI

        self.set_cursor_icon(window, cursor_icon);

        if let Some(open_url) = open_url {
            open_url_in_browser(&open_url.url);
        }

        if !copied_text.is_empty() {
            self.clipboard.set(copied_text);
        }

        let allow_ime = text_cursor_pos.is_some();
        if self.allow_ime != allow_ime {
            self.allow_ime = allow_ime;
            window.set_ime_allowed(allow_ime);
        }

        if let Some(egui::Pos2 { x, y }) = text_cursor_pos {
            window.set_ime_position(winit::dpi::LogicalPosition { x, y });
        }

        #[cfg(feature = "accesskit")]
        if let Some(accesskit) = self.accesskit.as_ref() {
            if let Some(update) = accesskit_update {
                accesskit.update_if_active(|| update);
            }
        }
    }

    fn set_cursor_icon(&mut self, window: &Window, cursor_icon: egui::CursorIcon) {
        if self.current_cursor_icon == Some(cursor_icon) {
            // Prevent flickering near frame boundary when Windows OS tries to control cursor icon for window resizing.
            // On other platforms: just early-out to save CPU.
            return;
        }

        let is_pointer_in_window = self.pointer_pos_in_points.is_some();
        if is_pointer_in_window {
            self.current_cursor_icon = Some(cursor_icon);

            if let Some(winit_cursor_icon) = translate_cursor(cursor_icon) {
                window.set_cursor_visible(true);
                window.set_cursor_icon(winit_cursor_icon);
            } else {
                window.set_cursor_visible(false);
            }
        } else {
            // Remember to set the cursor again once the cursor returns to the screen:
            self.current_cursor_icon = None;
        }
    }
}

fn update_viewport_info(viewport_info: &mut ViewportInfo, window: &Window, pixels_per_point: f32) {
    crate::profile_function!();

    let has_a_position = match window.is_minimized() {
        None | Some(true) => false,
        Some(false) => true,
    };

    let inner_pos_px = if has_a_position {
        window
            .inner_position()
            .map(|pos| Pos2::new(pos.x as f32, pos.y as f32))
            .ok()
    } else {
        None
    };

    let outer_pos_px = if has_a_position {
        window
            .outer_position()
            .map(|pos| Pos2::new(pos.x as f32, pos.y as f32))
            .ok()
    } else {
        None
    };

    let inner_size_px = if has_a_position {
        let size = window.inner_size();
        Some(Vec2::new(size.width as f32, size.height as f32))
    } else {
        None
    };

    let outer_size_px = if has_a_position {
        let size = window.outer_size();
        Some(Vec2::new(size.width as f32, size.height as f32))
    } else {
        None
    };

    let inner_rect_px = if let (Some(pos), Some(size)) = (inner_pos_px, inner_size_px) {
        Some(Rect::from_min_size(pos, size))
    } else {
        None
    };

    let outer_rect_px = if let (Some(pos), Some(size)) = (outer_pos_px, outer_size_px) {
        Some(Rect::from_min_size(pos, size))
    } else {
        None
    };

    let inner_rect = inner_rect_px.map(|r| r / pixels_per_point);
    let outer_rect = outer_rect_px.map(|r| r / pixels_per_point);

    let monitor = window.current_monitor().is_some();
    let monitor_size = if monitor {
        let size = window
            .current_monitor()
            .unwrap()
            .size()
            .to_logical::<f32>(pixels_per_point.into());
        Some(egui::vec2(size.width, size.height))
    } else {
        None
    };

    viewport_info.title = Some(window.title());
    viewport_info.pixels_per_point = pixels_per_point;
    viewport_info.monitor_size = monitor_size;
    viewport_info.inner_rect = inner_rect;
    viewport_info.outer_rect = outer_rect;
    viewport_info.fullscreen = Some(window.fullscreen().is_some());
    viewport_info.focused = Some(window.has_focus());
}

fn open_url_in_browser(_url: &str) {
    #[cfg(feature = "webbrowser")]
    if let Err(err) = webbrowser::open(_url) {
        log::warn!("Failed to open url: {}", err);
    }

    #[cfg(not(feature = "webbrowser"))]
    {
        log::warn!("Cannot open url - feature \"links\" not enabled.");
    }
}

/// Winit sends special keys (backspace, delete, F1, …) as characters.
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
        winit::event::MouseButton::Other(1) => Some(egui::PointerButton::Extra1),
        winit::event::MouseButton::Other(2) => Some(egui::PointerButton::Extra2),
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
        VirtualKeyCode::Return | VirtualKeyCode::NumpadEnter => Key::Enter,
        VirtualKeyCode::Space => Key::Space,

        VirtualKeyCode::Insert => Key::Insert,
        VirtualKeyCode::Delete => Key::Delete,
        VirtualKeyCode::Home => Key::Home,
        VirtualKeyCode::End => Key::End,
        VirtualKeyCode::PageUp => Key::PageUp,
        VirtualKeyCode::PageDown => Key::PageDown,

        VirtualKeyCode::Minus | VirtualKeyCode::NumpadSubtract => Key::Minus,
        // Using Mac the key with the Plus sign on it is reported as the Equals key
        // (with both English and Swedish keyboard).
        VirtualKeyCode::Equals | VirtualKeyCode::Plus | VirtualKeyCode::NumpadAdd => {
            Key::PlusEquals
        }

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

        VirtualKeyCode::F1 => Key::F1,
        VirtualKeyCode::F2 => Key::F2,
        VirtualKeyCode::F3 => Key::F3,
        VirtualKeyCode::F4 => Key::F4,
        VirtualKeyCode::F5 => Key::F5,
        VirtualKeyCode::F6 => Key::F6,
        VirtualKeyCode::F7 => Key::F7,
        VirtualKeyCode::F8 => Key::F8,
        VirtualKeyCode::F9 => Key::F9,
        VirtualKeyCode::F10 => Key::F10,
        VirtualKeyCode::F11 => Key::F11,
        VirtualKeyCode::F12 => Key::F12,
        VirtualKeyCode::F13 => Key::F13,
        VirtualKeyCode::F14 => Key::F14,
        VirtualKeyCode::F15 => Key::F15,
        VirtualKeyCode::F16 => Key::F16,
        VirtualKeyCode::F17 => Key::F17,
        VirtualKeyCode::F18 => Key::F18,
        VirtualKeyCode::F19 => Key::F19,
        VirtualKeyCode::F20 => Key::F20,

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

        egui::CursorIcon::ResizeEast => Some(winit::window::CursorIcon::EResize),
        egui::CursorIcon::ResizeSouthEast => Some(winit::window::CursorIcon::SeResize),
        egui::CursorIcon::ResizeSouth => Some(winit::window::CursorIcon::SResize),
        egui::CursorIcon::ResizeSouthWest => Some(winit::window::CursorIcon::SwResize),
        egui::CursorIcon::ResizeWest => Some(winit::window::CursorIcon::WResize),
        egui::CursorIcon::ResizeNorthWest => Some(winit::window::CursorIcon::NwResize),
        egui::CursorIcon::ResizeNorth => Some(winit::window::CursorIcon::NResize),
        egui::CursorIcon::ResizeNorthEast => Some(winit::window::CursorIcon::NeResize),
        egui::CursorIcon::ResizeColumn => Some(winit::window::CursorIcon::ColResize),
        egui::CursorIcon::ResizeRow => Some(winit::window::CursorIcon::RowResize),

        egui::CursorIcon::Text => Some(winit::window::CursorIcon::Text),
        egui::CursorIcon::VerticalText => Some(winit::window::CursorIcon::VerticalText),
        egui::CursorIcon::Wait => Some(winit::window::CursorIcon::Wait),
        egui::CursorIcon::ZoomIn => Some(winit::window::CursorIcon::ZoomIn),
        egui::CursorIcon::ZoomOut => Some(winit::window::CursorIcon::ZoomOut),
    }
}

// Helpers for egui Viewports
// ---------------------------------------------------------------------------

pub fn process_viewport_commands(
    info: &mut ViewportInfo,
    commands: impl IntoIterator<Item = ViewportCommand>,
    window: &Window,
    is_viewport_focused: bool,
    screenshot_requested: &mut bool,
) {
    crate::profile_function!();

    use winit::window::ResizeDirection;

    for command in commands {
        match command {
            ViewportCommand::Close => {
                info.close_requested = true;
            }
            ViewportCommand::StartDrag => {
                // If `is_viewport_focused` is not checked on x11 the input will be permanently taken until the app is killed!

                // TODO: check that the left mouse-button was pressed down recently,
                // or we will have bugs on Windows.
                // See https://github.com/emilk/egui/pull/1108
                if is_viewport_focused {
                    if let Err(err) = window.drag_window() {
                        log::warn!("{command:?}: {err}");
                    }
                }
            }
            ViewportCommand::InnerSize(size) => {
                let width = size.x.max(1.0);
                let height = size.y.max(1.0);
                window.set_inner_size(LogicalSize::new(width, height));
            }
            ViewportCommand::BeginResize(direction) => {
                if let Err(err) = window.drag_resize_window(match direction {
                    egui::viewport::ResizeDirection::North => ResizeDirection::North,
                    egui::viewport::ResizeDirection::South => ResizeDirection::South,
                    egui::viewport::ResizeDirection::West => ResizeDirection::West,
                    egui::viewport::ResizeDirection::NorthEast => ResizeDirection::NorthEast,
                    egui::viewport::ResizeDirection::SouthEast => ResizeDirection::SouthEast,
                    egui::viewport::ResizeDirection::NorthWest => ResizeDirection::NorthWest,
                    egui::viewport::ResizeDirection::SouthWest => ResizeDirection::SouthWest,
                }) {
                    log::warn!("{command:?}: {err}");
                }
            }
            ViewportCommand::Title(title) => {
                window.set_title(&title);
            }
            ViewportCommand::Transparent(v) => window.set_transparent(v),
            ViewportCommand::Visible(v) => window.set_visible(v),
            ViewportCommand::OuterPosition(pos) => {
                window.set_outer_position(LogicalPosition::new(pos.x, pos.y));
            }
            ViewportCommand::MinInnerSize(s) => {
                window.set_min_inner_size(
                    (s.is_finite() && s != Vec2::ZERO).then_some(LogicalSize::new(s.x, s.y)),
                );
            }
            ViewportCommand::MaxInnerSize(s) => {
                window.set_max_inner_size(
                    (s.is_finite() && s != Vec2::INFINITY).then_some(LogicalSize::new(s.x, s.y)),
                );
            }
            ViewportCommand::ResizeIncrements(s) => {
                window.set_resize_increments(s.map(|s| LogicalSize::new(s.x, s.y)));
            }
            ViewportCommand::Resizable(v) => window.set_resizable(v),
            ViewportCommand::EnableButtons {
                close,
                minimized,
                maximize,
            } => window.set_enabled_buttons(
                if close {
                    WindowButtons::CLOSE
                } else {
                    WindowButtons::empty()
                } | if minimized {
                    WindowButtons::MINIMIZE
                } else {
                    WindowButtons::empty()
                } | if maximize {
                    WindowButtons::MAXIMIZE
                } else {
                    WindowButtons::empty()
                },
            ),
            ViewportCommand::Minimized(v) => {
                window.set_minimized(v);
                info.minimized = Some(v);
            }
            ViewportCommand::Maximized(v) => {
                window.set_maximized(v);
                info.maximized = Some(v);
            }
            ViewportCommand::Fullscreen(v) => {
                window.set_fullscreen(v.then_some(winit::window::Fullscreen::Borderless(None)));
            }
            ViewportCommand::Decorations(v) => window.set_decorations(v),
            ViewportCommand::WindowLevel(l) => window.set_window_level(match l {
                egui::viewport::WindowLevel::AlwaysOnBottom => WindowLevel::AlwaysOnBottom,
                egui::viewport::WindowLevel::AlwaysOnTop => WindowLevel::AlwaysOnTop,
                egui::viewport::WindowLevel::Normal => WindowLevel::Normal,
            }),
            ViewportCommand::WindowIcon(icon) => {
                window.set_window_icon(icon.map(|icon| {
                    winit::window::Icon::from_rgba(icon.rgba.clone(), icon.width, icon.height)
                        .expect("Invalid ICON data!")
                }));
            }
            ViewportCommand::IMEPosition(pos) => {
                window.set_ime_position(LogicalPosition::new(pos.x, pos.y));
            }
            ViewportCommand::IMEAllowed(v) => window.set_ime_allowed(v),
            ViewportCommand::IMEPurpose(p) => window.set_ime_purpose(match p {
                egui::viewport::IMEPurpose::Password => winit::window::ImePurpose::Password,
                egui::viewport::IMEPurpose::Terminal => winit::window::ImePurpose::Terminal,
                egui::viewport::IMEPurpose::Normal => winit::window::ImePurpose::Normal,
            }),
            ViewportCommand::Focus => {
                if !window.has_focus() {
                    window.focus_window();
                }
            }
            ViewportCommand::RequestUserAttention(a) => {
                window.request_user_attention(match a {
                    egui::UserAttentionType::Reset => None,
                    egui::UserAttentionType::Critical => {
                        Some(winit::window::UserAttentionType::Critical)
                    }
                    egui::UserAttentionType::Informational => {
                        Some(winit::window::UserAttentionType::Informational)
                    }
                });
            }
            ViewportCommand::SetTheme(t) => window.set_theme(match t {
                egui::SystemTheme::Light => Some(winit::window::Theme::Light),
                egui::SystemTheme::Dark => Some(winit::window::Theme::Dark),
                egui::SystemTheme::SystemDefault => None,
            }),
            ViewportCommand::ContentProtected(v) => window.set_content_protected(v),
            ViewportCommand::CursorPosition(pos) => {
                if let Err(err) = window.set_cursor_position(LogicalPosition::new(pos.x, pos.y)) {
                    log::warn!("{command:?}: {err}");
                }
            }
            ViewportCommand::CursorGrab(o) => {
                if let Err(err) = window.set_cursor_grab(match o {
                    egui::viewport::CursorGrab::None => CursorGrabMode::None,
                    egui::viewport::CursorGrab::Confined => CursorGrabMode::Confined,
                    egui::viewport::CursorGrab::Locked => CursorGrabMode::Locked,
                }) {
                    log::warn!("{command:?}: {err}");
                }
            }
            ViewportCommand::CursorVisible(v) => window.set_cursor_visible(v),
            ViewportCommand::MousePassthrough(passthrough) => {
                if let Err(err) = window.set_cursor_hittest(!passthrough) {
                    log::warn!("{command:?}: {err}");
                }
            }
            ViewportCommand::Screenshot => {
                *screenshot_requested = true;
            }
        }
    }
}

pub fn create_winit_window_builder(
    viewport_builder: ViewportBuilder,
) -> winit::window::WindowBuilder {
    crate::profile_function!();

    let ViewportBuilder {
        title,
        position,
        inner_size,
        min_inner_size,
        max_inner_size,
        fullscreen,
        maximized,
        resizable,
        transparent,
        decorations,
        icon,
        active,
        visible,
        close_button,
        minimize_button,
        maximize_button,
        window_level,

        // only handled on some platforms:
        title_hidden: _title_hidden,
        titlebar_transparent: _titlebar_transparent,
        fullsize_content_view: _fullsize_content_view,
        app_id: _app_id,
        drag_and_drop: _drag_and_drop,

        mouse_passthrough: _, // handled in `apply_viewport_builder_to_new_window`
    } = viewport_builder;

    let mut window_builder = winit::window::WindowBuilder::new()
        .with_title(title.unwrap_or_else(|| "egui window".to_owned()))
        .with_transparent(transparent.unwrap_or(false))
        .with_decorations(decorations.unwrap_or(true))
        .with_resizable(resizable.unwrap_or(true))
        .with_visible(visible.unwrap_or(true))
        .with_maximized(maximized.unwrap_or(false))
        .with_window_level(match window_level {
            egui::viewport::WindowLevel::AlwaysOnBottom => WindowLevel::AlwaysOnBottom,
            egui::viewport::WindowLevel::AlwaysOnTop => WindowLevel::AlwaysOnTop,
            egui::viewport::WindowLevel::Normal => WindowLevel::Normal,
        })
        .with_fullscreen(
            fullscreen.and_then(|e| e.then_some(winit::window::Fullscreen::Borderless(None))),
        )
        .with_enabled_buttons({
            let mut buttons = WindowButtons::empty();
            if minimize_button.unwrap_or(true) {
                buttons |= WindowButtons::MINIMIZE;
            }
            if maximize_button.unwrap_or(true) {
                buttons |= WindowButtons::MAXIMIZE;
            }
            if close_button.unwrap_or(true) {
                buttons |= WindowButtons::CLOSE;
            }
            buttons
        })
        .with_active(active.unwrap_or(true));

    if let Some(inner_size) = inner_size {
        window_builder = window_builder
            .with_inner_size(winit::dpi::LogicalSize::new(inner_size.x, inner_size.y));
    }

    if let Some(min_inner_size) = min_inner_size {
        window_builder = window_builder.with_min_inner_size(winit::dpi::LogicalSize::new(
            min_inner_size.x,
            min_inner_size.y,
        ));
    }

    if let Some(max_inner_size) = max_inner_size {
        window_builder = window_builder.with_max_inner_size(winit::dpi::LogicalSize::new(
            max_inner_size.x,
            max_inner_size.y,
        ));
    }

    if let Some(position) = position {
        window_builder =
            window_builder.with_position(winit::dpi::LogicalPosition::new(position.x, position.y));
    }

    if let Some(icon) = icon {
        window_builder = window_builder.with_window_icon(Some(
            winit::window::Icon::from_rgba(icon.rgba.clone(), icon.width, icon.height)
                .expect("Invalid Icon Data!"),
        ));
    }

    #[cfg(all(feature = "wayland", target_os = "linux"))]
    if let Some(app_id) = _app_id {
        use winit::platform::wayland::WindowBuilderExtWayland as _;
        window_builder = window_builder.with_name(app_id, "");
    }

    #[cfg(target_os = "windows")]
    if let Some(enable) = _drag_and_drop {
        use winit::platform::windows::WindowBuilderExtWindows as _;
        window_builder = window_builder.with_drag_and_drop(enable);
    }

    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::WindowBuilderExtMacOS as _;
        window_builder = window_builder
            .with_title_hidden(_title_hidden.unwrap_or(false))
            .with_titlebar_transparent(_titlebar_transparent.unwrap_or(false))
            .with_fullsize_content_view(_fullsize_content_view.unwrap_or(false));
    }

    window_builder
}

/// Applies what `create_winit_window_builder` couldn't
pub fn apply_viewport_builder_to_new_window(window: &Window, builder: &ViewportBuilder) {
    if let Some(mouse_passthrough) = builder.mouse_passthrough {
        if let Err(err) = window.set_cursor_hittest(!mouse_passthrough) {
            log::warn!("set_cursor_hittest failed: {err}");
        }
    }
}

// ---------------------------------------------------------------------------

mod profiling_scopes {
    #![allow(unused_macros)]
    #![allow(unused_imports)]

    /// Profiling macro for feature "puffin"
    macro_rules! profile_function {
        ($($arg: tt)*) => {
            #[cfg(feature = "puffin")]
            #[cfg(not(target_arch = "wasm32"))] // Disabled on web because of the coarse 1ms clock resolution there.
            puffin::profile_function!($($arg)*);
        };
    }
    pub(crate) use profile_function;

    /// Profiling macro for feature "puffin"
    macro_rules! profile_scope {
        ($($arg: tt)*) => {
            #[cfg(feature = "puffin")]
            #[cfg(not(target_arch = "wasm32"))] // Disabled on web because of the coarse 1ms clock resolution there.
            puffin::profile_scope!($($arg)*);
        };
    }
    pub(crate) use profile_scope;
}

#[allow(unused_imports)]
pub(crate) use profiling_scopes::*;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    window::{CursorGrabMode, Window, WindowButtons, WindowLevel},
};
