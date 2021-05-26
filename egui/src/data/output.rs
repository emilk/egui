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

    /// Position of text edit cursor (used for IME).
    pub text_cursor_pos: Option<crate::Pos2>,
}

impl Output {
    /// Open the given url in a web browser.
    /// If egui is running in a browser, the same tab will be reused.
    pub fn open_url(&mut self, url: impl ToString) {
        self.open_url = Some(OpenUrl::same_tab(url))
    }

    /// This can be used by a text-to-speech system to describe the events (if any).
    pub fn events_description(&self) -> String {
        // only describe last event:
        if let Some(event) = self.events.iter().rev().next() {
            match event {
                OutputEvent::WidgetEvent(WidgetEvent::Focus, widget_info) => {
                    return widget_info.description();
                }
            }
        }
        Default::default()
    }
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
    #[allow(clippy::needless_pass_by_value)]
    pub fn same_tab(url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            new_tab: false,
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn new_tab(url: impl ToString) -> Self {
        Self {
            url: url.to_string(),
            new_tab: true,
        }
    }
}

/// A mouse cursor icon.
///
/// egui emits a [`CursorIcon`] in [`Output`] each frame as a request to the integration.
///
/// Loosely based on <https://developer.mozilla.org/en-US/docs/Web/CSS/cursor>.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CursorIcon {
    /// Normal cursor icon, whatever that is.
    Default,

    /// Show no cursor
    None,

    // ------------------------------------
    // Links and status:
    /// A context menu is available
    ContextMenu,

    /// Question mark
    Help,

    /// Pointing hand, used for e.g. web links
    PointingHand,

    /// Shows that processing is being done, but that the program is still interactive.
    Progress,

    /// Not yet ready, try later.
    Wait,

    // ------------------------------------
    // Selection:
    /// Hover a cell in a table
    Cell,

    /// For precision work
    Crosshair,

    /// Text caret, e.g. "Click here to edit text"
    Text,

    /// Vertical text caret, e.g. "Click here to edit vertical text"
    VerticalText,

    // ------------------------------------
    // Drag-and-drop:
    /// Indicated an alias, e.g. a shortcut
    Alias,

    /// Indicate that a copy will be made
    Copy,

    /// Omnidirectional move icon (e.g. arrows in all cardinal directions)
    Move,

    /// Can't drop here
    NoDrop,

    /// Forbidden
    NotAllowed,

    /// The thing you are hovering can be grabbed
    Grab,

    /// You are grabbing the thing you are hovering
    Grabbing,

    // ------------------------------------
    // Resizing and scrolling
    /// Something can be scrolled in any direction (panned).
    AllScroll,

    /// Horizontal resize `-` to make something wider or more narrow (left to/from right)
    ResizeHorizontal,
    /// Diagonal resize `/` (right-up to/from left-down)
    ResizeNeSw,
    /// Diagonal resize `\` (left-up to/from right-down)
    ResizeNwSe,
    /// Vertical resize `|` (up-down or down-up)
    ResizeVertical,

    /// Enhance!
    ZoomIn,
    /// Let's get a better overview
    ZoomOut,
}

impl CursorIcon {
    pub const ALL: [CursorIcon; 25] = [
        CursorIcon::Default,
        CursorIcon::None,
        CursorIcon::ContextMenu,
        CursorIcon::Help,
        CursorIcon::PointingHand,
        CursorIcon::Progress,
        CursorIcon::Wait,
        CursorIcon::Cell,
        CursorIcon::Crosshair,
        CursorIcon::Text,
        CursorIcon::VerticalText,
        CursorIcon::Alias,
        CursorIcon::Copy,
        CursorIcon::Move,
        CursorIcon::NoDrop,
        CursorIcon::NotAllowed,
        CursorIcon::Grab,
        CursorIcon::Grabbing,
        CursorIcon::AllScroll,
        CursorIcon::ResizeHorizontal,
        CursorIcon::ResizeNeSw,
        CursorIcon::ResizeNwSe,
        CursorIcon::ResizeVertical,
        CursorIcon::ZoomIn,
        CursorIcon::ZoomOut,
    ];
}

impl Default for CursorIcon {
    fn default() -> Self {
        Self::Default
    }
}

/// Things that happened during this frame that the integration may be interested in.
///
/// In particular, these events may be useful for accessability, i.e. for screen readers.
#[derive(Clone, PartialEq)]
pub enum OutputEvent {
    /// A widget gained keyboard focus (by tab key).
    WidgetEvent(WidgetEvent, WidgetInfo),
}

