//! Widgets are pieces of GUI such as labels, buttons, sliders etc.
//!
//! Example widget uses:
//! * `ui.add(Label::new("Text").text_color(color::red));`//!
//! * `if ui.add(Button::new("Click me")).clicked { ... }`

#![allow(clippy::new_without_default)]

use crate::{layout::Direction, *};

pub mod color_picker;
mod drag_value;
mod image;
mod slider;
pub(crate) mod text_edit;

pub use {drag_value::DragValue, image::Image, slider::*, text_edit::*};

use paint::*;

// ----------------------------------------------------------------------------

/// Anything implementing Widget can be added to a Ui with `Ui::add`
pub trait Widget {
    fn ui(self, ui: &mut Ui) -> Response;
}

// ----------------------------------------------------------------------------

/// Static text.
pub struct Label {
    // TODO: not pub
    pub(crate) text: String,
    pub(crate) multiline: bool,
    pub(crate) text_style: Option<TextStyle>,
    pub(crate) text_color: Option<Srgba>,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            multiline: true,
            text_style: None,
            text_color: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    /// If you do not set a `TextStyle`, the default `style.text_style`.
    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }

    pub fn heading(self) -> Self {
        self.text_style(TextStyle::Heading)
    }

    pub fn text_color(mut self, text_color: Srgba) -> Self {
        self.text_color = Some(text_color);
        self
    }

    pub fn layout(&self, ui: &Ui) -> font::Galley {
        let max_width = ui.available().width();
        // Prevent word-wrapping after a single letter, and other silly shit:
        // TODO: general "don't force labels and similar to wrap so early"
        // TODO: max_width = max_width.at_least(ui.spacing.first_wrap_width);
        self.layout_width(ui, max_width)
    }

    pub fn layout_width(&self, ui: &Ui, max_width: f32) -> font::Galley {
        let text_style = self.text_style_or_default(ui.style());
        let font = &ui.fonts()[text_style];
        if self.multiline {
            font.layout_multiline(self.text.clone(), max_width) // TODO: avoid clone
        } else {
            font.layout_single_line(self.text.clone()) // TODO: avoid clone
        }
    }

    pub fn font_height(&self, fonts: &Fonts, style: &Style) -> f32 {
        let text_style = self.text_style_or_default(style);
        fonts[text_style].height()
    }

    // TODO: this should return a LabelLayout which has a paint method.
    // We can then split Widget::Ui in two: layout + allocating space, and painting.
    // this allows us to assemble labels, THEN detect interaction, THEN chose color style based on that.
    // pub fn layout(self, ui: &mut ui) -> LabelLayout { }

    // TODO: a paint method for painting anywhere in a ui.
    // This should be the easiest method of putting text anywhere.

    pub fn paint_galley(&self, ui: &mut Ui, pos: Pos2, galley: font::Galley) {
        let text_style = self.text_style_or_default(ui.style());
        let text_color = self
            .text_color
            .unwrap_or_else(|| ui.style().visuals.text_color());
        ui.painter().galley(pos, galley, text_style, text_color);
    }

    /// Read the text style, or get the default for the current style
    pub fn text_style_or_default(&self, style: &Style) -> TextStyle {
        self.text_style.unwrap_or_else(|| style.body_text_style)
    }
}

/// Shortcut for creating a `Label` widget.
///
/// Usage: `label!("Foo: {}", bar)` equivalent to `Label::new(format!("Foo: {}", bar))`.
#[macro_export]
macro_rules! label {
    ($fmt:expr) => ($crate::widgets::Label::new($fmt));
    ($fmt:expr, $($arg:tt)*) => ($crate::widgets::Label::new(format!($fmt, $($arg)*)));
}

impl Widget for Label {
    fn ui(self, ui: &mut Ui) -> Response {
        let galley = self.layout(ui);
        let rect = ui.allocate_space(galley.size);
        self.paint_galley(ui, rect.min, galley);
        ui.interact_hover(rect)
    }
}

impl Into<Label> for &str {
    fn into(self) -> Label {
        Label::new(self)
    }
}

impl Into<Label> for &String {
    fn into(self) -> Label {
        Label::new(self)
    }
}

impl Into<Label> for String {
    fn into(self) -> Label {
        Label::new(self)
    }
}

// ----------------------------------------------------------------------------

/// A clickable hyperlink, e.g. to `"https://github.com/emilk/egui"`.
pub struct Hyperlink {
    url: String,
    text: String,
}

impl Hyperlink {
    pub fn new(url: impl Into<String>) -> Self {
        let url = url.into();
        Self {
            text: url.clone(),
            url,
        }
    }

