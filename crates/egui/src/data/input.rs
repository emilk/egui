//! The input needed by egui.

use epaint::ColorImage;

use crate::{emath::*, Key, Theme, ViewportId, ViewportIdMap};

/// What the integrations provides to egui at the start of each frame.
///
/// Set the values that make sense, leave the rest at their `Default::default()`.
///
/// You can check if `egui` is using the inputs using
/// [`crate::Context::wants_pointer_input`] and [`crate::Context::wants_keyboard_input`].
///
/// All coordinates are in points (logical pixels) with origin (0, 0) in the top left .corner.
///
/// Ii "points" can be calculated from native physical pixels
/// using `pixels_per_point` = [`crate::Context::zoom_factor`] * `native_pixels_per_point`;
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RawInput {
    /// The id of the active viewport.
    pub viewport_id: ViewportId,

    /// Information about all egui viewports.
    pub viewports: ViewportIdMap<ViewportInfo>,

    /// Position and size of the area that egui should use, in points.
    /// Usually you would set this to
    ///
    /// `Some(Rect::from_min_size(Default::default(), screen_size_in_points))`.
    ///
    /// but you could also constrain egui to some smaller portion of your window if you like.
    ///
    /// `None` will be treated as "same as last frame", with the default being a very big area.
    pub screen_rect: Option<Rect>,

    /// Maximum size of one side of the font texture.
    ///
    /// Ask your graphics drivers about this. This corresponds to `GL_MAX_TEXTURE_SIZE`.
    ///
    /// The default is a very small (but very portable) 2048.
    pub max_texture_side: Option<usize>,

    /// Monotonically increasing time, in seconds. Relative to whatever. Used for animations.
    /// If `None` is provided, egui will assume a time delta of `predicted_dt` (default 1/60 seconds).
    pub time: Option<f64>,

    /// Should be set to the expected time between frames when painting at vsync speeds.
    /// The default for this is 1/60.
    /// Can safely be left at its default value.
    pub predicted_dt: f32,

    /// Which modifier keys are down at the start of the frame?
    pub modifiers: Modifiers,

    /// In-order events received this frame.
    ///
    /// There is currently no way to know if egui handles a particular event,
    /// but you can check if egui is using the keyboard with [`crate::Context::wants_keyboard_input`]
    /// and/or the pointer (mouse/touch) with [`crate::Context::is_using_pointer`].
    pub events: Vec<Event>,

    /// Dragged files hovering over egui.
    pub hovered_files: Vec<HoveredFile>,

    /// Dragged files dropped into egui.
    ///
    /// Note: when using `eframe` on Windows you need to enable
    /// drag-and-drop support using `eframe::NativeOptions`.
    pub dropped_files: Vec<DroppedFile>,

    /// The native window has the keyboard focus (i.e. is receiving key presses).
    ///
    /// False when the user alt-tab away from the application, for instance.
    pub focused: bool,

    /// Does the OS use dark or light mode?
    ///
    /// `None` means "don't know".
    pub system_theme: Option<Theme>,
}

impl Default for RawInput {
    fn default() -> Self {
        Self {
            viewport_id: ViewportId::ROOT,
            viewports: std::iter::once((ViewportId::ROOT, Default::default())).collect(),
            screen_rect: None,
            max_texture_side: None,
            time: None,
            predicted_dt: 1.0 / 60.0,
            modifiers: Modifiers::default(),
            events: vec![],
            hovered_files: Default::default(),
            dropped_files: Default::default(),
            focused: true, // integrations opt into global focus tracking
            system_theme: None,
        }
    }
}

impl RawInput {
    /// Info about the active viewport
    #[inline]
    pub fn viewport(&self) -> &ViewportInfo {
        self.viewports.get(&self.viewport_id).expect("Failed to find current viewport in egui RawInput. This is the fault of the egui backend")
    }

    /// Helper: move volatile (deltas and events), clone the rest.
    ///
    /// * [`Self::hovered_files`] is cloned.
    /// * [`Self::dropped_files`] is moved.
    pub fn take(&mut self) -> Self {
        Self {
            viewport_id: self.viewport_id,
            viewports: self.viewports.clone(),
            screen_rect: self.screen_rect.take(),
            max_texture_side: self.max_texture_side.take(),
            time: self.time.take(),
            predicted_dt: self.predicted_dt,
            modifiers: self.modifiers,
            events: std::mem::take(&mut self.events),
            hovered_files: self.hovered_files.clone(),
            dropped_files: std::mem::take(&mut self.dropped_files),
            focused: self.focused,
            system_theme: self.system_theme,
        }
    }

