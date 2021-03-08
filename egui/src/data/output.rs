//! All the data egui returns to the backend at the end of each frame.

/// What egui emits each frame.
/// The backend should use this.
#[derive(Clone, Default, PartialEq)]
pub struct Output {
    /// Set the cursor to this icon.
    pub cursor_icon: CursorIcon,

    /// If set, open this url.
    pub open_url: Option<OpenUrl>,

    /// Response to [`crate::Event::Copy`] or [`crate::Event::Cut`]. Ignore if empty.
    pub copied_text: String,

    /// If `true`, egui is requesting immediate repaint (i.e. on the next frame).
    ///
    /// This happens for instance when there is an animation, or if a user has called `Context::request_repaint()`.
    ///
    /// As an egui user: don't set this value directly.
    /// Call `Context::request_repaint()` instead and it will do so for you.
    pub needs_repaint: bool,

    /// Events that may be useful to e.g. a screen reader.
    pub events: Vec<OutputEvent>,
}

#[derive(Clone, PartialEq)]
pub struct OpenUrl {
    pub url: String,
    /// If `true`, open the url in a new tab.
    /// If `false` open it in the same tab.
    /// Only matters when in a web browser.
    pub new_tab: bool,
}

impl OpenUrl {
    pub fn same_tab(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            new_tab: false,
        }
    }

    pub fn new_tab(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            new_tab: true,
        }
    }
}

/// A mouse cursor icon.
///
/// egui emits a [`CursorIcon`] in [`Output`] each frame as a request to the integration.
#[derive(Clone, Copy, PartialEq)]
pub enum CursorIcon {
    Default,
    /// Pointing hand, used for e.g. web links
    PointingHand,
    ResizeHorizontal,
    ResizeNeSw,
    ResizeNwSe,
    ResizeVertical,
    Text,
    /// Used when moving
    Grab,
    Grabbing,
}

impl Default for CursorIcon {
    fn default() -> Self {
        Self::Default
    }
}

/// Things that happened during this frame that the integration may be interested in.
///
/// In particular, these events may be useful for accessability, i.e. for screen readers.
#[derive(Clone, Debug, PartialEq)]
pub enum OutputEvent {
    /// A widget gained keyboard focus (by tab key).
    ///
    /// An integration can for instance read the newly selected widget out loud for the visually impaired.
    //
    // TODO: we should output state too, e.g. if a checkbox is selected, or current slider value.
    Focused(WidgetType, String),
}

/// The different types of built-in widgets in egui
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WidgetType {
    Label,
    Hyperlink,
    TextEdit,
    Button,
    Checkbox,
    RadioButton,
    SelectableLabel,
    ComboBox,
    Slider,
    DragValue,
    ColorButton,
    ImageButton,
    CollapsingHeader,
}

impl Output {
    /// Open the given url in a web browser.
    /// If egui is running in a browser, the same tab will be reused.
    pub fn open_url(&mut self, url: impl Into<String>) {
        self.open_url = Some(OpenUrl::new_tab(url))
    }

    /// Inform the backend integration that a widget gained focus
    pub fn push_gained_focus_event(&mut self, widget_type: WidgetType, text: impl Into<String>) {
        self.events
            .push(OutputEvent::Focused(widget_type, text.into()));
    }
}
