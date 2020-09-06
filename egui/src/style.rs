#![allow(clippy::if_same_then_else)]

use crate::{
    color::*,
    math::*,
    paint::{Stroke, TextStyle},
    types::*,
};

/// Specifies the look and feel of a `Ui`.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Style {
    /// Default `TextStyle` for normal text (i.e. for `Label` and `TextEdit`).
    pub body_text_style: TextStyle,

    pub spacing: Spacing,
    pub interaction: Interaction,
    pub visuals: Visuals,

    /// How many seconds a typical animation should last
    pub animation_time: f32,
}

impl Style {
    // TODO: rename style.interact() to maybe... `style.response_visuals` ?
    /// Use this style for interactive things
    pub fn interact(&self, response: &Response) -> &WidgetVisuals {
        self.visuals.interacted.style(response)
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Spacing {
    /// Horizontal and vertical spacing between widgets
    pub item_spacing: Vec2,

    /// Horizontal and vertical padding within a window frame.
    pub window_padding: Vec2,

    /// Button size is text size plus this on each side
    pub button_padding: Vec2,

    /// Expand buttons by this much *after* allocating them.
    /// This is then mostly a visual change (but also makes them easier to hit with the mouse).
    /// This allows for compact layout where buttons actually eat into item_spacing a bit
    pub button_expand: Vec2,

    /// Indent collapsing regions etc by this much.
    pub indent: f32,

    /// Anything clickable is (at least) this wide.
    pub clickable_diameter: f32,

    /// Total width of a slider
    pub slider_width: f32,

    /// Checkboxes, radio button and collapsing headers have an icon at the start.
    /// The text starts after this many pixels.
    pub icon_width: f32,

    pub menu_bar_height: f32,
}

impl Spacing {
    /// Returns small icon rectangle and big icon rectangle
    pub fn icon_rectangles(&self, rect: Rect) -> (Rect, Rect) {
        let box_side = self.icon_width;
        let big_icon_rect = Rect::from_center_size(
            pos2(rect.left() + box_side / 2.0, rect.center().y),
            vec2(box_side, box_side),
        );

        let small_rect_side = 8.0; // TODO: make a parameter
        let small_icon_rect =
            Rect::from_center_size(big_icon_rect.center(), Vec2::splat(small_rect_side));

        (small_icon_rect, big_icon_rect)
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Interaction {
    /// Mouse must be the close to the side of a window to resize
    pub resize_grab_radius_side: f32,

    /// Mouse must be the close to the corner of a window to resize
    pub resize_grab_radius_corner: f32,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Visuals {
    pub interacted: Interacted,

    pub text_color: Srgba,

    /// For stuff like check marks in check boxes.
    pub line_width: f32,

    pub thin_stroke: Stroke,

    /// e.g. the background of windows
    pub background_fill: Srgba,

    /// e.g. the background of the slider or text edit
    pub dark_bg_color: Srgba,

    pub window_corner_radius: f32,

    pub resize_corner_size: f32,

    /// Blink text cursor by this frequency. If 0, always show the cursor.
    pub cursor_blink_hz: f32,
    pub text_cursor_width: f32,

    /// Allow child widgets to be just on the border and still have a stroke with some thickness
    pub clip_rect_margin: f32,

    // -----------------------------------------------
    // Debug rendering:
    pub debug_widget_rects: bool,
    pub debug_resize: bool,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Interacted {
    pub active: WidgetVisuals,
    pub hovered: WidgetVisuals,
    pub inactive: WidgetVisuals,
    pub disabled: WidgetVisuals,
}

impl Interacted {
    pub fn style(&self, response: &Response) -> &WidgetVisuals {
        if response.active || response.has_kb_focus {
            &self.active
        } else if response.sense == Sense::nothing() {
            &self.disabled
        } else if response.hovered {
            &self.hovered
        } else {
            &self.inactive
        }
    }
}

/// bg = background, fg = foreground.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WidgetVisuals {
    /// Background color of widget
    pub bg_fill: Srgba,

    /// For surrounding rectangle of things that need it,
    /// like buttons, the box of the checkbox, etc.
    pub bg_stroke: Stroke,

    /// Button frames etc
    pub corner_radius: f32,

    /// Fill color of the interactive part of a component (slider grab, checkbox, ...)
    /// When you need a fill.
    pub fg_fill: Srgba,

    /// Stroke and text color of the interactive part of a component (button text, slider grab, check-mark, ...)
    pub fg_stroke: Stroke,
}

impl WidgetVisuals {
    pub fn text_color(&self) -> Srgba {
        self.fg_stroke.color
    }
}

// ----------------------------------------------------------------------------

impl Default for Style {
    fn default() -> Self {
        Self {
            body_text_style: TextStyle::Body,
            spacing: Spacing::default(),
            interaction: Interaction::default(),
            visuals: Visuals::default(),
            animation_time: 1.0 / 15.0,
        }
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            item_spacing: vec2(8.0, 6.0),
            window_padding: vec2(6.0, 6.0),
            button_padding: vec2(2.0, 0.0),
            button_expand: vec2(1.0, 1.0),
            indent: 21.0,
            clickable_diameter: 14.0, // TODO: automatically higher on touch screens
            slider_width: 140.0,
            icon_width: 14.0,
            menu_bar_height: 16.0,
        }
    }
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            resize_grab_radius_side: 5.0,
            resize_grab_radius_corner: 10.0,
        }
    }
}

impl Default for Visuals {
    fn default() -> Self {
        Self {
            interacted: Default::default(),
            text_color: Srgba::gray(160),
            line_width: 0.5,
            thin_stroke: Stroke::new(0.5, GRAY),
            background_fill: Rgba::luminance_alpha(0.013, 0.95).into(),
            dark_bg_color: Srgba::black_alpha(140),
            window_corner_radius: 10.0,
            resize_corner_size: 12.0,
            cursor_blink_hz: 0.0, // 1.0 looks good
            text_cursor_width: 2.0,
            clip_rect_margin: 3.0,
            debug_widget_rects: false,
            debug_resize: false,
        }
    }
}

impl Default for Interacted {
    fn default() -> Self {
        Self {
            active: WidgetVisuals {
                bg_fill: Srgba::black_alpha(128),
                bg_stroke: Stroke::new(2.0, WHITE),
                corner_radius: 4.0,
                fg_fill: srgba(120, 120, 200, 255),
                fg_stroke: Stroke::new(2.0, WHITE),
            },
            hovered: WidgetVisuals {
                bg_fill: Rgba::luminance_alpha(0.06, 0.5).into(),
                bg_stroke: Stroke::new(1.0, Rgba::white_alpha(0.5)),
                corner_radius: 4.0,
                fg_fill: srgba(100, 100, 150, 255),
                fg_stroke: Stroke::new(1.5, Srgba::gray(240)),
            },
            inactive: WidgetVisuals {
                bg_fill: Rgba::luminance_alpha(0.04, 0.5).into(),
                bg_stroke: Stroke::new(1.0, Rgba::white_alpha(0.04)),
                corner_radius: 4.0,
                fg_fill: srgba(60, 60, 80, 255),
                fg_stroke: Stroke::new(0.5, Srgba::gray(200)), // Should NOT look grayed out!
            },
            disabled: WidgetVisuals {
                bg_fill: TRANSPARENT,
                bg_stroke: Stroke::new(0.5, Srgba::gray(70)),
                corner_radius: 4.0,
                fg_fill: srgba(50, 50, 50, 255),
                fg_stroke: Stroke::new(0.5, Srgba::gray(128)), // Should look grayed out
            },
        }
    }
}

// ----------------------------------------------------------------------------

use crate::{widgets::*, Ui};

impl Style {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        if ui.add(Button::new("Reset")).clicked {
            *self = Default::default();
        }

        let Self {
            body_text_style,
            spacing,
            interaction,
            visuals,
            animation_time,
        } = self;
        ui.horizontal_centered(|ui| {
            ui.label("Default text style:");
            for &value in &[TextStyle::Body, TextStyle::Monospace] {
                ui.radio_value(format!("{:?}", value), body_text_style, value);
            }
        });
        ui.collapsing("Spacing", |ui| spacing.ui(ui));
        ui.collapsing("Interaction", |ui| interaction.ui(ui));
        ui.collapsing("Visuals", |ui| visuals.ui(ui));
        ui.add(Slider::f32(animation_time, 0.0..=1.0).text("animation_time"));
    }
}

impl Spacing {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        if ui.add(Button::new("Reset")).clicked {
            *self = Default::default();
        }