    /// Add on new input.
    pub fn append(&mut self, newer: Self) {
        let Self {
            viewport_id: viewport_ids,
            viewports,
            screen_rect,
            max_texture_side,
            time,
            predicted_dt,
            modifiers,
            mut events,
            mut hovered_files,
            mut dropped_files,
            focused,
            system_theme,
        } = newer;

        self.viewport_id = viewport_ids;
        self.viewports = viewports;
        self.screen_rect = screen_rect.or(self.screen_rect);
        self.max_texture_side = max_texture_side.or(self.max_texture_side);
        self.time = time; // use latest time
        self.predicted_dt = predicted_dt; // use latest dt
        self.modifiers = modifiers; // use latest
        self.events.append(&mut events);
        self.hovered_files.append(&mut hovered_files);
        self.dropped_files.append(&mut dropped_files);
        self.focused = focused;
        self.system_theme = system_theme;
    }
}

/// An input event from the backend into egui, about a specific [viewport](crate::viewport).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ViewportEvent {
    /// The user clicked the close-button on the window, or similar.
    ///
    /// If this is the root viewport, the application will exit
    /// after this frame unless you send a
    /// [`crate::ViewportCommand::CancelClose`] command.
    ///
    /// If this is not the root viewport,
    /// it is up to the user to hide this viewport the next frame.
    ///
    /// This even will wake up both the child and parent viewport.
    Close,
}

/// Information about the current viewport, given as input each frame.
///
/// `None` means "unknown".
///
/// All units are in ui "points", which can be calculated from native physical pixels
/// using `pixels_per_point` = [`crate::Context::zoom_factor`] * `[Self::native_pixels_per_point`];
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ViewportInfo {
    /// Parent viewport, if known.
    pub parent: Option<crate::ViewportId>,

    /// Name of the viewport, if known.
    pub title: Option<String>,

    pub events: Vec<ViewportEvent>,

    /// The OS native pixels-per-point.
    ///
    /// This should always be set, if known.
    ///
    /// On web this takes browser scaling into account,
    /// and corresponds to [`window.devicePixelRatio`](https://developer.mozilla.org/en-US/docs/Web/API/Window/devicePixelRatio) in JavaScript.
    pub native_pixels_per_point: Option<f32>,

    /// Current monitor size in egui points.
    pub monitor_size: Option<Vec2>,

    /// The inner rectangle of the native window, in monitor space and ui points scale.
    ///
    /// This is the content rectangle of the viewport.
    pub inner_rect: Option<Rect>,

    /// The outer rectangle of the native window, in monitor space and ui points scale.
    ///
    /// This is the content rectangle plus decoration chrome.
    pub outer_rect: Option<Rect>,

    /// Are we minimized?
    pub minimized: Option<bool>,

    /// Are we maximized?
    pub maximized: Option<bool>,

    /// Are we in fullscreen mode?
    pub fullscreen: Option<bool>,

    /// Is the window focused and able to receive input?
    ///
    /// This should be the same as [`RawInput::focused`].
    pub focused: Option<bool>,
}

impl ViewportInfo {
    /// This viewport has been told to close.
    ///
    /// If this is the root viewport, the application will exit
    /// after this frame unless you send a
    /// [`crate::ViewportCommand::CancelClose`] command.
    ///
    /// If this is not the root viewport,
    /// it is up to the user to hide this viewport the next frame.
    pub fn close_requested(&self) -> bool {
        self.events
            .iter()
            .any(|&event| event == ViewportEvent::Close)
    }

    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            parent,
            title,
            events,
            native_pixels_per_point,
            monitor_size,
            inner_rect,
            outer_rect,
            minimized,
            maximized,
            fullscreen,
            focused,
        } = self;

        crate::Grid::new("viewport_info").show(ui, |ui| {
            ui.label("Parent:");
            ui.label(opt_as_str(parent));
            ui.end_row();

            ui.label("Title:");
            ui.label(opt_as_str(title));
            ui.end_row();

            ui.label("Events:");
            ui.label(format!("{events:?}"));
            ui.end_row();

            ui.label("Native pixels-per-point:");
            ui.label(opt_as_str(native_pixels_per_point));
            ui.end_row();

            ui.label("Monitor size:");
            ui.label(opt_as_str(monitor_size));
            ui.end_row();

            ui.label("Inner rect:");
            ui.label(opt_rect_as_string(inner_rect));
            ui.end_row();

            ui.label("Outer rect:");
            ui.label(opt_rect_as_string(outer_rect));
            ui.end_row();

            ui.label("Minimized:");
            ui.label(opt_as_str(minimized));
            ui.end_row();

            ui.label("Maximized:");
            ui.label(opt_as_str(maximized));
            ui.end_row();

            ui.label("Fullscreen:");
            ui.label(opt_as_str(fullscreen));
            ui.end_row();

            ui.label("Focused:");
            ui.label(opt_as_str(focused));
            ui.end_row();

            fn opt_rect_as_string(v: &Option<Rect>) -> String {
                v.as_ref().map_or(String::new(), |r| {
                    format!("Pos: {:?}, size: {:?}", r.min, r.size())
                })
            }

            fn opt_as_str<T: std::fmt::Debug>(v: &Option<T>) -> String {
                v.as_ref().map_or(String::new(), |v| format!("{v:?}"))
            }
        });
    }
}

