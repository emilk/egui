//! Egui theme (spacing, colors, etc).

#![allow(clippy::if_same_then_else)]

use crate::{
    color::*,
    math::*,
    paint::{Shadow, Stroke, TextStyle},
    types::*,
};

/// Specifies the look and feel of a [`Ui`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
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
    // TODO: rename style.interact() to maybe... `style.interactive` ?
    /// Use this style for interactive things.
    /// Note that you must already have a response,
    /// i.e. you must allocate space and interact BEFORE painting the widget!
    pub fn interact(&self, response: &Response) -> &WidgetVisuals {
        self.visuals.widgets.style(response)
    }

    /// Style to use for non-interactive widgets.
    pub fn noninteractive(&self) -> &WidgetVisuals {
        &self.visuals.widgets.noninteractive
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Spacing {
    /// Horizontal and vertical spacing between widgets
    pub item_spacing: Vec2,

    /// Horizontal and vertical padding within a window frame.
    pub window_padding: Vec2,

    /// Button size is text size plus this on each side
    pub button_padding: Vec2,

    /// Indent collapsing regions etc by this much.
    pub indent: f32,

    /// Minimum size of e.g. a button (including padding).
    /// `interact_size.y` is the default height of button, slider, etc.
    /// Anything clickable should be (at least) this size.
    pub interact_size: Vec2, // TODO: rename min_interact_size ?

    /// Default width of a `Slider`.
    pub slider_width: f32, // TODO: rename big_interact_size ?

    /// Default width of a `TextEdit`.
    pub text_edit_width: f32,

    /// Checkboxes, radio button and collapsing headers have an icon at the start.
    /// This is the width/height of this icon.
    pub icon_width: f32,

    /// Checkboxes, radio button and collapsing headers have an icon at the start.
    /// This is the spacing between the icon and the text
    pub icon_spacing: f32,

    /// Width of a tooltip (`on_hover_ui`, `on_hover_text` etc).
    pub tooltip_width: f32,
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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Interaction {
    /// Mouse must be the close to the side of a window to resize
    pub resize_grab_radius_side: f32,

    /// Mouse must be the close to the corner of a window to resize
    pub resize_grab_radius_corner: f32,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Visuals {
    /// Override default text color for all text.
    ///
    /// This is great for setting the color of text for any widget.
    ///
    /// If `text_color` is `None` (default), then the text color will be the same as the
    /// foreground stroke color (`WidgetVisuals::fg_stroke`)
    /// and will depend on wether or not the widget is being interacted with.
    ///
    /// In the future we may instead modulate
    /// the `text_color` based on wether or not it is interacted with
    /// so that `visuals.text_color` is always used,
    /// but its alpha may be different based on whether or not
    /// it is disabled, non-interactive, hovered etc.
    pub override_text_color: Option<Color32>,

    /// Visual styles of widgets
    pub widgets: Widgets,

    pub selection: Selection,

    /// e.g. the background of the slider or text edit,
    /// needs to look different from other interactive stuff.
    pub dark_bg_color: Color32, // TODO: remove, rename, or clarify what it is for

    /// The color used for `Hyperlink`,
    pub hyperlink_color: Color32,

    pub window_corner_radius: f32,
    pub window_shadow: Shadow,

    pub resize_corner_size: f32,

    pub text_cursor_width: f32,

    /// Allow child widgets to be just on the border and still have a stroke with some thickness
    pub clip_rect_margin: f32,

    // -----------------------------------------------
    // Debug rendering:
    /// Show which widgets make their parent wider
    pub debug_expand_width: bool,
    /// Show which widgets make their parent higher
    pub debug_expand_height: bool,
    pub debug_resize: bool,
}

impl Visuals {
    pub fn noninteractive(&self) -> &WidgetVisuals {
        &self.widgets.noninteractive
    }

    pub fn text_color(&self) -> Color32 {
        self.override_text_color
            .unwrap_or_else(|| self.widgets.noninteractive.text_color())
    }
}

/// Selected text, selected elements etc
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Selection {
    pub bg_fill: Color32,
    pub stroke: Stroke,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Widgets {
    /// For a non-interactive widget
    pub noninteractive: WidgetVisuals,
    /// For an otherwise interactive widget that has been disabled
    pub disabled: WidgetVisuals,
    /// For an interactive widget that is "resting"
    pub inactive: WidgetVisuals,
    /// For an interactive widget that is being hovered
    pub hovered: WidgetVisuals,
    /// For an interactive widget that is being interacted with
    pub active: WidgetVisuals,
}

impl Widgets {
    pub fn style(&self, response: &Response) -> &WidgetVisuals {
        if response.active || response.has_kb_focus {
            &self.active
        } else if response.sense == Sense::hover() {
            &self.disabled
        } else if response.hovered {
            &self.hovered
        } else {
            &self.inactive
        }
    }
}

/// bg = background, fg = foreground.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct WidgetVisuals {
    /// Background color of widget
    pub bg_fill: Color32,

    /// For surrounding rectangle of things that need it,
    /// like buttons, the box of the checkbox, etc.
    /// Should maybe be called `frame_stroke`.
    pub bg_stroke: Stroke,

    /// Button frames etc
    pub corner_radius: f32,

    /// Stroke and text color of the interactive part of a component (button text, slider grab, check-mark, ...)
    pub fg_stroke: Stroke,

    /// Make the frame this much larger
    pub expansion: f32,
}

impl WidgetVisuals {
    pub fn text_color(&self) -> Color32 {
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
            animation_time: 1.0 / 12.0,
        }
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            item_spacing: vec2(8.0, 3.0),
            window_padding: Vec2::splat(6.0),
            button_padding: vec2(4.0, 1.0),
            indent: 25.0,
            interact_size: vec2(40.0, 20.0),
            slider_width: 100.0,
            text_edit_width: 280.0,
            icon_width: 16.0,
            icon_spacing: 0.0,
            tooltip_width: 400.0,
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
            override_text_color: None,
            widgets: Default::default(),
            selection: Default::default(),
            dark_bg_color: Color32::from_gray(10),
            hyperlink_color: Color32::from_rgb(90, 170, 255),
            window_corner_radius: 10.0,
            window_shadow: Shadow::big(),
            resize_corner_size: 12.0,
            text_cursor_width: 2.0,
            clip_rect_margin: 3.0, // should be at least half the size of the widest frame stroke + max WidgetVisuals::expansion
            debug_expand_width: false,
            debug_expand_height: false,
            debug_resize: false,
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self {
            bg_fill: Rgba::from_rgb(0.0, 0.5, 1.0)
                .additive()
                .multiply(0.10)
                .into(),
            stroke: Stroke::new(1.0, Rgba::from_rgb(0.3, 0.6, 1.0)),
        }
    }
}

impl Default for Widgets {
    fn default() -> Self {
        Self {
            noninteractive: WidgetVisuals {
                bg_stroke: Stroke::new(1.0, Color32::from_gray(65)), // window outline
                bg_fill: Color32::from_gray(30),                     // window background
                corner_radius: 4.0,

                fg_stroke: Stroke::new(1.0, Color32::from_gray(160)), // text color
                expansion: 0.0,
            },
            disabled: WidgetVisuals {
                bg_fill: Color32::from_gray(40), // Should look grayed out
                bg_stroke: Stroke::new(1.0, Color32::from_gray(70)),
                corner_radius: 4.0,

                fg_stroke: Stroke::new(1.0, Color32::from_gray(140)), // Should look grayed out
                expansion: 0.0,
            },
            inactive: WidgetVisuals {
                bg_fill: Color32::from_gray(70),
                bg_stroke: Default::default(),
                corner_radius: 4.0,

                fg_stroke: Stroke::new(1.0, Color32::from_gray(200)), // Should NOT look grayed out!
                expansion: 0.0,
            },
            hovered: WidgetVisuals {
                bg_fill: Color32::from_gray(80),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(150)), // e.g. hover over window edge or button
                corner_radius: 4.0,

                fg_stroke: Stroke::new(1.5, Color32::from_gray(240)),
                expansion: 1.0,
            },
            active: WidgetVisuals {
                bg_fill: Color32::from_gray(90),
                bg_stroke: Stroke::new(1.0, Color32::WHITE),
                corner_radius: 4.0,

                fg_stroke: Stroke::new(2.0, Color32::WHITE),
                expansion: 2.0,
            },
        }
    }
}

// ----------------------------------------------------------------------------

use crate::{widgets::*, Ui};

impl Style {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        crate::reset_button(ui, self);

        let Self {
            body_text_style,
            spacing,
            interaction,
            visuals,
            animation_time,
        } = self;
        ui.horizontal(|ui| {
            ui.label("Default text style:");
            for &value in &[TextStyle::Body, TextStyle::Monospace] {
                ui.radio_value(body_text_style, value, format!("{:?}", value));
            }
        });
        ui.collapsing("📏 Spacing", |ui| spacing.ui(ui));
        ui.collapsing("☝ Interaction", |ui| interaction.ui(ui));
        ui.collapsing("🎨 Visuals", |ui| visuals.ui(ui));
        ui.add(Slider::f32(animation_time, 0.0..=1.0).text("animation_time"));
    }
}

