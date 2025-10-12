use emath::Vec2;
use epaint::{Color32, FontId, Stroke};

use crate::{
    Frame, Response, Style,
    style::{WidgetVisuals, Widgets},
};

/// General text style
pub struct TextVisuals {
    /// Font used
    pub font_id: FontId,
    /// Font color
    pub color: Color32,
}

/// General widget style
pub struct WidgetStyle {
    pub frame: Frame,

    pub text: TextVisuals,

    pub stroke: Stroke,
}

pub struct ButtonStyle {
    pub frame: Frame,
    pub text: TextVisuals,
}

pub struct CheckboxStyle {
    /// Frame around
    pub frame: Frame,
    /// Text next to it
    pub text: TextVisuals,
    /// Size
    pub size: Vec2,
    /// Frame of the checkbox itself
    pub checkbox_frame: Frame,
    /// Checkmark stroke
    pub stroke: Stroke,
}

pub struct DragValueStyle {
    /// Frame around
    pub frame: Frame,
    /// Text of the value
    pub text: TextVisuals,
    pub min_size: Vec2,
}

pub struct HyperlinkStyle {
    pub frame: Frame,
    pub text: TextVisuals,
    pub size: Vec2,
    pub checkbox_frame: Frame,
    pub stroke: Stroke,
}

pub struct ImageStyle {
    pub frame: Frame,
    pub text: TextVisuals,
    pub size: Vec2,
    pub checkbox_frame: Frame,
    pub stroke: Stroke,
}

pub struct LabelStyle {
    pub frame: Frame,
    pub text: TextVisuals,
    pub size: Vec2,
    pub checkbox_frame: Frame,
    pub stroke: Stroke,
}

pub struct RadioButtonStyle {
    pub frame: Frame,
    pub text: TextVisuals,
    pub size: Vec2,
    pub checkbox_frame: Frame,
    pub stroke: Stroke,
}

pub struct SeparatorStyle {
    pub size: f32,
    pub stroke: Stroke,
}

pub struct SliderStyle {
    pub frame: Frame,
    pub text: TextVisuals,
    pub size: Vec2,
    pub checkbox_frame: Frame,
    pub stroke: Stroke,
}

pub struct SpinnerStyle {
    pub frame: Frame,
    pub text: TextVisuals,
    pub size: Vec2,
    pub checkbox_frame: Frame,
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
                ..Default::default()
            },
            stroke: visuals.fg_stroke,
            text: TextVisuals {
                color: visuals.fg_stroke.color,
                font_id: font_id.unwrap_or(FontId::new(13.0, epaint::FontFamily::Proportional)),
            },
        }
    }

    pub fn button_style(&self, state: WidgetState) -> ButtonStyle {
        let ws = self.widget_style(state);
        ButtonStyle {
            frame: ws.frame.inner_margin(self.spacing.button_padding),
            text: ws.text,
        }
    }

    // pub fn checkbox_style(&self, state: WidgetState) -> CheckboxStylee {}

    // pub fn label_style(&self, state: WidgetState) -> LabelStyle {}

    pub fn drag_value_style(&self, state: WidgetState) -> DragValueStyle {
        let ws = self.widget_style(state);
        DragValueStyle {
            frame: ws.frame.inner_margin(self.spacing.button_padding),
            min_size: self.spacing.interact_size,
            text: ws.text,
        }
    }

    // pub fn hyperlink_style(&self, state: WidgetState) -> HyperlinkStyle {}

    // pub fn image_style(&self, state: WidgetState) -> ImageStyle {}

    // pub fn slider_style(&self, state: WidgetState) -> SliderStyle {}

    // pub fn separator_style(&self, state: WidgetState) -> SeparatorStyle {}

    // pub fn spinner_style(&self, state: WidgetState) -> SpinnerStyle {}
}