/// A file about to be dropped into egui.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct HoveredFile {
    /// Set by the `egui-winit` backend.
    pub path: Option<std::path::PathBuf>,

    /// With the `eframe` web backend, this is set to the mime-type of the file (if available).
    pub mime: String,
}

/// A file dropped into egui.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DroppedFile {
    /// Set by the `egui-winit` backend.
    pub path: Option<std::path::PathBuf>,

    /// Name of the file. Set by the `eframe` web backend.
    pub name: String,

    /// With the `eframe` web backend, this is set to the mime-type of the file (if available).
    pub mime: String,

    /// Set by the `eframe` web backend.
    pub last_modified: Option<std::time::SystemTime>,

    /// Set by the `eframe` web backend.
    pub bytes: Option<std::sync::Arc<[u8]>>,
}

/// An input event generated by the integration.
///
/// This only covers events that egui cares about.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Event {
    /// The integration detected a "copy" event (e.g. Cmd+C).
    Copy,

    /// The integration detected a "cut" event (e.g. Cmd+X).
    Cut,

    /// The integration detected a "paste" event (e.g. Cmd+V).
    Paste(String),

    /// Text input, e.g. via keyboard.
    ///
    /// When the user presses enter/return, do not send a [`Text`](Event::Text) (just [`Key::Enter`]).
    Text(String),

    /// A key was pressed or released.
    Key {
        /// Most of the time, it's the logical key, heeding the active keymap -- for instance, if the user has Dvorak
        /// keyboard layout, it will be taken into account.
        ///
        /// If it's impossible to determine the logical key on desktop platforms (say, in case of non-Latin letters),
        /// `key` falls back to the value of the corresponding physical key. This is necessary for proper work of
        /// standard shortcuts that only respond to Latin-based bindings (such as `Ctrl` + `V`).
        key: Key,

        /// The physical key, corresponding to the actual position on the keyboard.
        ///
        /// This ignores keymaps, so it is not recommended to use this.
        /// The only thing it makes sense for is things like games,
        /// where e.g. the physical location of WSAD on QWERTY should always map to movement,
        /// even if the user is using Dvorak or AZERTY.
        ///
        /// `eframe` does not (yet) implement this on web.
        physical_key: Option<Key>,

        /// Was it pressed or released?
        pressed: bool,

        /// If this is a `pressed` event, is it a key-repeat?
        ///
        /// On many platforms, holding down a key produces many repeated "pressed" events for it, so called key-repeats.
        /// Sometimes you will want to ignore such events, and this lets you do that.
        ///
        /// egui will automatically detect such repeat events and mark them as such here.
        /// Therefore, if you are writing an egui integration, you do not need to set this (just set it to `false`).
        repeat: bool,

        /// The state of the modifier keys at the time of the event.
        modifiers: Modifiers,
    },

    /// The mouse or touch moved to a new place.
    PointerMoved(Pos2),

    /// The mouse moved, the units are unspecified.
    /// Represents the actual movement of the mouse, without acceleration or clamped by screen edges.
    /// `PointerMoved` and `MouseMoved` can be sent at the same time.
    /// This event is optional. If the integration can not determine unfiltered motion it should not send this event.
    MouseMoved(Vec2),

    /// A mouse button was pressed or released (or a touch started or stopped).
    PointerButton {
        /// Where is the pointer?
        pos: Pos2,

        /// What mouse button? For touches, use [`PointerButton::Primary`].
        button: PointerButton,

        /// Was it the button/touch pressed this frame, or released?
        pressed: bool,

        /// The state of the modifier keys at the time of the event.
        modifiers: Modifiers,
    },

    /// The mouse left the screen, or the last/primary touch input disappeared.
    ///
    /// This means there is no longer a cursor on the screen for hovering etc.
    ///
    /// On touch-up first send `PointerButton{pressed: false, …}` followed by `PointerLeft`.
    PointerGone,

    /// Zoom scale factor this frame (e.g. from a pinch gesture).
    ///
    /// * `zoom = 1`: no change.
    /// * `zoom < 1`: pinch together
    /// * `zoom > 1`: pinch spread
    ///
    /// Note that egui also implement zooming by holding `Ctrl` and scrolling the mouse wheel,
    /// so integration need NOT emit this `Zoom` event in those cases, just [`Self::MouseWheel`].
    ///
    /// As a user, check [`crate::InputState::smooth_scroll_delta`] to see if the user did any zooming this frame.
    Zoom(f32),

    /// IME Event
    Ime(ImeEvent),

    /// On touch screens, report this *in addition to*
    /// [`Self::PointerMoved`], [`Self::PointerButton`], [`Self::PointerGone`]
    Touch {
        /// Hashed device identifier (if available; may be zero).
        /// Can be used to separate touches from different devices.
        device_id: TouchDeviceId,

        /// Unique identifier of a finger/pen. Value is stable from touch down
        /// to lift-up
        id: TouchId,

        /// One of: start move end cancel.
        phase: TouchPhase,

        /// Position of the touch (or where the touch was last detected)
        pos: Pos2,

        /// Describes how hard the touch device was pressed. May always be `None` if the platform does
        /// not support pressure sensitivity.
        /// The value is in the range from 0.0 (no pressure) to 1.0 (maximum pressure).
        force: Option<f32>,
    },

    /// A raw mouse wheel event as sent by the backend.
    ///
    /// Used for scrolling.
    MouseWheel {
        /// The unit of `delta`: points, lines, or pages.
        unit: MouseWheelUnit,

        /// The direction of the vector indicates how to move the _content_ that is being viewed.
        /// So if you get positive values, the content being viewed should move to the right and down,
        /// revealing new things to the left and up.
        ///
        /// A positive X-value indicates the content is being moved right,
        /// as when swiping right on a touch-screen or track-pad with natural scrolling.
        ///
        /// A positive Y-value indicates the content is being moved down,
        /// as when swiping down on a touch-screen or track-pad with natural scrolling.
        delta: Vec2,

        /// The state of the modifier keys at the time of the event.
        modifiers: Modifiers,
    },

    /// The native window gained or lost focused (e.g. the user clicked alt-tab).
    WindowFocused(bool),

    /// An assistive technology (e.g. screen reader) requested an action.
    #[cfg(feature = "accesskit")]
    AccessKitActionRequest(accesskit::ActionRequest),

    /// The reply of a screenshot requested with [`crate::ViewportCommand::Screenshot`].
    Screenshot {
        viewport_id: crate::ViewportId,
        image: std::sync::Arc<ColorImage>,
    },
}

