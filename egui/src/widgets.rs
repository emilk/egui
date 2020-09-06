//! Widgets are pieces of GUI such as labels, buttons, sliders etc.
//!
//! Example widget uses:
//! * `ui.add(Label::new("Text").text_color(color::red));`//!
//! * `if ui.add(Button::new("Click me")).clicked { ... }`

#![allow(clippy::new_without_default)]

use crate::{layout::Direction, *};

mod slider;
pub(crate) mod text_edit;

pub use {slider::*, text_edit::*};

use paint::*;

use std::ops::RangeInclusive;

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
    auto_shrink: bool,
    pub(crate) text_style: Option<TextStyle>,
    pub(crate) text_color: Option<Srgba>,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            multiline: true,
            auto_shrink: false,
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

    /// If true, will word wrap to `ui.available_finite().width()`.
    /// If false (default), will word wrap to `ui.available().width()`.
    /// This only makes a difference for auto-sized parents.
    pub fn auto_shrink(mut self) -> Self {
        self.auto_shrink = true;
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
        let max_width = if self.auto_shrink {
            ui.available_finite().width()
        } else {
            ui.available().width()
        };
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
            .unwrap_or_else(|| ui.style().visuals.text_color);
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
            ui.ctx().output().open_url = Some(url);
        }

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
                    (ui.style().visuals.line_width, color),
                );
            }
        }

        ui.painter()
            .galley(response.rect.min, galley, text_style, color);

        response
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
        desired_size.y = desired_size.y.max(ui.style().spacing.clickable_diameter);
        let rect = ui.allocate_space(desired_size);
        let rect = rect.expand2(ui.style().spacing.button_expand);

        let response = ui.interact(rect, id, sense);
        let text_cursor = response.rect.center() - 0.5 * galley.size;
        let fill = fill.unwrap_or(ui.style().interact(&response).bg_fill);
        ui.painter().add(PaintCmd::Rect {
            rect: response.rect,
            corner_radius: ui.style().interact(&response).corner_radius,
            fill,
            stroke: ui.style().interact(&response).bg_stroke,
        });
        let text_color = text_color.unwrap_or_else(|| ui.style().interact(&response).text_color());
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

        let icon_width = ui.style().spacing.icon_width;
        let button_padding = ui.style().spacing.button_padding;
        let mut desired_size =
            button_padding + vec2(icon_width, 0.0) + galley.size + button_padding;
        desired_size.y = desired_size.y.max(ui.style().spacing.clickable_diameter);
        let rect = ui.allocate_space(desired_size);

        let response = ui.interact(rect, id, Sense::click());
        if response.clicked {
            *checked = !*checked;
        }

        let visuals = ui.style().interact(&response);
        let text_cursor = pos2(
            response.rect.min.x + button_padding.x + icon_width,
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
        let button_padding = ui.style().spacing.button_padding;
        let mut desired_size =
            button_padding + vec2(icon_width, 0.0) + galley.size + button_padding;
        desired_size.y = desired_size.y.max(ui.style().spacing.clickable_diameter);
        let rect = ui.allocate_space(desired_size);

        let response = ui.interact(rect, id, Sense::click());

        let text_cursor = pos2(
            response.rect.min.x + button_padding.x + icon_width,
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
    line_width: Option<f32>,
    spacing: f32,
    extra: f32,
    color: Srgba,
}

impl Separator {
    pub fn new() -> Self {
        Self {
            line_width: None,
            spacing: 6.0,
            extra: 0.0,
            color: Srgba::gray(128), // TODO: from style
        }
    }

    pub fn line_width(mut self, line_width: f32) -> Self {
        self.line_width = Some(line_width);
        self
    }

    /// How much space we take up. The line is painted in the middle of this.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Draw this much longer on each side
    pub fn extra(mut self, extra: f32) -> Self {
        self.extra = extra;
        self
    }

    pub fn color(mut self, color: Srgba) -> Self {
        self.color = color;
        self
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui) -> Response {
        let Separator {
            line_width,
            spacing,
            extra,
            color,
        } = self;

        let line_width = line_width.unwrap_or_else(|| ui.style().visuals.line_width);

        let available_space = ui.available_finite().size();

        let (points, rect) = match ui.layout().dir() {
            Direction::Horizontal => {
                let rect = ui.allocate_space(vec2(spacing, available_space.y));
                (
                    [
                        pos2(rect.center().x, rect.top() - extra),
                        pos2(rect.center().x, rect.bottom() + extra),
                    ],
                    rect,
                )
            }
            Direction::Vertical => {
                let rect = ui.allocate_space(vec2(available_space.x, spacing));
                (
                    [
                        pos2(rect.left() - extra, rect.center().y),
                        pos2(rect.right() + extra, rect.center().y),
                    ],
                    rect,
                )
            }
        };
        ui.painter().line_segment(points, (line_width, color));
        ui.interact_hover(rect)
    }
}

// ----------------------------------------------------------------------------

/// Combined into one function (rather than two) to make it easier
/// for the borrow checker.
type GetSetValue<'a> = Box<dyn 'a + FnMut(Option<f64>) -> f64>;

fn get(value_function: &mut GetSetValue<'_>) -> f64 {
    (value_function)(None)
}

fn set(value_function: &mut GetSetValue<'_>, value: f64) {
    (value_function)(Some(value));
}

/// A floating point value that you can change by dragging the number. More compact than a slider.
pub struct DragValue<'a> {
    value_function: GetSetValue<'a>,
    speed: f32,
    prefix: String,
    suffix: String,
    range: RangeInclusive<f64>,
}

