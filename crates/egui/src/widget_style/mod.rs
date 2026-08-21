// This module is only public with the `experimental_theme` feature,
// so without it a lot of it looks unused:
#![cfg_attr(not(feature = "experimental"), allow(dead_code, unused_imports))]

use core::fmt::Debug;

use emath::{Align2, Vec2};
use epaint::{Color32, FontId, Stroke};

use crate::{
    AtomLayout, Context, FontSelection, Frame, Response, Style, UiStack,
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

/// Visual and layout style shared by widgets built from an [`AtomLayout`].
#[derive(Debug, Clone)]
pub struct AtomLayoutStyle {
    /// Alignment of the atoms within the allocated rectangle.
    ///
    /// `None` uses the alignment of the surrounding [`crate::Ui`].
    pub align2: Option<Align2>,

    /// Minimum size of the atom layout.
    pub min_size: Vec2,

    /// Space between adjacent atoms.
    pub gap: f32,

    /// Frame around the atoms.
    pub frame: Frame,

    /// Fallback visuals for text atoms.
    pub text_style: TextVisuals,

    /// Fallback tint for images whose tint is [`Color32::WHITE`] (untinted).
    pub image_tint: Color32,
}

impl Default for AtomLayoutStyle {
    fn default() -> Self {
        Self {
            align2: None,
            min_size: Vec2::ZERO,
            gap: 0.0,
            frame: Frame::default(),
            text_style: TextVisuals {
                font_id: FontId::default(),
                color: Color32::WHITE,
            },
            image_tint: Color32::WHITE,
        }
    }
}

impl AtomLayoutStyle {
    /// Apply this style to an [`AtomLayout`].
    ///
    /// A per-widget [`AtomLayout::gap`] wins over [`Self::gap`], so widgets like
    /// [`crate::DragValue`] can pack their atoms tighter than the theme does.
    pub fn apply(self, mut layout: AtomLayout<'_>) -> AtomLayout<'_> {
        let Self {
            align2,
            min_size,
            gap,
            frame,
            text_style,
            image_tint,
        } = self;
        layout.map_images(|image| {
            let current_tint = image.image_options().tint;
            // Multiply the tints so they are combined
            image.tint(current_tint * image_tint)
        });

        let layout = layout
            .min_size(min_size)
            .fallback_gap(gap)
            .frame(frame)
            .fallback_font(text_style.font_id)
            .fallback_text_color(text_style.color);

        if let Some(align2) = align2 {
            layout.align2(align2)
        } else {
            layout
        }
    }
}

/// Dedicated button style
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub atom_layout: AtomLayoutStyle,
}

impl WidgetStyle for ButtonStyle {}

/// Dedicated style for a [`crate::Popup`], including menus and tooltips
#[derive(Debug, Clone)]
pub struct PopupStyle {
    /// Frame around the popup's contents, including its padding.
    pub frame: Frame,

    /// Spacing between the items inside the popup.
    ///
    /// A menu wants its items flush against each other, while a tooltip wants them spaced out
    /// like any other content.
    pub item_spacing: Vec2,
}

impl WidgetStyle for PopupStyle {}

/// Dedicated text edit style
#[derive(Debug, Clone)]
pub struct TextEditStyle {
    /// Style of the field's atom layout.
    ///
    /// [`AtomLayoutStyle::frame`] surrounds the text, including its padding, and
    /// [`AtomLayoutStyle::text_style`] is the text being edited.
    pub atom_layout: AtomLayoutStyle,

    /// The color of the hint text shown while the buffer is empty.
    pub hint_text_color: Color32,

    /// The default color of the prefix and suffix atoms.
    pub prefix_suffix_color: Color32,
}

impl WidgetStyle for TextEditStyle {}

/// Dedicated checkbox style
#[derive(Debug, Clone)]
pub struct CheckboxStyle {
    /// Style of the checkbox's atom layout.
    pub atom_layout: AtomLayoutStyle,

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