/// IME event.
///
/// See <https://docs.rs/winit/latest/winit/event/enum.Ime.html>
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ImeEvent {
    /// Notifies when the IME was enabled.
    Enabled,

    /// A new IME candidate is being suggested.
    Preedit(String),

    /// IME composition ended with this final result.
    Commit(String),

    /// Notifies when the IME was disabled.
    Disabled,
}

/// Mouse button (or similar for touch input)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum PointerButton {
    /// The primary mouse button is usually the left one.
    Primary = 0,

    /// The secondary mouse button is usually the right one,
    /// and most often used for context menus or other optional things.
    Secondary = 1,

    /// The tertiary mouse button is usually the middle mouse button (e.g. clicking the scroll wheel).
    Middle = 2,

    /// The first extra mouse button on some mice. In web typically corresponds to the Browser back button.
    Extra1 = 3,

    /// The second extra mouse button on some mice. In web typically corresponds to the Browser forward button.
    Extra2 = 4,
}

/// Number of pointer buttons supported by egui, i.e. the number of possible states of [`PointerButton`].
pub const NUM_POINTER_BUTTONS: usize = 5;

/// State of the modifier keys. These must be fed to egui.
///
/// The best way to compare [`Modifiers`] is by using [`Modifiers::matches`].
///
/// NOTE: For cross-platform uses, ALT+SHIFT is a bad combination of modifiers
/// as on mac that is how you type special characters,
/// so those key presses are usually not reported to egui.
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Modifiers {
    /// Either of the alt keys are down (option ⌥ on Mac).
    pub alt: bool,

    /// Either of the control keys are down.
    /// When checking for keyboard shortcuts, consider using [`Self::command`] instead.
    pub ctrl: bool,

    /// Either of the shift keys are down.
    pub shift: bool,

    /// The Mac ⌘ Command key. Should always be set to `false` on other platforms.
    pub mac_cmd: bool,

    /// On Windows and Linux, set this to the same value as `ctrl`.
    /// On Mac, this should be set whenever one of the ⌘ Command keys are down (same as `mac_cmd`).
    /// This is so that egui can, for instance, select all text by checking for `command + A`
    /// and it will work on both Mac and Windows.
    pub command: bool,
}

