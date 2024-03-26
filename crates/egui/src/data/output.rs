//! All the data egui returns to the backend at the end of each frame.

use crate::{ViewportIdMap, ViewportOutput, WidgetType};

/// What egui emits each frame from [`crate::Context::run`].
///
/// The backend should use this.
#[derive(Clone, Default)]
pub struct FullOutput {
    /// Non-rendering related output.
    pub platform_output: PlatformOutput,

    /// Texture changes since last frame (including the font texture).
    ///
    /// The backend needs to apply [`crate::TexturesDelta::set`] _before_ painting,
    /// and free any texture in [`crate::TexturesDelta::free`] _after_ painting.
    ///
    /// It is assumed that all egui viewports share the same painter and texture namespace.
    pub textures_delta: epaint::textures::TexturesDelta,

    /// What to paint.
    ///
    /// You can use [`crate::Context::tessellate`] to turn this into triangles.
    pub shapes: Vec<epaint::ClippedShape>,

    /// The number of physical pixels per logical ui point, for the viewport that was updated.
    ///
    /// You can pass this to [`crate::Context::tessellate`] together with [`Self::shapes`].
    pub pixels_per_point: f32,

    /// All the active viewports, including the root.
    ///
    /// It is up to the integration to spawn a native window for each viewport,
    /// and to close any window that no longer has a viewport in this map.
    pub viewport_output: ViewportIdMap<ViewportOutput>,
}

impl FullOutput {
    /// Add on new output.
    pub fn append(&mut self, newer: Self) {
        let Self {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output: viewports,
        } = newer;

        self.platform_output.append(platform_output);
        self.textures_delta.append(textures_delta);
        self.shapes = shapes; // Only paint the latest
        self.pixels_per_point = pixels_per_point; // Use latest

        for (id, new_viewport) in viewports {
            match self.viewport_output.entry(id) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(new_viewport);
                }
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().append(new_viewport);
                }
            }
        }
    }
}

/// Information about text being edited.
///
/// Useful for IME.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct IMEOutput {
    /// Where the [`crate::TextEdit`] is located on screen.
    pub rect: crate::Rect,

    /// Where the primary cursor is.
    ///
    /// This is a very thin rectangle.
    pub cursor_rect: crate::Rect,
}

/// The non-rendering part of what egui emits each frame.
///
/// You can access (and modify) this with [`crate::Context::output`].
///
/// The backend should use this.
#[derive(Default, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PlatformOutput {
    /// Set the cursor to this icon.
    pub cursor_icon: CursorIcon,

    /// If set, open this url.
    pub open_url: Option<OpenUrl>,

    /// If set, put this text in the system clipboard. Ignore if empty.
    ///
    /// This is often a response to [`crate::Event::Copy`] or [`crate::Event::Cut`].
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// if ui.button("📋").clicked() {
    ///     ui.output_mut(|o| o.copied_text = "some_text".to_string());
    /// }
    /// # });
    /// ```
    pub copied_text: String,

    /// Events that may be useful to e.g. a screen reader.
    pub events: Vec<OutputEvent>,

    /// Is there a mutable [`TextEdit`](crate::TextEdit) under the cursor?
    /// Use by `eframe` web to show/hide mobile keyboard and IME agent.
    pub mutable_text_under_cursor: bool,

    /// This is et if, and only if, the user is currently editing text.
    ///
    /// Useful for IME.
    pub ime: Option<IMEOutput>,

    /// The difference in the widget tree since last frame.
    ///
    /// NOTE: this needs to be per-viewport.
    #[cfg(feature = "accesskit")]
    pub accesskit_update: Option<accesskit::TreeUpdate>,
}

impl PlatformOutput {
    /// This can be used by a text-to-speech system to describe the events (if any).
    pub fn events_description(&self) -> String {
        // only describe last event:
        if let Some(event) = self.events.iter().next_back() {
            match event {
                OutputEvent::Clicked(widget_info)
                | OutputEvent::DoubleClicked(widget_info)
                | OutputEvent::TripleClicked(widget_info)
                | OutputEvent::FocusGained(widget_info)
                | OutputEvent::TextSelectionChanged(widget_info)
                | OutputEvent::ValueChanged(widget_info) => {
                    return widget_info.description();
                }
            }
        }
        Default::default()
    }

    /// Add on new output.
    pub fn append(&mut self, newer: Self) {
        let Self {
            cursor_icon,
            open_url,
            copied_text,
            mut events,
            mutable_text_under_cursor,
            ime,
            #[cfg(feature = "accesskit")]
            accesskit_update,
        } = newer;

        self.cursor_icon = cursor_icon;
        if open_url.is_some() {
            self.open_url = open_url;
        }
        if !copied_text.is_empty() {
            self.copied_text = copied_text;
        }
        self.events.append(&mut events);
        self.mutable_text_under_cursor = mutable_text_under_cursor;
        self.ime = ime.or(self.ime);

        #[cfg(feature = "accesskit")]
        {
            // egui produces a complete AccessKit tree for each frame,
            // so overwrite rather than appending.
            self.accesskit_update = accesskit_update;
        }
    }