impl std::fmt::Debug for OutputEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WidgetEvent(we, wi) => write!(f, "{:?}: {:?}", we, wi),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidgetEvent {
    /// Keyboard focused moved onto the widget.
    Focus,
    // /// Started hovering a new widget.
    // Hover, // TODO: cursor hovered events
}

/// Describes a widget such as a [`crate::Button`] or a [`crate::TextEdit`].
#[derive(Clone, PartialEq)]
pub struct WidgetInfo {
    /// The type of widget this is.
    pub typ: WidgetType,
    /// The text on labels, buttons, checkboxes etc.
    pub label: Option<String>,
    /// The contents of some editable text (for `TextEdit` fields).
    pub edit_text: Option<String>,
    /// The current value of checkboxes and radio buttons.
    pub selected: Option<bool>,
    /// The current value of sliders etc.
    pub value: Option<f64>,
}

impl std::fmt::Debug for WidgetInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            typ,
            label,
            edit_text,
            selected,
            value,
        } = self;

        let mut s = f.debug_struct("WidgetInfo");

        s.field("typ", typ);

        if let Some(label) = label {
            s.field("label", label);
        }
        if let Some(edit_text) = edit_text {
            s.field("edit_text", edit_text);
        }
        if let Some(selected) = selected {
            s.field("selected", selected);
        }
        if let Some(value) = value {
            s.field("value", value);
        }

        s.finish()
    }
}

impl WidgetInfo {
    pub fn new(typ: WidgetType) -> Self {
        Self {
            typ,
            label: None,
            edit_text: None,
            selected: None,
            value: None,
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn labeled(typ: WidgetType, label: impl ToString) -> Self {
        Self {
            label: Some(label.to_string()),
            ..Self::new(typ)
        }
    }

    /// checkboxes, radio-buttons etc
    #[allow(clippy::needless_pass_by_value)]
    pub fn selected(typ: WidgetType, selected: bool, label: impl ToString) -> Self {
        Self {
            label: Some(label.to_string()),
            selected: Some(selected),
            ..Self::new(typ)
        }
    }

    pub fn drag_value(value: f64) -> Self {
        Self {
            value: Some(value),
            ..Self::new(WidgetType::DragValue)
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn slider(value: f64, label: impl ToString) -> Self {
        let label = label.to_string();
        Self {
            label: if label.is_empty() { None } else { Some(label) },
            value: Some(value),
            ..Self::new(WidgetType::Slider)
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn text_edit(edit_text: impl ToString) -> Self {
        Self {
            edit_text: Some(edit_text.to_string()),
            ..Self::new(WidgetType::TextEdit)
        }
    }

    /// This can be used by a text-to-speech system to describe the widget.
    pub fn description(&self) -> String {
        let Self {
            typ,
            label,
            edit_text,
            selected,
            value,
        } = self;

        // TODO: localization
        let widget_name = match typ {
            WidgetType::Hyperlink => "link",
            WidgetType::TextEdit => "text edit",
            WidgetType::Button => "button",
            WidgetType::Checkbox => "checkbox",
            WidgetType::RadioButton => "radio",
            WidgetType::SelectableLabel => "selectable",
            WidgetType::ComboBox => "combo",
            WidgetType::Slider => "slider",
            WidgetType::DragValue => "drag value",
            WidgetType::ColorButton => "color button",
            WidgetType::ImageButton => "image button",
            WidgetType::CollapsingHeader => "collapsing header",
            WidgetType::Label | WidgetType::Other => "",
        };

        let mut description = widget_name.to_owned();

        if let Some(selected) = selected {
            if *typ == WidgetType::Checkbox {
                description += " ";
                description += if *selected { "checked" } else { "unchecked" };
            } else {
                description += if *selected { "selected" } else { "" };
            };
        }

        if let Some(label) = label {
            description += " ";
            description += label;
        }

        if let Some(edit_text) = edit_text {
            description += " ";
            description += edit_text;
        }

        if let Some(value) = value {
            description += " ";
            description += &value.to_string();
        }

        description.trim().to_owned()
    }
}

/// The different types of built-in widgets in egui
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidgetType {
    Label, // TODO: emit Label events
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

    /// If you cannot fit any of the above slots.
    ///
    /// If this is something you think should be added, file an issue.
    Other,
}