        let Self {
            item_spacing,
            window_padding,
            button_padding,
            button_expand,
            indent,
            clickable_diameter,
            slider_width,
            icon_width,
            menu_bar_height,
        } = self;

        ui_slider_vec2(ui, item_spacing, 0.0..=10.0, "item_spacing");
        ui_slider_vec2(ui, window_padding, 0.0..=10.0, "window_padding");
        ui_slider_vec2(ui, button_padding, 0.0..=10.0, "button_padding");
        ui_slider_vec2(ui, button_expand, 0.0..=10.0, "button_expand");
        ui.add(Slider::f32(indent, 0.0..=100.0).text("indent"));
        ui.add(Slider::f32(clickable_diameter, 0.0..=40.0).text("clickable_diameter"));
        ui.add(Slider::f32(slider_width, 0.0..=1000.0).text("slider_width"));
        ui.add(Slider::f32(icon_width, 0.0..=40.0).text("icon_width"));
        ui.add(Slider::f32(menu_bar_height, 0.0..=40.0).text("menu_bar_height"));
    }
}

impl Interaction {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        if ui.add(Button::new("Reset")).clicked {
            *self = Default::default();
        }

        let Self {
            resize_grab_radius_side,
            resize_grab_radius_corner,
        } = self;

        ui.add(Slider::f32(resize_grab_radius_side, 0.0..=20.0).text("resize_grab_radius_side"));
        ui.add(
            Slider::f32(resize_grab_radius_corner, 0.0..=20.0).text("resize_grab_radius_corner"),
        );
    }
}