impl<'a> DragValue<'a> {
    fn from_get_set(value_function: impl 'a + FnMut(Option<f64>) -> f64) -> Self {
        Self {
            value_function: Box::new(value_function),
            speed: 1.0,
            prefix: Default::default(),
            suffix: Default::default(),
            range: f64::NEG_INFINITY..=f64::INFINITY,
        }
    }

    pub fn f32(value: &'a mut f32) -> Self {
        Self {
            ..Self::from_get_set(move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v as f32
                }
                *value as f64
            })
        }
    }

    pub fn u8(value: &'a mut u8) -> Self {
        Self {
            ..Self::from_get_set(move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v.round() as u8;
                }
                *value as f64
            })
        }
    }

    pub fn i32(value: &'a mut i32) -> Self {
        Self {
            ..Self::from_get_set(move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v.round() as i32;
                }
                *value as f64
            })
        }
    }

    /// How much the value changes when dragged one point (logical pixel).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Clamp the value to this range
    pub fn range(mut self, range: RangeInclusive<f64>) -> Self {
        self.range = range;
        self
    }

    /// Show a prefix before the number, e.g. "x: "
    pub fn prefix(mut self, prefix: impl ToString) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    /// Add a suffix to the number, this can be e.g. a unit ("°" or " m")
    pub fn suffix(mut self, suffix: impl ToString) -> Self {
        self.suffix = suffix.to_string();
        self
    }
}

impl<'a> Widget for DragValue<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            mut value_function,
            speed,
            range,
            prefix,
            suffix,
        } = self;
        let value = get(&mut value_function);
        let aim_rad = ui.input().physical_pixel_size(); // ui.input().aim_radius(); // TODO
        let precision = (aim_rad / speed.abs()).log10().ceil().max(0.0) as usize;
        let value_text = format_with_minimum_precision(value as f32, precision); //  TODO: full precision

        let kb_edit_id = ui.make_position_id().with("edit");
        let is_kb_editing = ui.memory().has_kb_focus(kb_edit_id);

        if is_kb_editing {
            let mut value_text = ui
                .memory()
                .temp_edit_string
                .take()
                .unwrap_or_else(|| value_text);
            let response = ui.add(
                TextEdit::new(&mut value_text)
                    .id(kb_edit_id)
                    .multiline(false)
                    .desired_width(0.0)
                    .text_style(TextStyle::Monospace),
            );
            if let Ok(parsed_value) = value_text.parse() {
                let parsed_value = clamp(parsed_value, range);
                set(&mut value_function, parsed_value)
            }
            if ui.input().key_pressed(Key::Enter) {
                ui.memory().surrender_kb_focus(kb_edit_id);
            } else {
                ui.memory().temp_edit_string = Some(value_text);
            }
            response
        } else {
            let button = Button::new(format!("{}{}{}", prefix, value_text, suffix))
                .sense(Sense::click_and_drag())
                .text_style(TextStyle::Monospace);
            let response = ui.add(button);
            // response.tooltip_text("Drag to edit, click to enter a value"); // TODO: may clash with users own tooltips
            if response.clicked {
                ui.memory().request_kb_focus(kb_edit_id);
                ui.memory().temp_edit_string = None; // Filled in next frame
            } else if response.active {
                let mdelta = ui.input().mouse.delta;
                let delta_points = mdelta.x - mdelta.y; // Increase to the right and up
                let delta_value = speed * delta_points;
                if delta_value != 0.0 {
                    let new_value = value + delta_value as f64;
                    let new_value = round_to_precision(new_value, precision);
                    let new_value = clamp(new_value, range);
                    set(&mut value_function, new_value);
                    // TODO: To make use or `smart_aim` for `DragValue` we need to store some state somewhere,
                    // otherwise we will just keep rounding to the same value while moving the mouse.
                }
            }
            response
        }
    }
}