    /// Take everything ephemeral (everything except `cursor_icon` currently)
    pub fn take(&mut self) -> Self {
        let taken = std::mem::take(self);
        self.cursor_icon = taken.cursor_icon; // everything else is ephemeral
        taken
    }
}

/// What URL to open, and how.
///
/// Use with [`crate::Context::open_url`].
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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

/// Types of attention to request from a user when a native window is not in focus.
///
/// See [winit's documentation][user_attention_type] for platform-specific meaning of the attention types.
///
/// [user_attention_type]: https://docs.rs/winit/latest/winit/window/enum.UserAttentionType.html
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum UserAttentionType {
    /// Request an elevated amount of animations and flair for the window and the task bar or dock icon.
    Critical,

    /// Request a standard amount of attention-grabbing actions.
    Informational,

    /// Reset the attention request and interrupt related animations and flashes.
    Reset,
}

/// A mouse cursor icon.
///
/// egui emits a [`CursorIcon`] in [`PlatformOutput`] each frame as a request to the integration.
///
/// Loosely based on <https://developer.mozilla.org/en-US/docs/Web/CSS/cursor>.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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
    /// Something can be scrolled in any direction (panned).
    AllScroll,

    // ------------------------------------
    // Resizing in two directions:
    /// Horizontal resize `-` to make something wider or more narrow (left to/from right)
    ResizeHorizontal,

    /// Diagonal resize `/` (right-up to/from left-down)
    ResizeNeSw,

    /// Diagonal resize `\` (left-up to/from right-down)
    ResizeNwSe,

    /// Vertical resize `|` (up-down or down-up)
    ResizeVertical,

    // ------------------------------------
    // Resizing in one direction:
    /// Resize something rightwards (e.g. when dragging the right-most edge of something)
    ResizeEast,

    /// Resize something down and right (e.g. when dragging the bottom-right corner of something)
    ResizeSouthEast,

    /// Resize something downwards (e.g. when dragging the bottom edge of something)
    ResizeSouth,

    /// Resize something down and left (e.g. when dragging the bottom-left corner of something)
    ResizeSouthWest,

    /// Resize something leftwards (e.g. when dragging the left edge of something)
    ResizeWest,

    /// Resize something up and left (e.g. when dragging the top-left corner of something)
    ResizeNorthWest,

    /// Resize something up (e.g. when dragging the top edge of something)
    ResizeNorth,

    /// Resize something up and right (e.g. when dragging the top-right corner of something)
    ResizeNorthEast,

    // ------------------------------------
    /// Resize a column
    ResizeColumn,

    /// Resize a row
    ResizeRow,

    // ------------------------------------
    // Zooming:
    /// Enhance!
    ZoomIn,

    /// Let's get a better overview
    ZoomOut,
}

impl CursorIcon {
    pub const ALL: [Self; 35] = [
        Self::Default,
        Self::None,
        Self::ContextMenu,
        Self::Help,
        Self::PointingHand,
        Self::Progress,
        Self::Wait,
        Self::Cell,
        Self::Crosshair,
        Self::Text,
        Self::VerticalText,
        Self::Alias,
        Self::Copy,
        Self::Move,
        Self::NoDrop,
        Self::NotAllowed,
        Self::Grab,
        Self::Grabbing,
        Self::AllScroll,
        Self::ResizeHorizontal,
        Self::ResizeNeSw,
        Self::ResizeNwSe,
        Self::ResizeVertical,
        Self::ResizeEast,
        Self::ResizeSouthEast,
        Self::ResizeSouth,
        Self::ResizeSouthWest,
        Self::ResizeWest,
        Self::ResizeNorthWest,
        Self::ResizeNorth,
        Self::ResizeNorthEast,
        Self::ResizeColumn,
        Self::ResizeRow,
        Self::ZoomIn,
        Self::ZoomOut,
    ];
}

impl Default for CursorIcon {
    fn default() -> Self {
        Self::Default
    }
}

/// Things that happened during this frame that the integration may be interested in.
///
/// In particular, these events may be useful for accessibility, i.e. for screen readers.
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum OutputEvent {
    /// A widget was clicked.
    Clicked(WidgetInfo),

    /// A widget was double-clicked.
    DoubleClicked(WidgetInfo),

    /// A widget was triple-clicked.
    TripleClicked(WidgetInfo),

    /// A widget gained keyboard focus (by tab key).
    FocusGained(WidgetInfo),

    /// Text selection was updated.
    TextSelectionChanged(WidgetInfo),

    /// A widget's value changed.
    ValueChanged(WidgetInfo),
}

