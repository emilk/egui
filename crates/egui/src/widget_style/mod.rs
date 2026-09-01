// This module is only public with the `experimental_theme` feature,
// so without it a lot of it looks unused:
#![cfg_attr(not(feature = "experimental"), allow(dead_code, unused_imports))]

use core::fmt::Debug;
use epaint::{Color32, FontId, Stroke, Vec2};

use crate::{
    Context, FontSelection, Frame, Response, Style, UiStack,
    class::{Classes, HasClasses as _},
    style::{WidgetVisuals, Widgets},
};

/// Each dedicated style must implement this trait to be used in the theme plugin system
pub trait WidgetStyle: Debug + Clone + Send + Sync + core::any::Any + 'static {}

/// General text style
#[derive(Debug, Clone)]
pub struct TextVisuals {
    /// Font used
    pub font_id: FontId,

    /// Font color
    pub color: Color32,
}

impl TextVisuals {
    /// Text in `color`, using the given font.
    ///
    /// `style.override_font_id` wins over `font`, if it is set.
    pub fn new(style: &Style, font: impl Into<FontSelection>, color: Color32) -> Self {
        Self {
            color,
            font_id: style
                .override_font_id
                .clone()
                .unwrap_or_else(|| font.into().resolve(style)),
        }
    }

    /// The text of a widget, colored by the [`WidgetVisuals`] of its current state.
    pub fn from_widget_visuals(
        style: &Style,
        font: impl Into<FontSelection>,
        widget_visuals: &WidgetVisuals,
    ) -> Self {
        Self::new(style, font, widget_visuals.text_color())
    }
}

/// Dedicated button style
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    /// The minimum size of the button before any per-button override.
    pub min_size: Vec2,

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

pub struct StyleArgs<'a> {
    pub classes: &'a Classes,
    pub state: WidgetState,
    pub stack: &'a UiStack,
    pub style: &'a Style,
    pub ctx: &'a Context,
}

impl StyleArgs<'_> {
    /// Does the widget or any of its parents contain this class?
    ///
    /// See also:
    /// - [`Classes::has_class`]
    /// - [`UiStack::has_class`]
    pub fn has_class(&self, class: &str) -> bool {
        self.classes.has_class(class) || self.stack.has_class(class)
    }
}
