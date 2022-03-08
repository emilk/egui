//! The input needed by egui.

use crate::emath::*;

/// What the integrations provides to egui at the start of each frame.
///
/// Set the values that make sense, leave the rest at their `Default::default()`.
///
/// You can check if `egui` is using the inputs using
/// [`crate::Context::wants_pointer_input`] and [`crate::Context::wants_keyboard_input`].
///
/// All coordinates are in points (logical pixels) with origin (0, 0) in the top left corner.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RawInput {
    /// Position and size of the area that egui should use.
    /// Usually you would set this to
    ///
    /// `Some(Rect::from_pos_size(Default::default(), screen_size))`.
    ///
    /// but you could also constrain egui to some smaller portion of your window if you like.
    ///
    /// `None` will be treated as "same as last frame", with the default being a very big area.
    pub screen_rect: Option<Rect>,

    /// Also known as device pixel ratio, > 1 for high resolution screens.
    /// If text looks blurry you probably forgot to set this.
    /// Set this the first frame, whenever it changes, or just on every frame.
    pub pixels_per_point: Option<f32>,

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
    /// drag-and-drop support using `epi::NativeOptions`.
    pub dropped_files: Vec<DroppedFile>,
}

impl Default for RawInput {
    fn default() -> Self {
        Self {
            screen_rect: None,
            pixels_per_point: None,
            max_texture_side: None,
            time: None,
            predicted_dt: 1.0 / 60.0,
            modifiers: Modifiers::default(),
            events: vec![],
            hovered_files: Default::default(),
            dropped_files: Default::default(),
        }
    }
}

impl RawInput {
    /// Helper: move volatile (deltas and events), clone the rest.
    ///
    /// * [`Self::hovered_files`] is cloned.
    /// * [`Self::dropped_files`] is moved.
    pub fn take(&mut self) -> RawInput {
        RawInput {
            screen_rect: self.screen_rect.take(),
            pixels_per_point: self.pixels_per_point.take(),
            max_texture_side: self.max_texture_side.take(),
            time: self.time.take(),
            predicted_dt: self.predicted_dt,
            modifiers: self.modifiers,
            events: std::mem::take(&mut self.events),
            hovered_files: self.hovered_files.clone(),
            dropped_files: std::mem::take(&mut self.dropped_files),
        }
    }

    /// Add on new input.
    pub fn append(&mut self, newer: Self) {
        let Self {
            screen_rect,
            pixels_per_point,
            max_texture_side,
            time,
            predicted_dt,
            modifiers,
            mut events,
            mut hovered_files,
            mut dropped_files,
        } = newer;

        self.screen_rect = screen_rect.or(self.screen_rect);
        self.pixels_per_point = pixels_per_point.or(self.pixels_per_point);
        self.max_texture_side = max_texture_side.or(self.max_texture_side);
        self.time = time; // use latest time
        self.predicted_dt = predicted_dt; // use latest dt
        self.modifiers = modifiers; // use latest
        self.events.append(&mut events);
        self.hovered_files.append(&mut hovered_files);
        self.dropped_files.append(&mut dropped_files);
    }
}

/// A file about to be dropped into egui.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct HoveredFile {
    /// Set by the `egui-winit` backend.
    pub path: Option<std::path::PathBuf>,
    /// With the `egui_web` backend, this is set to the mime-type of the file (if available).
    pub mime: String,
}

/// A file dropped into egui.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DroppedFile {
    /// Set by the `egui-winit` backend.
    pub path: Option<std::path::PathBuf>,
    /// Name of the file. Set by the `egui_web` backend.
    pub name: String,
    /// Set by the `egui_web` backend.
    pub last_modified: Option<std::time::SystemTime>,
    /// Set by the `egui_web` backend.
    pub bytes: Option<epaint::mutex::Arc<[u8]>>,
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
    /// When the user presses enter/return, do not send a `Text` (just [`Key::Enter`]).
    Text(String),
    /// A key was pressed or released.
    Key {
        key: Key,
        /// Was it pressed or released?
        pressed: bool,
        /// The state of the modifier keys at the time of the event.
        modifiers: Modifiers,
    },

    /// The mouse or touch moved to a new place.
    PointerMoved(Pos2),

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

    /// How many points (logical pixels) the user scrolled.
    ///
    /// The direction of the vector indicates how to move the _content_ that is being viewed.
    /// So if you get positive values, the content being viewed should move to the right and down,
    /// revealing new things to the left and up.
    ///
    /// Shift-scroll should result in horizontal scrolling (it is up to the integrations to do this).
    Scroll(Vec2),

    /// Zoom scale factor this frame (e.g. from ctrl-scroll or pinch gesture).
    /// * `zoom = 1`: no change.
    /// * `zoom < 1`: pinch together
    /// * `zoom > 1`: pinch spread
    Zoom(f32),

    /// IME composition start.
    CompositionStart,
    /// A new IME candidate is being suggested.
    CompositionUpdate(String),
    /// IME composition ended with this final result.
    CompositionEnd(String),

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
        /// Describes how hard the touch device was pressed. May always be `0` if the platform does
        /// not support pressure sensitivity.
        /// The value is in the range from 0.0 (no pressure) to 1.0 (maximum pressure).
        force: f32,
    },
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
}