impl Spacing {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        crate::reset_button(ui, self);

        let Self {
            item_spacing,
            window_padding,
            button_padding,
            indent,
            interact_size,
            slider_width,
            text_edit_width,
            icon_width,
            icon_spacing,
            tooltip_width,
        } = self;

        ui_slider_vec2(ui, item_spacing, 0.0..=10.0, "item_spacing");
        ui_slider_vec2(ui, window_padding, 0.0..=10.0, "window_padding");
        ui_slider_vec2(ui, button_padding, 0.0..=10.0, "button_padding");
        ui_slider_vec2(ui, interact_size, 0.0..=60.0, "interact_size")
            .on_hover_text("Minimum size of an interactive widget");
        ui.add(Slider::f32(indent, 0.0..=100.0).text("indent"));
        ui.add(Slider::f32(slider_width, 0.0..=1000.0).text("slider_width"));
        ui.add(Slider::f32(text_edit_width, 0.0..=1000.0).text("text_edit_width"));
        ui.add(Slider::f32(icon_width, 0.0..=60.0).text("icon_width"));
        ui.add(Slider::f32(icon_spacing, 0.0..=10.0).text("icon_spacing"));
        ui.add(Slider::f32(tooltip_width, 0.0..=10.0).text("tooltip_width"));
    }
}

