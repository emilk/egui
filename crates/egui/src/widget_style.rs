use std::{borrow::Cow, fmt};

use emath::Vec2;
use epaint::{Color32, FontId, Shadow, Stroke, text::TextWrapMode};

use crate::{
    Frame, Response, Style, TextBuffer as _, TextStyle,
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
    pub fn widget_style(&self, _classes: &Classes, state: WidgetState) -> WidgetStyle {
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
                    .unwrap_or_else(|| visuals.text_color()),
                font_id: font_id.unwrap_or_else(|| TextStyle::Body.resolve(self)),
                strikethrough: Stroke::NONE,
                underline: Stroke::NONE,
            },
        }
    }

    pub fn button_style(&self, classes: &Classes, state: WidgetState) -> ButtonStyle {
        let mut visuals = *self.visuals.widgets.state(state);
        let mut ws = self.widget_style(classes, state);

        if classes.has("selected") {
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

    pub fn checkbox_style(&self, classes: &Classes, state: WidgetState) -> CheckboxStyle {
        let visuals = self.visuals.widgets.state(state);
        let ws = self.widget_style(classes, state);
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

    pub fn label_style(&self, classes: &Classes, state: WidgetState) -> LabelStyle {
        let ws = self.widget_style(classes, state);
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

    pub fn separator_style(&self, _classes: &Classes, _state: WidgetState) -> SeparatorStyle {
        let visuals = self.visuals.noninteractive();
        SeparatorStyle {
            spacing: 6.0,
            stroke: visuals.bg_stroke,
        }
    }
}

pub const ROOT_CLASS: &str = "root";

pub type ClassName = Cow<'static, str>;

#[derive(Debug, Default, Clone)]
pub struct Classes {
    classes: Vec<ClassName>,
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

    #[inline]
    fn with_class(mut self, class: impl Into<ClassName>) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), true);
        self
    }

    #[inline]
    fn with_class_if(mut self, class: impl Into<ClassName>, condition: bool) -> Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), condition);
        self
    }

    #[inline]
    fn add_class(&mut self, class: impl Into<ClassName>) -> &mut Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), true);
        self
    }

    #[inline]
    fn add_class_if(&mut self, class: impl Into<ClassName>, condition: bool) -> &mut Self
    where
        Self: Sized,
    {
        self.classes_mut().add_if(class.into(), condition);
        self
    }

    /// Return true if the class is present in the list
    fn has(&self, class: impl Into<ClassName>) -> bool {
        self.classes().classes.contains(&class.into())
    }
}