impl OutputEvent {
    pub fn widget_info(&self) -> &WidgetInfo {
        match self {
            Self::Clicked(info)
            | Self::DoubleClicked(info)
            | Self::TripleClicked(info)
            | Self::FocusGained(info)
            | Self::TextSelectionChanged(info)
            | Self::ValueChanged(info) => info,
        }
    }
}

impl std::fmt::Debug for OutputEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clicked(wi) => write!(f, "Clicked({wi:?})"),
            Self::DoubleClicked(wi) => write!(f, "DoubleClicked({wi:?})"),
            Self::TripleClicked(wi) => write!(f, "TripleClicked({wi:?})"),
            Self::FocusGained(wi) => write!(f, "FocusGained({wi:?})"),
            Self::TextSelectionChanged(wi) => write!(f, "TextSelectionChanged({wi:?})"),
            Self::ValueChanged(wi) => write!(f, "ValueChanged({wi:?})"),
        }
    }
}

/// Describes a widget such as a [`crate::Button`] or a [`crate::TextEdit`].
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WidgetInfo {
    /// The type of widget this is.
    pub typ: WidgetType,

    /// Whether the widget is enabled.
    pub enabled: bool,

    /// The text on labels, buttons, checkboxes etc.
    pub label: Option<String>,

    /// The contents of some editable text (for [`TextEdit`](crate::TextEdit) fields).
    pub current_text_value: Option<String>,

    /// The previous text value.
    pub prev_text_value: Option<String>,

    /// The current value of checkboxes and radio buttons.
    pub selected: Option<bool>,

    /// The current value of sliders etc.
    pub value: Option<f64>,

    /// Selected range of characters in [`Self::current_text_value`].
    pub text_selection: Option<std::ops::RangeInclusive<usize>>,
}

impl std::fmt::Debug for WidgetInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            typ,
            enabled,
            label,
            current_text_value: text_value,
            prev_text_value,
            selected,
            value,
            text_selection,
        } = self;

        let mut s = f.debug_struct("WidgetInfo");

        s.field("typ", typ);
        s.field("enabled", enabled);

        if let Some(label) = label {
            s.field("label", label);
        }
        if let Some(text_value) = text_value {
            s.field("text_value", text_value);
        }
        if let Some(prev_text_value) = prev_text_value {
            s.field("prev_text_value", prev_text_value);
        }
        if let Some(selected) = selected {
            s.field("selected", selected);
        }
        if let Some(value) = value {
            s.field("value", value);
        }
        if let Some(text_selection) = text_selection {
            s.field("text_selection", text_selection);
        }

        s.finish()
    }
}

impl WidgetInfo {
    pub fn new(typ: WidgetType) -> Self {
        Self {
            typ,
            enabled: true,
            label: None,
            current_text_value: None,
            prev_text_value: None,
            selected: None,
            value: None,
            text_selection: None,
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
    pub fn text_edit(prev_text_value: impl ToString, text_value: impl ToString) -> Self {
        let text_value = text_value.to_string();
        let prev_text_value = prev_text_value.to_string();
        let prev_text_value = if text_value == prev_text_value {
            None
        } else {
            Some(prev_text_value)
        };
        Self {
            current_text_value: Some(text_value),
            prev_text_value,
            ..Self::new(WidgetType::TextEdit)
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn text_selection_changed(
        text_selection: std::ops::RangeInclusive<usize>,
        current_text_value: impl ToString,
    ) -> Self {
        Self {
            text_selection: Some(text_selection),
            current_text_value: Some(current_text_value.to_string()),
            ..Self::new(WidgetType::TextEdit)
        }
    }

    /// This can be used by a text-to-speech system to describe the widget.
    pub fn description(&self) -> String {
        let Self {
            typ,
            enabled,
            label,
            current_text_value: text_value,
            prev_text_value: _,
            selected,
            value,
            text_selection: _,
        } = self;

        // TODO(emilk): localization
        let widget_type = match typ {
            WidgetType::Link => "link",
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
            WidgetType::ProgressIndicator => "progress indicator",
            WidgetType::Label | WidgetType::Other => "",
        };

        let mut description = widget_type.to_owned();

        if let Some(selected) = selected {
            if *typ == WidgetType::Checkbox {
                let state = if *selected { "checked" } else { "unchecked" };
                description = format!("{state} {description}");
            } else {
                description += if *selected { "selected" } else { "" };
            };
        }

        if let Some(label) = label {
            description = format!("{label}: {description}");
        }

        if typ == &WidgetType::TextEdit {
            let text = if let Some(text_value) = text_value {
                if text_value.is_empty() {
                    "blank".into()
                } else {
                    text_value.to_string()
                }
            } else {
                "blank".into()
            };
            description = format!("{text}: {description}");
        }

        if let Some(value) = value {
            description += " ";
            description += &value.to_string();
        }

        if !enabled {
            description += ": disabled";
        }
        description.trim().to_owned()
    }
}
