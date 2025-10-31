use epaint::{Color32, FontId, Shadow, Stroke, text::TextWrapMode};

use crate::{
    Frame, Response, Style, TextStyle,
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
    pub text: TextVisuals,
}

pub struct CheckboxStyle {
    /// Frame around
    pub frame: Frame,
    /// Text next to it
    pub text: TextVisuals,
    /// Checkbox size
    pub size: f32,
    /// Checkmark size
    pub check_size: f32,
    /// Frame of the checkbox itself
    pub checkbox_frame: Frame,
    /// Checkmark stroke
    pub stroke: Stroke,
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
                color: visuals.text_color(),
                font_id: font_id.unwrap_or(TextStyle::Body.resolve(self)),
                strikethrough: Stroke::NONE,
                underline: Stroke::NONE,
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

    pub fn checkbox_style(&self, state: WidgetState) -> CheckboxStyle {
        let visuals = self.visuals.widgets.state(state);
        let ws = self.widget_style(state);
        CheckboxStyle {
            frame: ws.frame.fill(Color32::TRANSPARENT),
            size: self.spacing.icon_width,
            check_size: self.spacing.icon_width_inner,
            checkbox_frame: Frame {
                fill: visuals.weak_bg_fill,
                corner_radius: visuals.corner_radius,
                stroke: visuals.bg_stroke,
                ..Default::default()
            },
            text: ws.text,
            stroke: ws.stroke,
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

    pub fn separator_style(&self, state: WidgetState) -> SeparatorStyle {
        let visuals = self.visuals.widgets.state(state);
        SeparatorStyle {
            spacing: 0.0,
            stroke: visuals.fg_stroke,
        }
    }
}
