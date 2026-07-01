use std::{
    borrow::Cow,
    fmt::{self, Debug},
};

use epaint::{Color32, FontId, Stroke, text::TextWrapMode};
use smallvec::SmallVec;

use crate::{
    Frame, Response, TextBuffer as _,
    style::{WidgetVisuals, Widgets},
};

/// Each dedicated style must implement this trait to be used in the theme plugin system
pub trait WidgetStyle: Debug + Clone + Send + Sync + std::any::Any + 'static {}

/// General text style
#[derive(Debug, Clone)]
pub struct TextVisuals {
    /// Font used
    pub font_id: FontId,

    /// Font color
    pub color: Color32,

    /// Text decoration
    pub underline: Stroke,
    pub strikethrough: Stroke,
}

/// General widget style
#[derive(Debug, Clone)]
pub struct BaseStyle {
    pub frame: Frame,

    pub text: TextVisuals,

    pub stroke: Stroke,
}

impl WidgetStyle for BaseStyle {}

/// Dedicated button style
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub frame: Frame,
    pub text_style: TextVisuals,
}

impl WidgetStyle for ButtonStyle {}

/// Dedicated checkbox style
#[derive(Debug, Clone)]
pub struct CheckboxStyle {
    /// Frame around
    pub frame: Frame,

    /// Text next to it
    pub text_style: TextVisuals,

    /// Checkbox size
    pub checkbox_size: f32,

    /// Checkmark size
    pub check_size: f32,

    /// Frame of the checkbox itself
    pub checkbox_frame: Frame,

    /// Checkmark stroke
    pub check_stroke: Stroke,
}

impl WidgetStyle for CheckboxStyle {}

/// Dedicated label style
#[derive(Debug, Clone)]
pub struct LabelStyle {
    /// Frame around
    pub frame: Frame,

    /// Text style
    pub text: TextVisuals,

    /// Wrap mode used
    pub wrap_mode: TextWrapMode,
}

impl WidgetStyle for LabelStyle {}

/// Dedicated separator style
#[derive(Debug, Clone)]
pub struct SeparatorStyle {
    /// How much space is allocated in the layout direction
    pub spacing: f32,

    /// How to paint it
    pub stroke: Stroke,
}

impl WidgetStyle for SeparatorStyle {}

/// The different state of a widget can be
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetState {
    Noninteractive,
    #[default]
    Inactive,
    Hovered,
    Active,
}

impl Widgets {
    /// The widget visuals according to the state
    pub fn state(&self, state: WidgetState) -> &WidgetVisuals {
        match state {
            WidgetState::Noninteractive => &self.noninteractive,
            WidgetState::Inactive => &self.inactive,
            WidgetState::Hovered => &self.hovered,
            WidgetState::Active => &self.active,
        }
    }
}

impl Response {
    pub fn widget_state(&self) -> WidgetState {
        if !self.sense.interactive() {
            WidgetState::Noninteractive
        } else if self.is_pointer_button_down_on() || self.has_focus() || self.clicked() {
            WidgetState::Active
        } else if self.hovered() || self.highlighted() {
            WidgetState::Hovered
        } else {
            WidgetState::Inactive
        }
    }
}

/// The root class is a special class present on every top-level [`crate::Ui`].
pub const ROOT_CLASS: &str = "root";

/// The selected class is a special class present on selected [`crate::Button`].
pub const SELECTED_CLASS: &str = "selected";

/// A class is a static string identifier.
pub type ClassName = Cow<'static, str>;

/// Classes are string identifier that can be set on widget/Ui.
///
/// This can be used by styling engine to compute a different style
/// based on the set of classes present on the widget/Ui.
#[derive(Debug, Default, Clone, Hash)]
pub struct Classes {
    classes: SmallVec<[ClassName; 5]>,
}

impl Classes {
    /// Add a class to the list if the condition is true
    #[inline]
    fn add_if(&mut self, class: impl Into<ClassName>, condition: bool) {
        if condition {
            self.classes.push(class.into());
        }
    }
}

impl HasClasses for Classes {
    fn classes(&self) -> &Classes {
        self
    }

    fn classes_mut(&mut self) -> &mut Classes {
        self
    }
}

impl std::fmt::Display for Classes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.classes.iter().for_each(|class| {
            let _ = f.write_str(class.as_str());
        });
        f.write_str("")
    }
}

/// Any widgets supporting [`Classes`] must implement this trait
pub trait HasClasses {
    fn classes(&self) -> &Classes;

    fn classes_mut(&mut self) -> &mut Classes;

    /// Add the given class by consuming [`self`]
    #[inline]
    fn with_class(mut self, class: impl Into<ClassName>) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), true);
        self
    }

    /// Add the given class by consuming [`self`] if the condition is true
    #[inline]
    fn with_class_if(mut self, class: impl Into<ClassName>, condition: bool) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), condition);
        self
    }

    /// Add the given class in-place
    #[inline]
    fn add_class(&mut self, class: impl Into<ClassName>) -> &mut Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), true);
        self
    }

    /// Add the given class in-place if the condition is true
    #[inline]
    fn add_class_if(&mut self, class: impl Into<ClassName>, condition: bool) -> &mut Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), condition);
        self
    }

    /// True if the class is present
    fn has(&self, class: impl Into<ClassName>) -> bool {
        self.classes().classes.contains(&class.into())
    }

    /// The list of class
    fn list(&self) -> Vec<ClassName> {
        self.classes().classes.to_vec()
    }
}
