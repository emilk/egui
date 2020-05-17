#![allow(clippy::if_same_then_else)]

use serde_derive::{Deserialize, Serialize};

use crate::{color::*, math::*, types::*};

// TODO: split into Spacing and Style?
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Style {
    /// Horizontal and vertical padding within a window frame.
    pub window_padding: Vec2,

    /// Button size is text size plus this on each side
    pub button_padding: Vec2,

    /// Horizontal and vertical spacing between widgets
    pub item_spacing: Vec2,

    /// Indent collapsing regions etc by this much.
    pub indent: f32,

    /// Anything clickable is (at least) this wide.
    pub clickable_diameter: f32,

    /// Checkboxes, radio button and collapsing headers have an icon at the start.
    /// The text starts after this many pixels.
    pub start_icon_width: f32,

    // -----------------------------------------------
    // Purely visual:
    pub interact: Interact,

    // TODO: an WidgetStyle ?
    pub text_color: Color,

    /// For stuff like check marks in check boxes.
    pub line_width: f32,

    pub thin_outline: Outline,

    /// e.g. the background of windows
    pub background_fill_color: Color,

    /// e.g. the background of the slider or text edit
    pub dark_bg_color: Color,

    pub cursor_blink_hz: f32,
    pub text_cursor_width: f32,

    // TODO: add ability to disable animations!
    /// How many seconds a typical animation should last
    pub animation_time: f32,

    pub window: Window,

    pub menu_bar: MenuBar,

    /// Allow child widgets to be just on the border and still have an outline with some thickness
    pub clip_rect_margin: f32,

    // -----------------------------------------------
    // Debug rendering:
    pub debug_widget_rects: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            window_padding: vec2(6.0, 6.0),
            button_padding: vec2(5.0, 3.0),
            item_spacing: vec2(8.0, 4.0),
            indent: 21.0,
            clickable_diameter: 22.0,
            start_icon_width: 14.0,
            interact: Default::default(),
            text_color: gray(160, 255),
            line_width: 1.0,
            thin_outline: Outline::new(0.5, GRAY),
            background_fill_color: gray(32, 250),
            dark_bg_color: gray(0, 140),
            cursor_blink_hz: 1.0,
            text_cursor_width: 2.0,
            animation_time: 1.0 / 15.0,
            window: Window::default(),
            menu_bar: MenuBar::default(),
            clip_rect_margin: 3.0,
            debug_widget_rects: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Interact {
    pub active: WidgetStyle,
    pub hovered: WidgetStyle,
    pub inactive: WidgetStyle,
}

impl Default for Interact {
    fn default() -> Self {
        Self {
            active: WidgetStyle {
                bg_fill_color: Some(gray(0, 128)),
                fill_color: srgba(120, 120, 200, 255),
                stroke_color: WHITE,
                stroke_width: 2.0,
                rect_outline: Some(Outline::new(2.0, WHITE)),
                corner_radius: 5.0,
            },
            hovered: WidgetStyle {
                bg_fill_color: None,
                fill_color: srgba(100, 100, 150, 255),
                stroke_color: gray(240, 255),
                stroke_width: 1.5,
                rect_outline: Some(Outline::new(1.0, WHITE)),
                corner_radius: 5.0,
            },
            inactive: WidgetStyle {
                bg_fill_color: None,
                fill_color: srgba(60, 60, 80, 255),
                stroke_color: gray(210, 255), // Mustn't look grayed out!
                stroke_width: 1.0,
                rect_outline: Some(Outline::new(0.5, WHITE)),
                corner_radius: 0.0,
            },
        }
    }
}

impl Interact {
    pub fn style(&self, interact: &InteractInfo) -> &WidgetStyle {
        if interact.active {
            &self.active
        } else if interact.hovered {
            &self.hovered
        } else {
            &self.inactive
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct WidgetStyle {
    /// Background color of widget
    pub bg_fill_color: Option<Color>,

    /// Fill color of the interactive part of a component (slider grab, checkbox, ...)
    /// When you need a fill_color.
    pub fill_color: Color,

    /// Stroke and text color of the interactive part of a component (button, slider grab, checkbox, ...)
    pub stroke_color: Color,

    /// For lines etc
    pub stroke_width: f32,

    /// For surrounding rectangle of things that need it,
    /// like buttons, the box of the checkbox, etc.
    pub rect_outline: Option<Outline>,

    /// Button frames etdc
    pub corner_radius: f32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Window {
    pub corner_radius: f32,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            corner_radius: 10.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct MenuBar {
    pub height: f32,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self { height: 16.0 }
    }
}

impl Style {
    /// Use this style for interactive things
    pub fn interact(&self, interact: &InteractInfo) -> &WidgetStyle {
        self.interact.style(interact)
    }

    /// Returns small icon rectangle and big icon rectangle
    pub fn icon_rectangles(&self, rect: Rect) -> (Rect, Rect) {
        let box_side = self.start_icon_width;
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

impl Style {
    #[rustfmt::skip]
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        use crate::{widgets::*};
        if ui.add(Button::new("Reset style")).clicked {
            *self = Default::default();
        }

        ui.add(Checkbox::new(&mut self.debug_widget_rects, "Paint debug rectangles around widgets"));

        ui.add(Slider::f32(&mut self.item_spacing.x,     0.0..=10.0).text("item_spacing.x").precision(0));
        ui.add(Slider::f32(&mut self.item_spacing.y,     0.0..=10.0).text("item_spacing.y").precision(0));
        ui.add(Slider::f32(&mut self.window_padding.x,   0.0..=10.0).text("window_padding.x").precision(0));
        ui.add(Slider::f32(&mut self.window_padding.y,   0.0..=10.0).text("window_padding.y").precision(0));
        ui.add(Slider::f32(&mut self.indent,             0.0..=100.0).text("indent").precision(0));
        ui.add(Slider::f32(&mut self.button_padding.x,   0.0..=20.0).text("button_padding.x").precision(0));
        ui.add(Slider::f32(&mut self.button_padding.y,   0.0..=20.0).text("button_padding.y").precision(0));
        ui.add(Slider::f32(&mut self.clickable_diameter, 0.0..=60.0).text("clickable_diameter").precision(0));
        ui.add(Slider::f32(&mut self.start_icon_width,   0.0..=60.0).text("start_icon_width").precision(0));
        ui.add(Slider::f32(&mut self.line_width,         0.0..=10.0).text("line_width").precision(1));
        ui.add(Slider::f32(&mut self.animation_time,     0.0..=1.0).text("animation_time").precision(2));
    }
}