    /// Show some other text than the url
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }
}

impl Widget for Hyperlink {
    fn ui(self, ui: &mut Ui) -> Response {
        let Hyperlink { url, text } = self;

        let color = color::LIGHT_BLUE;
        let text_style = ui.style().body_text_style;
        let id = ui.make_child_id(&url);
        let font = &ui.fonts()[text_style];
        let galley = font.layout_multiline(text, ui.available().width());
        let rect = ui.allocate_space(galley.size);
        let response = ui.interact(rect, id, Sense::click());
        if response.hovered {
            ui.ctx().output().cursor_icon = CursorIcon::PointingHand;
        }
        if response.clicked {
            ui.ctx().output().open_url = Some(url.clone());
        }

        let style = ui.style().interact(&response);

        if response.hovered {
            // Underline:
            for line in &galley.lines {
                let pos = response.rect.min;
                let y = pos.y + line.y_max;
                let y = ui.painter().round_to_pixel(y);
                let min_x = pos.x + line.min_x();
                let max_x = pos.x + line.max_x();
                ui.painter().line_segment(
                    [pos2(min_x, y), pos2(max_x, y)],
                    (style.fg_stroke.width, color),
                );
            }
        }

        ui.painter()
            .galley(response.rect.min, galley, text_style, color);

        response.tooltip_text(url)
    }
}

// ----------------------------------------------------------------------------

/// Clickable button with text
pub struct Button {
    text: String,
    text_color: Option<Srgba>,
    text_style: TextStyle,
    /// None means default for interact
    fill: Option<Srgba>,
    sense: Sense,
}

impl Button {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            text_color: None,
            text_style: TextStyle::Button,
            fill: Default::default(),
            sense: Sense::click(),
        }
    }

    pub fn text_color(mut self, text_color: Srgba) -> Self {
        self.text_color = Some(text_color);
        self
    }

    pub fn text_color_opt(mut self, text_color: Option<Srgba>) -> Self {
        self.text_color = text_color;
        self
    }

    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = text_style;
        self
    }

    pub fn fill(mut self, fill: Option<Srgba>) -> Self {
        self.fill = fill;
        self
    }

    /// By default, buttons senses clicks.
    /// Change this to a drag-button with `Sense::drag()`.
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// If you set this to `false`, the button will be grayed out and un-clickable.
    /// `enabled(false)` has the same effect as calling `sense(Sense::nothing())`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        if !enabled {
            self.sense = Sense::nothing();
        }
        self
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let Button {
            text,
            text_color,
            text_style,
            fill,
            sense,
        } = self;

        let id = ui.make_position_id();
        let font = &ui.fonts()[text_style];
        let galley = font.layout_multiline(text, ui.available().width());
        let mut desired_size = galley.size + 2.0 * ui.style().spacing.button_padding;
        desired_size = desired_size.at_least(ui.style().spacing.interact_size);
        let rect = ui.allocate_space(desired_size);

        let response = ui.interact(rect, id, sense);
        let style = ui.style().interact(&response);
        let text_cursor = response.rect.center() - 0.5 * galley.size;
        let fill = fill.unwrap_or(style.bg_fill);
        ui.painter().add(PaintCmd::Rect {
            rect: response.rect,
            corner_radius: style.corner_radius,
            fill,
            stroke: style.bg_stroke,
        });
        let text_color = text_color.unwrap_or_else(|| style.text_color());
        ui.painter()
            .galley(text_cursor, galley, text_style, text_color);
        response
    }
}

// ----------------------------------------------------------------------------

// TODO: allow checkbox without a text label
/// Boolean on/off control with text label
#[derive(Debug)]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    text: String,
    text_color: Option<Srgba>,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, text: impl Into<String>) -> Self {
        Checkbox {
            checked,
            text: text.into(),
            text_color: None,
        }
    }

    pub fn text_color(mut self, text_color: Srgba) -> Self {
        self.text_color = Some(text_color);
        self
    }
}