impl std::fmt::Debug for Modifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_none() {
            return write!(f, "Modifiers::NONE");
        }

        let Self {
            alt,
            ctrl,
            shift,
            mac_cmd,
            command,
        } = *self;

        let mut debug = f.debug_struct("Modifiers");
        if alt {
            debug.field("alt", &true);
        }
        if ctrl {
            debug.field("ctrl", &true);
        }
        if shift {
            debug.field("shift", &true);
        }
        if mac_cmd {
            debug.field("mac_cmd", &true);
        }
        if command {
            debug.field("command", &true);
        }
        debug.finish()
    }
}

impl Modifiers {
    pub const NONE: Self = Self {
        alt: false,
        ctrl: false,
        shift: false,
        mac_cmd: false,
        command: false,
    };

    pub const ALT: Self = Self {
        alt: true,
        ctrl: false,
        shift: false,
        mac_cmd: false,
        command: false,
    };
    pub const CTRL: Self = Self {
        alt: false,
        ctrl: true,
        shift: false,
        mac_cmd: false,
        command: false,
    };
    pub const SHIFT: Self = Self {
        alt: false,
        ctrl: false,
        shift: true,
        mac_cmd: false,
        command: false,
    };

    /// The Mac ⌘ Command key
    pub const MAC_CMD: Self = Self {
        alt: false,
        ctrl: false,
        shift: false,
        mac_cmd: true,
        command: false,
    };

    /// On Mac: ⌘ Command key, elsewhere: Ctrl key
    pub const COMMAND: Self = Self {
        alt: false,
        ctrl: false,
        shift: false,
        mac_cmd: false,
        command: true,
    };

    /// ```
    /// # use egui::Modifiers;
    /// assert_eq!(
    ///     Modifiers::CTRL | Modifiers::ALT,
    ///     Modifiers { ctrl: true, alt: true, ..Default::default() }
    /// );
    /// assert_eq!(
    ///     Modifiers::ALT.plus(Modifiers::CTRL),
    ///     Modifiers::CTRL.plus(Modifiers::ALT),
    /// );
    /// assert_eq!(
    ///     Modifiers::CTRL | Modifiers::ALT,
    ///     Modifiers::CTRL.plus(Modifiers::ALT),
    /// );
    /// ```
    #[inline]
    pub const fn plus(self, rhs: Self) -> Self {
        Self {
            alt: self.alt | rhs.alt,
            ctrl: self.ctrl | rhs.ctrl,
            shift: self.shift | rhs.shift,
            mac_cmd: self.mac_cmd | rhs.mac_cmd,
            command: self.command | rhs.command,
        }
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self == &Self::default()
    }

    #[inline]
    pub fn any(&self) -> bool {
        !self.is_none()
    }

    #[inline]
    pub fn all(&self) -> bool {
        self.alt && self.ctrl && self.shift && self.command
    }

    /// Is shift the only pressed button?
    #[inline]
    pub fn shift_only(&self) -> bool {
        self.shift && !(self.alt || self.command)
    }

    /// true if only [`Self::ctrl`] or only [`Self::mac_cmd`] is pressed.
    #[inline]
    pub fn command_only(&self) -> bool {
        !self.alt && !self.shift && self.command
    }

    /// Checks that the `ctrl/cmd` matches, and that the `shift/alt` of the argument is a subset
    /// of the pressed ksey (`self`).
    ///
    /// This means that if the pattern has not set `shift`, then `self` can have `shift` set or not.
    ///
    /// The reason is that many logical keys require `shift` or `alt` on some keyboard layouts.
    /// For instance, in order to press `+` on an English keyboard, you need to press `shift` and `=`,
    /// but a Swedish keyboard has dedicated `+` key.
    /// So if you want to make a [`KeyboardShortcut`] looking for `Cmd` + `+`, it makes sense
    /// to ignore the shift key.
    /// Similarly, the `Alt` key is sometimes used to type special characters.
    ///
    /// However, if the pattern (the argument) explicitly requires the `shift` or `alt` keys
    /// to be pressed, then they must be pressed.
    ///
    /// # Example:
    /// ```
    /// # use egui::Modifiers;
    /// # let pressed_modifiers = Modifiers::default();
    /// if pressed_modifiers.matches(Modifiers::ALT | Modifiers::SHIFT) {
    ///     // Alt and Shift are pressed, and nothing else
    /// }
    /// ```
    ///
    /// ## Behavior:
    /// ```
    /// # use egui::Modifiers;
    /// assert!(Modifiers::CTRL.matches_logically(Modifiers::CTRL));
    /// assert!(!Modifiers::CTRL.matches_logically(Modifiers::CTRL | Modifiers::SHIFT));
    /// assert!((Modifiers::CTRL | Modifiers::SHIFT).matches_logically(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches_logically(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches_logically(Modifiers::COMMAND));
    /// assert!((Modifiers::MAC_CMD | Modifiers::COMMAND).matches_logically(Modifiers::COMMAND));
    /// assert!(!Modifiers::COMMAND.matches_logically(Modifiers::MAC_CMD));
    /// ```
    pub fn matches_logically(&self, pattern: Self) -> bool {
        if pattern.alt && !self.alt {
            return false;
        }
        if pattern.shift && !self.shift {
            return false;
        }

        self.cmd_ctrl_matches(pattern)
    }

