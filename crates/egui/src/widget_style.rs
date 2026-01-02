use emath::Vec2;
use epaint::{Color32, FontId, Shadow, Stroke, text::TextWrapMode};

use crate::{
    Frame, Id, Response, Style, TextStyle, Theme, Ui,
    style::{WidgetVisuals, Widgets},
};

/// General text style
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
pub struct WidgetStyle {
    pub frame: Frame,

    pub text: TextVisuals,

    pub stroke: Stroke,
}

pub struct ButtonStyle {
    pub frame: Frame,
    pub text_style: TextVisuals,
}

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

pub struct LabelStyle {
    /// Frame around
    pub frame: Frame,

    /// Text style
    pub text: TextVisuals,

    /// Wrap mode used
    pub wrap_mode: TextWrapMode,
}

pub struct SeparatorStyle {
    /// How much space is allocated in the layout direction
    pub spacing: f32,

    /// How to paint it
    pub stroke: Stroke,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetState {
    Noninteractive,
    #[default]
    Inactive,
    Hovered,
    Active,
}

impl Widgets {
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

impl Style {
    pub fn widget_style(&self, state: WidgetState) -> WidgetStyle {
        let visuals = self.visuals.widgets.state(state);
        let font_id = self.override_font_id.clone();
        WidgetStyle {
            frame: Frame {
                fill: visuals.bg_fill,
                stroke: visuals.bg_stroke,
                corner_radius: visuals.corner_radius,
                inner_margin: self.spacing.button_padding.into(),
                ..Default::default()
            },
            stroke: visuals.fg_stroke,
            text: TextVisuals {
                color: self
                    .visuals
                    .override_text_color
                    .unwrap_or(visuals.text_color()),
                font_id: font_id.unwrap_or(TextStyle::Body.resolve(self)),
                strikethrough: Stroke::NONE,
                underline: Stroke::NONE,
            },
        }
    }

    pub fn button_style(&self, state: WidgetState, selected: bool) -> ButtonStyle {
        let mut visuals = *self.visuals.widgets.state(state);
        let mut ws = self.widget_style(state);

        if selected {
            visuals.weak_bg_fill = self.visuals.selection.bg_fill;
            visuals.bg_fill = self.visuals.selection.bg_fill;
            visuals.fg_stroke = self.visuals.selection.stroke;
            ws.text.color = self.visuals.selection.stroke.color;
        }

        ButtonStyle {
            frame: Frame {
                fill: visuals.weak_bg_fill,
                stroke: visuals.bg_stroke,
                corner_radius: visuals.corner_radius,
                outer_margin: (-Vec2::splat(visuals.expansion)).into(),
                inner_margin: (self.spacing.button_padding + Vec2::splat(visuals.expansion)
                    - Vec2::splat(visuals.bg_stroke.width))
                .into(),
                ..Default::default()
            },
            text_style: ws.text,
        }
    }

    pub fn checkbox_style(&self, state: WidgetState) -> CheckboxStyle {
        let visuals = self.visuals.widgets.state(state);
        let ws = self.widget_style(state);
        CheckboxStyle {
            frame: Frame::new(),
            checkbox_size: self.spacing.icon_width,
            check_size: self.spacing.icon_width_inner,
            checkbox_frame: Frame {
                fill: visuals.bg_fill,
                corner_radius: visuals.corner_radius,
                stroke: visuals.bg_stroke,
                ..Default::default()
            },
            text_style: ws.text,
            check_stroke: ws.stroke,
        }
    }

    pub fn label_style(&self, state: WidgetState) -> LabelStyle {
        let ws = self.widget_style(state);
        LabelStyle {
            frame: Frame {
                fill: ws.frame.fill,
                inner_margin: 0.0.into(),
                outer_margin: 0.0.into(),
                stroke: Stroke::NONE,
                shadow: Shadow::NONE,
                corner_radius: 0.into(),
            },
            text: ws.text,
            wrap_mode: TextWrapMode::Wrap,
        }
    }

    pub fn separator_style(&self, _state: WidgetState) -> SeparatorStyle {
        let visuals = self.visuals.noninteractive();
        SeparatorStyle {
            spacing: 6.0,
            stroke: visuals.bg_stroke,
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub enum StyleModifier {
    /// Widget type should be a modifier or a separate information ?
    /// Could have the trait [`HasClasses`] force to implement a method "name"
    /// or "`widget_type`"
    Button,
    Label,
    Separator,
    Checkbox,
    /// Classes and Id are string
    Class(String),
    Id(String),
    /// Theme can be useful
    Theme(Theme),
}

/// Text modifiers affect only the text of the widgets
pub enum TextModifier {
    Header,
    Small,
    Weak,
    Strong,
    Code,
}

impl From<&str> for StyleModifier {
    fn from(class: &str) -> Self {
        match class {
            "dark" => Self::Theme(Theme::Dark),
            "light" => Self::Theme(Theme::Light),
            "button" => Self::Button,
            "label" => Self::Label,
            "separator" => Self::Separator,
            "checkbox" => Self::Checkbox,
            // Maybe add a prefix for class and ID ?
            _ => Self::Class(class.to_owned()),
        }
    }
}

pub(crate) const CLASSES_SMALL_VEC_SIZE: usize = 5;

/// Small vec for performance
#[derive(Default)]
pub struct Modifiers {
    pub modifiers: Vec<StyleModifier>,
    text: Option<TextModifier>,
    parent: Option<Id>,
}

impl Modifiers {
    pub fn with_classes(mut self, classes: &[StyleModifier]) -> Self {
        // debug_assert!(
        //     classes.len() <= CLASSES_SMALL_VEC_SIZE - self.modifiers.len(),
        //     "Too many modifiers !"
        // );
        self.modifiers.append(&mut classes.to_vec());
        self
    }

    pub fn with_class(mut self, class: impl Into<StyleModifier>) -> Self {
        self.modifiers.push(class.into());
        self
    }

    pub fn with_parent(mut self, parent: &Ui) -> Self {
        self.parent = Some(parent.id());
        self
    }

    /// Add a class to the list
    pub fn add_if(&mut self, class: impl Into<StyleModifier>, condition: bool) {
        if condition {
            self.modifiers.push(class.into());
        }
    }
    /// Add a class to the list and return the list, for method chaining
    pub fn with_if(mut self, class: impl Into<StyleModifier>, condition: bool) -> Self {
        self.add_if(class.into(), condition);
        self
    }
    pub fn has(&self, class: impl Into<StyleModifier>) -> bool {
        self.modifiers.contains(&class.into())
    }
}

/// Any widgets supporting classes must implement this trait
pub trait HasModifier {
    fn classes(&self) -> &Modifiers;

    fn classes_mut(&mut self) -> &mut Modifiers;

    fn add_class(&mut self, class: impl Into<StyleModifier>) -> &Self {
        self.classes_mut().add_if(class.into(), true);
        self
    }

    fn with_class(mut self, class: impl Into<StyleModifier>) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), true);
        self
    }

    fn with_class_if(mut self, class: impl Into<StyleModifier>, condition: bool) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), condition);
        self
    }
}