impl<'a> Widget for Checkbox<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Checkbox {
            checked,
            text,
            text_color,
        } = self;

        let id = ui.make_position_id();
        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];
        let galley = font.layout_single_line(text);

        let spacing = &ui.style().spacing;
        let icon_width = spacing.icon_width;
        let icon_spacing = ui.style().spacing.icon_spacing;
        let button_padding = spacing.button_padding;
        let mut desired_size =
            button_padding + vec2(icon_width + icon_spacing, 0.0) + galley.size + button_padding;
        desired_size = desired_size.at_least(spacing.interact_size);
        let rect = ui.allocate_space(desired_size);

        let response = ui.interact(rect, id, Sense::click());
        if response.clicked {
            *checked = !*checked;
        }

        let visuals = ui.style().interact(&response);
        let text_cursor = pos2(
            response.rect.min.x + button_padding.x + icon_width + icon_spacing,
            response.rect.center().y - 0.5 * galley.size.y,
        );
        let (small_icon_rect, big_icon_rect) = ui.style().spacing.icon_rectangles(response.rect);
        ui.painter().add(PaintCmd::Rect {
            rect: big_icon_rect,
            corner_radius: visuals.corner_radius,
            fill: visuals.bg_fill,
            stroke: visuals.bg_stroke,
        });

        if *checked {
            ui.painter().add(PaintCmd::Path {
                points: vec![
                    pos2(small_icon_rect.left(), small_icon_rect.center().y),
                    pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                    pos2(small_icon_rect.right(), small_icon_rect.top()),
                ],
                closed: false,
                fill: Default::default(),
                stroke: visuals.fg_stroke,
            });
        }

        let text_color = text_color.unwrap_or_else(|| visuals.text_color());
        ui.painter()
            .galley(text_cursor, galley, text_style, text_color);
        response
    }
}

// ----------------------------------------------------------------------------

/// One out of several alternatives, either checked or not.
#[derive(Debug)]
pub struct RadioButton {
    checked: bool,
    text: String,
    text_color: Option<Srgba>,
}

impl RadioButton {
    pub fn new(checked: bool, text: impl Into<String>) -> Self {
        Self {
            checked,
            text: text.into(),
            text_color: None,
        }
    }

    pub fn text_color(mut self, text_color: Srgba) -> Self {
        self.text_color = Some(text_color);
        self
    }
}

impl Widget for RadioButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let RadioButton {
            checked,
            text,
            text_color,
        } = self;
        let id = ui.make_position_id();
        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];
        let galley = font.layout_multiline(text, ui.available().width());

        let icon_width = ui.style().spacing.icon_width;
        let icon_spacing = ui.style().spacing.icon_spacing;
        let button_padding = ui.style().spacing.button_padding;
        let mut desired_size =
            button_padding + vec2(icon_width + icon_spacing, 0.0) + galley.size + button_padding;
        desired_size = desired_size.at_least(ui.style().spacing.interact_size);
        let rect = ui.allocate_space(desired_size);

        let response = ui.interact(rect, id, Sense::click());

        let text_cursor = pos2(
            response.rect.min.x + button_padding.x + icon_width + icon_spacing,
            response.rect.center().y - 0.5 * galley.size.y,
        );

        let visuals = ui.style().interact(&response);

        let (small_icon_rect, big_icon_rect) = ui.style().spacing.icon_rectangles(response.rect);

        let painter = ui.painter();

        painter.add(PaintCmd::Circle {
            center: big_icon_rect.center(),
            radius: big_icon_rect.width() / 2.0,
            fill: visuals.bg_fill,
            stroke: visuals.bg_stroke,
        });

        if checked {
            painter.add(PaintCmd::Circle {
                center: small_icon_rect.center(),
                radius: small_icon_rect.width() / 3.0,
                fill: visuals.fg_stroke.color, // Intentional to use stroke and not fill
                stroke: Default::default(),
                // fill: visuals.fg_fill,
                // stroke: visuals.fg_stroke,
            });
        }

        let text_color = text_color.unwrap_or_else(|| visuals.text_color());
        painter.galley(text_cursor, galley, text_style, text_color);
        response
    }
}

// ----------------------------------------------------------------------------

/// A visual separator. A horizontal or vertical line (depending on `Layout`).
pub struct Separator {
    spacing: f32,
}

impl Separator {
    pub fn new() -> Self {
        Self { spacing: 6.0 }
    }

    /// How much space we take up. The line is painted in the middle of this.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui) -> Response {
        let Separator { spacing } = self;

        let available_space = ui.available_finite().size();

        let (points, rect) = match ui.layout().dir() {
            Direction::Horizontal => {
                let rect = ui.allocate_space(vec2(spacing, available_space.y));
                (
                    [
                        pos2(rect.center().x, rect.top()),
                        pos2(rect.center().x, rect.bottom()),
                    ],
                    rect,
                )
            }
            Direction::Vertical => {
                let rect = ui.allocate_space(vec2(available_space.x, spacing));
                (
                    [
                        pos2(rect.left(), rect.center().y),
                        pos2(rect.right(), rect.center().y),
                    ],
                    rect,
                )
            }
        };
        let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
        ui.painter().line_segment(points, stroke);
        ui.interact_hover(rect)
    }
}