    /// Check for equality but with proper handling of [`Self::command`].
    ///
    /// `self` here are the currently pressed modifiers,
    /// and the argument the pattern we are testing for.
    ///
    /// Note that this will require the `shift` and `alt` keys match, even though
    /// these modifiers are sometimes required to produce some logical keys.
    /// For instance, to press `+` on an English keyboard, you need to press `shift` and `=`,
    /// but on a Swedish keyboard you can press the dedicated `+` key.
    /// Therefore, you often want to use [`Self::matches_logically`] instead.
    ///
    /// # Example:
    /// ```
    /// # use egui::Modifiers;
    /// # let pressed_modifiers = Modifiers::default();
    /// if pressed_modifiers.matches(Modifiers::ALT | Modifiers::SHIFT) {
    ///     // Alt and Shift are pressed, and nothing else
    /// }
    /// ```
    ///
    /// ## Behavior:
    /// ```
    /// # use egui::Modifiers;
    /// assert!(Modifiers::CTRL.matches(Modifiers::CTRL));
    /// assert!(!Modifiers::CTRL.matches(Modifiers::CTRL | Modifiers::SHIFT));
    /// assert!(!(Modifiers::CTRL | Modifiers::SHIFT).matches(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches(Modifiers::COMMAND));
    /// assert!((Modifiers::MAC_CMD | Modifiers::COMMAND).matches(Modifiers::COMMAND));
    /// assert!(!Modifiers::COMMAND.matches(Modifiers::MAC_CMD));
    /// ```
    pub fn matches_exact(&self, pattern: Self) -> bool {
        // alt and shift must always match the pattern:
        if pattern.alt != self.alt || pattern.shift != self.shift {
            return false;
        }

        self.cmd_ctrl_matches(pattern)
    }

    #[deprecated = "Renamed `matches_exact`, but maybe you want to use `matches_logically` instead"]
    pub fn matches(&self, pattern: Self) -> bool {
        self.matches_exact(pattern)
    }

    /// Checks only cmd/ctrl, not alt/shift.
    ///
    /// `self` here are the currently pressed modifiers,
    /// and the argument the pattern we are testing for.
    ///
    /// This takes care to properly handle the difference between
    /// [`Self::ctrl`], [`Self::command`] and [`Self::mac_cmd`].
    pub fn cmd_ctrl_matches(&self, pattern: Self) -> bool {
        if pattern.mac_cmd {
            // Mac-specific match:
            if !self.mac_cmd {
                return false;
            }
            if pattern.ctrl != self.ctrl {
                return false;
            }
            return true;
        }

        if !pattern.ctrl && !pattern.command {
            // the pattern explicitly doesn't want any ctrl/command:
            return !self.ctrl && !self.command;
        }

        // if the pattern is looking for command, then `ctrl` may or may not be set depending on platform.
        // if the pattern is looking for `ctrl`, then `command` may or may not be set depending on platform.

        if pattern.ctrl && !self.ctrl {
            return false;
        }
        if pattern.command && !self.command {
            return false;
        }

        true
    }