impl Interacted {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        if ui.add(Button::new("Reset")).clicked {
            *self = Default::default();
        }

        let Self {
            active,
            hovered,
            inactive,
            disabled,
        } = self;

        ui.collapsing("active", |ui| active.ui(ui));
        ui.collapsing("hovered", |ui| hovered.ui(ui));
        ui.collapsing("inactive", |ui| inactive.ui(ui));
        ui.collapsing("disabled", |ui| disabled.ui(ui));
    }
}

impl WidgetVisuals {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            bg_fill,
            bg_stroke,
            corner_radius,
            fg_fill,
            fg_stroke,
        } = self;

        ui_color(ui, bg_fill, "bg_fill");
        bg_stroke.ui(ui, "bg_stroke");
        ui.add(Slider::f32(corner_radius, 0.0..=10.0).text("corner_radius"));
        ui_color(ui, fg_fill, "fg_fill");
        fg_stroke.ui(ui, "fg_stroke");
    }
}

impl Visuals {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        if ui.add(Button::new("Reset")).clicked {
            *self = Default::default();
        }

        let Self {
            interacted,
            text_color,
            line_width,
            thin_stroke,
            background_fill,
            dark_bg_color,
            window_corner_radius,
            resize_corner_size,
            cursor_blink_hz,
            text_cursor_width,
            clip_rect_margin,
            debug_widget_rects,
            debug_resize,
        } = self;

        ui.collapsing("interacted", |ui| interacted.ui(ui));
        ui_color(ui, text_color, "text_color");
        ui.add(Slider::f32(line_width, 0.0..=10.0).text("line_width"));
        thin_stroke.ui(ui, "thin_stroke");
        ui_color(ui, background_fill, "background_fill");
        ui_color(ui, dark_bg_color, "dark_bg_color");
        ui.add(Slider::f32(window_corner_radius, 0.0..=20.0).text("window_corner_radius"));
        ui.add(Slider::f32(resize_corner_size, 0.0..=20.0).text("resize_corner_size"));
        ui.add(Slider::f32(cursor_blink_hz, 0.0..=4.0).text("cursor_blink_hz"));
        ui.add(Slider::f32(text_cursor_width, 0.0..=2.0).text("text_cursor_width"));
        ui.add(Slider::f32(clip_rect_margin, 0.0..=20.0).text("clip_rect_margin"));

        ui.add(Checkbox::new(
            debug_widget_rects,
            "Paint debug rectangles around widgets",
        ));
        ui.add(Checkbox::new(debug_resize, "Debug Resize"));
    }
}

impl Stroke {
    pub fn ui(&mut self, ui: &mut crate::Ui, text: &str) {
        let Self { width, color } = self;
        ui.horizontal_centered(|ui| {
            ui.label(format!("{}: ", text));
            ui.add(DragValue::f32(width).speed(0.1).range(0.0..=5.0))
                .tooltip_text("Width");
            ui_color(ui, color, "Color");
        });
    }
}

// TODO: improve and standardize ui_slider_vec2
fn ui_slider_vec2(ui: &mut Ui, value: &mut Vec2, range: std::ops::RangeInclusive<f32>, text: &str) {
    ui.horizontal_centered(|ui| {
        ui.label(format!("{}: ", text));
        ui.add(Slider::f32(&mut value.x, range.clone()).text("w"));
        ui.add(Slider::f32(&mut value.y, range.clone()).text("h"));
    });
}

// TODO: improve color picker
fn ui_color(ui: &mut Ui, srgba: &mut Srgba, text: &str) {
    ui.horizontal_centered(|ui| {
        ui.label(format!("{} sRGBA: ", text));
        ui.add(DragValue::u8(&mut srgba[0])).tooltip_text("r");
        ui.add(DragValue::u8(&mut srgba[1])).tooltip_text("g");
        ui.add(DragValue::u8(&mut srgba[2])).tooltip_text("b");
        ui.add(DragValue::u8(&mut srgba[3])).tooltip_text("a");
    });
}
