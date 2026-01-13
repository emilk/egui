use emath::Vec2;
use epaint::{Color32, FontId, Shadow, Stroke, text::TextWrapMode};

use crate::{
    Frame, Response, Style, TextStyle, Theme,
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
    pub fn widget_style(&self, modifier: &StyleModifiers) -> WidgetStyle {
        let visuals = self.visuals.widgets.state(modifier.state);
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

    pub fn button_style(&self, modifier: &StyleModifiers) -> ButtonStyle {
        let mut visuals = *self.visuals.widgets.state(modifier.state);
        let mut ws = self.widget_style(modifier);

        if modifier.has("selected") {
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

    pub fn checkbox_style(&self, modifier: &StyleModifiers) -> CheckboxStyle {
        let visuals = self.visuals.widgets.state(modifier.state);
        let ws = self.widget_style(modifier);
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

    pub fn label_style(&self, modifier: &StyleModifiers) -> LabelStyle {
        let ws = self.widget_style(modifier);
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

    pub fn separator_style(&self, _modifier: &StyleModifiers) -> SeparatorStyle {
        let visuals = self.visuals.noninteractive();
        SeparatorStyle {
            spacing: 6.0,
            stroke: visuals.bg_stroke,
        }
    }
}

pub type WidgetStyleModifier = String;

/// For now we use [`Vec`] but later we could use [`SmallVec`] for performance
#[derive(Default)]
pub struct StyleModifiers {
    modifiers: Vec<WidgetStyleModifier>,
    theme: Option<Theme>,
    state: WidgetState,
}

impl StyleModifiers {
    /// Add multiples modifiers in one method and return the list for method chaining
    pub fn with_modifiers(mut self, classes: &[WidgetStyleModifier]) -> Self {
        self.modifiers.append(&mut classes.to_vec());
        self
    }

    /// Add a single modifier and return the list for method chaining
    pub fn with_modifier(mut self, class: impl Into<WidgetStyleModifier>) -> Self {
        self.modifiers.push(class.into());
        self
    }

    /// Add a class to the list if the condition is true
    pub fn add_if(&mut self, modifier: impl Into<WidgetStyleModifier>, condition: bool) {
        if condition {
            self.modifiers.push(modifier.into());
        }
    }
    /// Add a class to the list and return the list for method chaining
    pub fn with_if(mut self, modifier: impl Into<WidgetStyleModifier>, condition: bool) -> Self {
        self.add_if(modifier.into(), condition);
        self
    }

    /// Return true if the modifier is present in the list
    pub fn has(&self, modifier: impl Into<WidgetStyleModifier>) -> bool {
        self.modifiers.contains(&modifier.into())
    }

    pub fn with_theme(&mut self, theme: Theme) {
        self.theme = Some(theme);
    }

    pub fn with_state(&mut self, state: WidgetState) {
        self.state = state;
    }
}

/// Any widgets supporting [`StyleModifiers`] must implement this trait
pub trait HasModifiers {
    fn modifiers(&self) -> &StyleModifiers;

    fn modifiers_mut(&mut self) -> &mut StyleModifiers;

    fn add_class(&mut self, modifier: impl Into<WidgetStyleModifier>) -> &Self {
        self.modifiers_mut().add_if(modifier.into(), true);
        self
    }

    fn with_modifier(mut self, modifier: impl Into<WidgetStyleModifier>) -> Self
    where
        Self: Sized,
    {
        self.modifiers_mut().add_if(modifier.into(), true);
        self
    }

    fn with_modifier_if(mut self, modifier: impl Into<WidgetStyleModifier>, condition: bool) -> Self
    where
        Self: Sized,
    {
        self.modifiers_mut().add_if(modifier.into(), condition);
        self
    }

    fn theme(mut self, theme: Theme) -> Self
    where
        Self: Sized,
    {
        self.modifiers_mut().with_theme(theme);
        self
    }
}

/// Add a shortcut to add modifiers. The syntax is add_modifiers!([Name]: (modifier1, modifier2, modifier3,)) for any number of modifiers
#[macro_export]
macro_rules! add_modifiers {
    ($trait_name:ident: ($( $name:ident )+),?) => {

        pub trait $trait_name {
            $(
                fn $name(self) -> Self;
            )*
        }

        impl<T> $trait_name for T
        where
            T: HasModifiers,
        {
                $(fn $name(mut self) -> Self {
                    self.with_modifier(stringify!($name))
                })?
        }
    };
}