    /// Whether another set of modifiers is contained in this set of modifiers with proper handling of [`Self::command`].
    ///
    /// ```
    /// # use egui::Modifiers;
    /// assert!(Modifiers::default().contains(Modifiers::default()));
    /// assert!(Modifiers::CTRL.contains(Modifiers::default()));
    /// assert!(Modifiers::CTRL.contains(Modifiers::CTRL));
    /// assert!(Modifiers::CTRL.contains(Modifiers::COMMAND));
    /// assert!(Modifiers::MAC_CMD.contains(Modifiers::COMMAND));
    /// assert!(Modifiers::COMMAND.contains(Modifiers::MAC_CMD));
    /// assert!(Modifiers::COMMAND.contains(Modifiers::CTRL));
    /// assert!(!(Modifiers::ALT | Modifiers::CTRL).contains(Modifiers::SHIFT));
    /// assert!((Modifiers::CTRL | Modifiers::SHIFT).contains(Modifiers::CTRL));
    /// assert!(!Modifiers::CTRL.contains(Modifiers::CTRL | Modifiers::SHIFT));
    /// ```
    pub fn contains(&self, query: Self) -> bool {
        if query == Self::default() {
            return true;
        }

        let Self {
            alt,
            ctrl,
            shift,
            mac_cmd,
            command,
        } = *self;

        if alt && query.alt {
            return self.contains(Self {
                alt: false,
                ..query
            });
        }
        if shift && query.shift {
            return self.contains(Self {
                shift: false,
                ..query
            });
        }

        if (ctrl || command) && (query.ctrl || query.command) {
            return self.contains(Self {
                command: false,
                ctrl: false,
                ..query
            });
        }
        if (mac_cmd || command) && (query.mac_cmd || query.command) {
            return self.contains(Self {
                mac_cmd: false,
                command: false,
                ..query
            });
        }

        false
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        self.plus(rhs)
    }
}

// ----------------------------------------------------------------------------

/// Names of different modifier keys.
///
/// Used to name modifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModifierNames<'a> {
    pub is_short: bool,

    pub alt: &'a str,
    pub ctrl: &'a str,
    pub shift: &'a str,
    pub mac_cmd: &'a str,
    pub mac_alt: &'a str,

    /// What goes between the names
    pub concat: &'a str,
}

impl ModifierNames<'static> {
    /// ⌥ ⌃ ⇧ ⌘ - NOTE: not supported by the default egui font.
    pub const SYMBOLS: Self = Self {
        is_short: true,
        alt: "⌥",
        ctrl: "⌃",
        shift: "⇧",
        mac_cmd: "⌘",
        mac_alt: "⌥",
        concat: "",
    };

    /// Alt, Ctrl, Shift, Cmd
    pub const NAMES: Self = Self {
        is_short: false,
        alt: "Alt",
        ctrl: "Ctrl",
        shift: "Shift",
        mac_cmd: "Cmd",
        mac_alt: "Option",
        concat: "+",
    };
}

impl<'a> ModifierNames<'a> {
    pub fn format(&self, modifiers: &Modifiers, is_mac: bool) -> String {
        let mut s = String::new();

        let mut append_if = |modifier_is_active, modifier_name| {
            if modifier_is_active {
                if !s.is_empty() {
                    s += self.concat;
                }
                s += modifier_name;
            }
        };

        if is_mac {
            append_if(modifiers.ctrl, self.ctrl);
            append_if(modifiers.shift, self.shift);
            append_if(modifiers.alt, self.mac_alt);
            append_if(modifiers.mac_cmd || modifiers.command, self.mac_cmd);
        } else {
            append_if(modifiers.ctrl || modifiers.command, self.ctrl);
            append_if(modifiers.alt, self.alt);
            append_if(modifiers.shift, self.shift);
        }

        s
    }
}

// ----------------------------------------------------------------------------

/// A keyboard shortcut, e.g. `Ctrl+Alt+W`.
///
/// Can be used with [`crate::InputState::consume_shortcut`]
/// and [`crate::Context::format_shortcut`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct KeyboardShortcut {
    pub modifiers: Modifiers,

    pub logical_key: Key,
}

impl KeyboardShortcut {
    pub const fn new(modifiers: Modifiers, logical_key: Key) -> Self {
        Self {
            modifiers,
            logical_key,
        }
    }

    pub fn format(&self, names: &ModifierNames<'_>, is_mac: bool) -> String {
        let mut s = names.format(&self.modifiers, is_mac);
        if !s.is_empty() {
            s += names.concat;
        }
        if names.is_short {
            s += self.logical_key.symbol_or_name();
        } else {
            s += self.logical_key.name();
        }
        s
    }
}

#[test]
fn format_kb_shortcut() {
    let cmd_shift_f = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::F);
    assert_eq!(
        cmd_shift_f.format(&ModifierNames::NAMES, false),
        "Ctrl+Shift+F"
    );
    assert_eq!(
        cmd_shift_f.format(&ModifierNames::NAMES, true),
        "Shift+Cmd+F"
    );
    assert_eq!(cmd_shift_f.format(&ModifierNames::SYMBOLS, false), "⌃⇧F");
    assert_eq!(cmd_shift_f.format(&ModifierNames::SYMBOLS, true), "⇧⌘F");
}

