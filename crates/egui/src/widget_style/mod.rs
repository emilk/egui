// This module is only public with the `experimental_theme` feature,
// so without it a lot of it looks unused:
#![cfg_attr(not(feature = "experimental"), allow(dead_code, unused_imports))]

mod classes;

pub use self::classes::{ClassName, Classes, HasClasses, ROOT_CLASS, SELECTED_CLASS};

use core::fmt::Debug;

use epaint::{Color32, FontId, Stroke, text::TextWrapMode};

use crate::{
    Context, Frame, Response, Style, UiStack,
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

pub struct StyleArgs<'a> {
    pub classes: &'a Classes,
    pub state: WidgetState,
    pub stack: &'a UiStack,
    pub style: &'a Style,
    pub ctx: &'a Context,
}