/// Number of pointer buttons supported by egui, i.e. the number of possible states of [`PointerButton`].
pub const NUM_POINTER_BUTTONS: usize = 3;

/// State of the modifier keys. These must be fed to egui.
///
/// The best way to compare `Modifiers` is by using [`Modifiers::matches`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

impl Modifiers {
    pub fn new() -> Self {
        Default::default()
    }

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

    #[inline(always)]
    pub fn is_none(&self) -> bool {
        self == &Self::default()
    }

    #[inline(always)]
    pub fn any(&self) -> bool {
        !self.is_none()
    }

    /// Is shift the only pressed button?
    #[inline(always)]
    pub fn shift_only(&self) -> bool {
        self.shift && !(self.alt || self.command)
    }

    /// true if only [`Self::ctrl`] or only [`Self::mac_cmd`] is pressed.
    #[inline(always)]
    pub fn command_only(&self) -> bool {
        !self.alt && !self.shift && self.command
    }

    /// Check for equality but with proper handling of [`Self::command`].
    ///
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
    pub fn matches(&self, pattern: Modifiers) -> bool {
        // alt and shift must always match the pattern:
        if pattern.alt != self.alt || pattern.shift != self.shift {
            return false;
        }

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
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self {
            alt: self.alt | rhs.alt,
            ctrl: self.ctrl | rhs.ctrl,
            shift: self.shift | rhs.shift,
            mac_cmd: self.mac_cmd | rhs.mac_cmd,
            command: self.command | rhs.command,
        }
    }
}

/// Keyboard keys.
///
/// Includes all keys egui is interested in (such as `Home` and `End`)
/// plus a few that are useful for detecting keyboard shortcuts.
///
/// Many keys are omitted because they are not always physical keys (depending on keyboard language), e.g. `;` and `§`,
/// and are therefor unsuitable as keyboard shortcuts if you want your app to be portable.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Key {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,

    Escape,
    Tab,
    Backspace,
    Enter,
    Space,

    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,

    /// Either from the main row or from the numpad.
    Num0,
    /// Either from the main row or from the numpad.
    Num1,
    /// Either from the main row or from the numpad.
    Num2,
    /// Either from the main row or from the numpad.
    Num3,
    /// Either from the main row or from the numpad.
    Num4,
    /// Either from the main row or from the numpad.
    Num5,
    /// Either from the main row or from the numpad.
    Num6,
    /// Either from the main row or from the numpad.
    Num7,
    /// Either from the main row or from the numpad.
    Num8,
    /// Either from the main row or from the numpad.
    Num9,

    A, // Used for cmd+A (select All)
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K, // Used for ctrl+K (delete text after cursor)
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U, // Used for ctrl+U (delete text before cursor)
    V,
    W, // Used for ctrl+W (delete previous word)
    X,
    Y,
    Z, // Used for cmd+Z (undo)
}

impl RawInput {
    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            screen_rect,
            pixels_per_point,
            max_texture_side,
            time,
            predicted_dt,
            modifiers,
            events,
            hovered_files,
            dropped_files,
        } = self;

        ui.label(format!("screen_rect: {:?} points", screen_rect));
        ui.label(format!("pixels_per_point: {:?}", pixels_per_point))
            .on_hover_text(
                "Also called HDPI factor.\nNumber of physical pixels per each logical pixel.",
            );
        ui.label(format!("max_texture_side: {:?}", max_texture_side));
        if let Some(time) = time {
            ui.label(format!("time: {:.3} s", time));
        } else {
            ui.label("time: None");
        }
        ui.label(format!("predicted_dt: {:.1} ms", 1e3 * predicted_dt));
        ui.label(format!("modifiers: {:#?}", modifiers));
        ui.label(format!("hovered_files: {}", hovered_files.len()));
        ui.label(format!("dropped_files: {}", dropped_files.len()));
        ui.scope(|ui| {
            ui.set_min_height(150.0);
            ui.label(format!("events: {:#?}", events))
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