// ----------------------------------------------------------------------------

impl RawInput {
    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            viewport_id,
            viewports,
            screen_rect,
            max_texture_side,
            time,
            predicted_dt,
            modifiers,
            events,
            hovered_files,
            dropped_files,
            focused,
            system_theme,
        } = self;

        ui.label(format!("Active viwport: {viewport_id:?}"));
        for (id, viewport) in viewports {
            ui.group(|ui| {
                ui.label(format!("Viewport {id:?}"));
                ui.push_id(id, |ui| {
                    viewport.ui(ui);
                });
            });
        }
        ui.label(format!("screen_rect: {screen_rect:?} points"));

        ui.label(format!("max_texture_side: {max_texture_side:?}"));
        if let Some(time) = time {
            ui.label(format!("time: {time:.3} s"));
        } else {
            ui.label("time: None");
        }
        ui.label(format!("predicted_dt: {:.1} ms", 1e3 * predicted_dt));
        ui.label(format!("modifiers: {modifiers:#?}"));
        ui.label(format!("hovered_files: {}", hovered_files.len()));
        ui.label(format!("dropped_files: {}", dropped_files.len()));
        ui.label(format!("focused: {focused}"));
        ui.label(format!("system_theme: {system_theme:?}"));
        ui.scope(|ui| {
            ui.set_min_height(150.0);
            ui.label(format!("events: {events:#?}"))
                .on_hover_text("key presses etc");
        });
    }
}

/// this is a `u64` as values of this kind can always be obtained by hashing
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TouchDeviceId(pub u64);

/// Unique identification of a touch occurrence (finger or pen or …).
/// A Touch ID is valid until the finger is lifted.
/// A new ID is used for the next touch.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TouchId(pub u64);

/// In what phase a touch event is in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TouchPhase {
    /// User just placed a touch point on the touch surface
    Start,

    /// User moves a touch point along the surface. This event is also sent when
    /// any attributes (position, force, …) of the touch point change.
    Move,

    /// User lifted the finger or pen from the surface, or slid off the edge of
    /// the surface
    End,

    /// Touch operation has been disrupted by something (various reasons are possible,
    /// maybe a pop-up alert or any other kind of interruption which may not have
    /// been intended by the user)
    Cancel,
}

/// The unit associated with the numeric value of a mouse wheel event
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum MouseWheelUnit {
    /// Number of ui points (logical pixels)
    Point,

    /// Number of lines
    Line,

    /// Number of pages
    Page,
}

impl From<u64> for TouchId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<i32> for TouchId {
    fn from(id: i32) -> Self {
        Self(id as u64)
    }
}

impl From<u32> for TouchId {
    fn from(id: u32) -> Self {
        Self(id as u64)
    }
}

// ----------------------------------------------------------------------------

// TODO(emilk): generalize this to a proper event filter.
/// Controls which events that a focused widget will have exclusive access to.
///
/// Currently this only controls a few special keyboard events,
/// but in the future this `struct` should be extended into a full callback thing.
///
/// Any events not covered by the filter are given to the widget, but are not exclusive.
#[derive(Clone, Copy, Debug)]
pub struct EventFilter {
    /// If `true`, pressing tab will act on the widget,
    /// and NOT move focus away from the focused widget.
    ///
    /// Default: `false`
    pub tab: bool,

    /// If `true`, pressing horizontal arrows will act on the
    /// widget, and NOT move focus away from the focused widget.
    ///
    /// Default: `false`
    pub horizontal_arrows: bool,

    /// If `true`, pressing vertical arrows will act on the
    /// widget, and NOT move focus away from the focused widget.
    ///
    /// Default: `false`
    pub vertical_arrows: bool,

    /// If `true`, pressing escape will act on the widget,
    /// and NOT surrender focus from the focused widget.
    ///
    /// Default: `false`
    pub escape: bool,
}

#[allow(clippy::derivable_impls)] // let's be explicit
impl Default for EventFilter {
    fn default() -> Self {
        Self {
            tab: false,
            horizontal_arrows: false,
            vertical_arrows: false,
            escape: false,
        }
    }
}

impl EventFilter {
    pub fn matches(&self, event: &Event) -> bool {
        if let Event::Key { key, .. } = event {
            match key {
                crate::Key::Tab => self.tab,
                crate::Key::ArrowUp | crate::Key::ArrowDown => self.vertical_arrows,
                crate::Key::ArrowRight | crate::Key::ArrowLeft => self.horizontal_arrows,
                crate::Key::Escape => self.escape,
                _ => true,
            }
        } else {
            true
        }
    }
}