impl Interaction {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        crate::reset_button(ui, self);

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

impl Widgets {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        crate::reset_button(ui, self);

        let Self {
            active,
            hovered,
            inactive,
            disabled,
            noninteractive,
        } = self;

        ui.collapsing("noninteractive", |ui| {
            ui.label("The style of something that you cannot interact with.");
            noninteractive.ui(ui)
        });
        ui.collapsing("interactive & disabled", |ui| {
            ui.label("The style of a disabled button.");
            disabled.ui(ui)
        });
        ui.collapsing("interactive & inactive", |ui| {
            ui.label("The style of a widget, such as a button, at rest.");
            inactive.ui(ui)
        });
        ui.collapsing("interactive & hovered", |ui| {
            ui.label("The style of a widget while you hover it.");
            hovered.ui(ui)
        });
        ui.collapsing("interactive & active", |ui| {
            ui.label("The style of a widget as you are clicking or dragging it.");
            active.ui(ui)
        });
    }
}

impl Selection {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self { bg_fill, stroke } = self;

        ui_color(ui, bg_fill, "bg_fill");
        stroke_ui(ui, stroke, "stroke");
    }
}

impl WidgetVisuals {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            bg_fill,
            bg_stroke,
            corner_radius,
            fg_stroke,
            expansion,
        } = self;

        ui_color(ui, bg_fill, "bg_fill");
        stroke_ui(ui, bg_stroke, "bg_stroke");
        ui.add(Slider::f32(corner_radius, 0.0..=10.0).text("corner_radius"));
        stroke_ui(ui, fg_stroke, "fg_stroke (text)");
        ui.add(Slider::f32(expansion, -5.0..=5.0).text("expansion"));
    }
}

impl Visuals {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        crate::reset_button(ui, self);

        let Self {
            override_text_color: _,
            widgets,
            selection,
            dark_bg_color,
            hyperlink_color,
            window_corner_radius,
            window_shadow,
            resize_corner_size,
            text_cursor_width,
            clip_rect_margin,
            debug_expand_width,
            debug_expand_height,
            debug_resize,
        } = self;

        ui.collapsing("widgets", |ui| widgets.ui(ui));
        ui.collapsing("selection", |ui| selection.ui(ui));
        ui_color(ui, dark_bg_color, "dark_bg_color");
        ui_color(ui, hyperlink_color, "hyperlink_color");
        ui.add(Slider::f32(window_corner_radius, 0.0..=20.0).text("window_corner_radius"));
        shadow_ui(ui, window_shadow, "Window shadow:");
        ui.add(Slider::f32(resize_corner_size, 0.0..=20.0).text("resize_corner_size"));
        ui.add(Slider::f32(text_cursor_width, 0.0..=2.0).text("text_cursor_width"));
        ui.add(Slider::f32(clip_rect_margin, 0.0..=20.0).text("clip_rect_margin"));

        ui.label("DEBUG:");
        ui.checkbox(
            debug_expand_width,
            "Show which widgets make their parent wider",
        );
        ui.checkbox(
            debug_expand_height,
            "Show which widgets make their parent higher",
        );
        ui.checkbox(debug_resize, "Debug Resize");
    }
}

// TODO: improve and standardize ui_slider_vec2
fn ui_slider_vec2(
    ui: &mut Ui,
    value: &mut Vec2,
    range: std::ops::RangeInclusive<f32>,
    text: &str,
) -> Response {
    ui.horizontal(|ui| {
        /*
        let fsw = full slider_width
        let ssw = small slider_width
        let space = item_spacing.x
        let value = interact_size.x;

        fsw + space + value = ssw + space + value + space + ssw + space + value
        fsw + space + value = 2 * ssw + 3 * space + 2 * value
        fsw + space - value = 2 * ssw + 3 * space
        fsw - 2 * space - value = 2 * ssw
        ssw = fsw / 2 - space - value / 2
        */
        // let spacing = &ui.style().spacing;
        // let space = spacing.item_spacing.x;
        // let value_w = spacing.interact_size.x;
        // let full_slider_width = spacing.slider_width;
        // let small_slider_width = full_slider_width / 2.0 - space - value_w / 2.0;
        // ui.style_mut().spacing.slider_width = small_slider_width;

        ui.add(Slider::f32(&mut value.x, range.clone()).text("w"));
        ui.add(Slider::f32(&mut value.y, range.clone()).text("h"));
        ui.label(text);
    })
    .1
}

fn ui_color(ui: &mut Ui, srgba: &mut Color32, text: &str) {
    ui.horizontal(|ui| {
        ui.color_edit_button_srgba(srgba);
        ui.label(text);
    });
}
