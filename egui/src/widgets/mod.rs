//! Widgets are pieces of GUI such as [`Label`], [`Button`], [`Slider`] etc.
//!
//! Example widget uses:
//! * `ui.add(Label::new("Text").text_color(color::red));`
//! * `if ui.add(Button::new("Click me")).clicked { ... }`

#![allow(clippy::new_without_default)]

use crate::*;

mod button;
pub mod color_picker;
mod drag_value;
mod image;
mod slider;
pub(crate) mod text_edit;

pub use {button::*, drag_value::DragValue, image::Image, slider::*, text_edit::*};

use paint::*;

// ----------------------------------------------------------------------------

/// Anything implementing Widget can be added to a [`Ui`] with [`Ui::add`].
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub trait Widget {
    /// Allocate space, interact, paint, and return a [`Response`].
    fn ui(self, ui: &mut Ui) -> Response;
}

// ----------------------------------------------------------------------------

/// Static text.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Label {
    // TODO: not pub
    pub(crate) text: String,
    pub(crate) multiline: Option<bool>,
    pub(crate) text_style: Option<TextStyle>,
    pub(crate) text_color: Option<Srgba>,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            multiline: None,
            text_style: None,
            text_color: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// If `true`, the text will wrap at the `max_width`.
    /// By default `multiline` will be true in vertical layouts
    /// and horizontal layouts with wrapping,
    /// and false on non-wrapping horizontal layouts.
    ///
    /// If the text has any newlines (`\n`) in it, multiline will automatically turn on.
    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = Some(multiline);
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

    pub fn monospace(self) -> Self {
        self.text_style(TextStyle::Monospace)
    }

    pub fn small(self) -> Self {
        self.text_style(TextStyle::Small)
    }

    pub fn text_color(mut self, text_color: impl Into<Srgba>) -> Self {
        self.text_color = Some(text_color.into());
        self
    }

    pub fn layout(&self, ui: &Ui) -> Galley {
        let max_width = ui.available_width();
        self.layout_width(ui, max_width)
    }

    pub fn layout_width(&self, ui: &Ui, max_width: f32) -> Galley {
        let text_style = self.text_style_or_default(ui.style());
        let font = &ui.fonts()[text_style];
        if self.is_multiline(ui) {
            font.layout_multiline(self.text.clone(), max_width) // TODO: avoid clone
        } else {
            font.layout_single_line(self.text.clone()) // TODO: avoid clone
        }
    }

    pub fn font_height(&self, fonts: &Fonts, style: &Style) -> f32 {
        let text_style = self.text_style_or_default(style);
        fonts[text_style].row_height()
    }

    // TODO: this should return a LabelLayout which has a paint method.
    // We can then split Widget::Ui in two: layout + allocating space, and painting.
    // this allows us to assemble labels, THEN detect interaction, THEN chose color style based on that.
    // pub fn layout(self, ui: &mut ui) -> LabelLayout { }

    // TODO: a paint method for painting anywhere in a ui.
    // This should be the easiest method of putting text anywhere.

    pub fn paint_galley(&self, ui: &mut Ui, pos: Pos2, galley: Galley) {
        let text_style = self.text_style_or_default(ui.style());
        let text_color = self
            .text_color
            .unwrap_or_else(|| ui.style().visuals.text_color());
        ui.painter().galley(pos, galley, text_style, text_color);
    }

    /// Read the text style, or get the default for the current style
    pub fn text_style_or_default(&self, style: &Style) -> TextStyle {
        self.text_style.unwrap_or(style.body_text_style)
    }

    fn is_multiline(&self, ui: &Ui) -> bool {
        self.multiline.unwrap_or_else(|| {
            let layout = ui.layout();
            layout.is_vertical()
                || layout.is_horizontal() && layout.main_wrap()
                || self.text.contains('\n')
        })
    }
}

impl Widget for Label {
    fn ui(self, ui: &mut Ui) -> Response {
        if self.is_multiline(ui)
            && ui.layout().main_dir() == Direction::LeftToRight
            && ui.layout().main_wrap()
        {
            // On a wrapping horizontal layout we want text to start after the last widget,
            // then continue on the line below! This will take some extra work:

            let max_width = ui.available_width();
            let first_row_indentation = max_width - ui.available_size_before_wrap().x;

            let text_style = self.text_style_or_default(ui.style());
            let font = &ui.fonts()[text_style];
            let mut galley = font.layout_multiline_with_indentation_and_max_width(
                self.text.clone(),
                first_row_indentation,
                max_width,
            );

            let pos = pos2(ui.min_rect().left(), ui.cursor().y);

            assert!(!galley.rows.is_empty(), "Galleys are never empty");
            let rect = galley.rows[0].rect().translate(vec2(pos.x, pos.y));
            let id = ui.advance_cursor_after_rect(rect);
            let mut total_response = ui.interact(rect, id, Sense::hover());

            let mut y_translation = 0.0;
            if let Some(row) = galley.rows.get(1) {
                // We could be sharing the first row with e.g. a button, that is higher than text.
                // So we need to compensate for that:
                if pos.y + row.y_min < ui.min_rect().bottom() {
                    y_translation = ui.min_rect().bottom() - row.y_min - pos.y;
                }
            }

            for row in galley.rows.iter_mut().skip(1) {
                row.y_min += y_translation;
                row.y_max += y_translation;
                let rect = row.rect().translate(vec2(pos.x, pos.y));
                ui.advance_cursor_after_rect(rect);
                total_response |= ui.interact(rect, id, Sense::hover());
            }

            self.paint_galley(ui, pos, galley);
            total_response
        } else {
            let galley = self.layout(ui);
            let response = ui.allocate_response(galley.size, Sense::click());
            let rect = ui
                .layout()
                .align_size_within_rect(galley.size, response.rect);
            self.paint_galley(ui, rect.min, galley);
            response
        }
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
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Hyperlink {
    // TODO: wrap Label
    url: String,
    text: String,
    pub(crate) text_style: Option<TextStyle>,
}

impl Hyperlink {
    pub fn new(url: impl Into<String>) -> Self {
        let url = url.into();
        Self {
            text: url.clone(),
            url,
            text_style: None,
        }
    }

    /// Show some other text than the url
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// If you do not set a `TextStyle`, the default `style.text_style`.
    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }

    pub fn small(self) -> Self {
        self.text_style(TextStyle::Small)
    }
}

impl Widget for Hyperlink {
    fn ui(self, ui: &mut Ui) -> Response {
        let Hyperlink {
            url,
            text,
            text_style,
        } = self;
        let text_style = text_style.unwrap_or_else(|| ui.style().body_text_style);
        let font = &ui.fonts()[text_style];
        let galley = font.layout_multiline(text, ui.available_width());
        let response = ui.allocate_response(galley.size, Sense::click());

        if response.hovered {
            ui.ctx().output().cursor_icon = CursorIcon::PointingHand;
        }
        if response.clicked {
            ui.ctx().output().open_url = Some(url.clone());
        }

        let color = ui.style().visuals.hyperlink_color;
        let visuals = ui.style().interact(&response);

        if response.hovered {
            // Underline:
            for line in &galley.rows {
                let pos = response.rect.min;
                let y = pos.y + line.y_max;
                let y = ui.painter().round_to_pixel(y);
                let min_x = pos.x + line.min_x();
                let max_x = pos.x + line.max_x();
                ui.painter().line_segment(
                    [pos2(min_x, y), pos2(max_x, y)],
                    (visuals.fg_stroke.width, color),
                );
            }
        }

        ui.painter()
            .galley(response.rect.min, galley, text_style, color);

        response.on_hover_text(url)
    }
}

// ----------------------------------------------------------------------------

/// Clickable button with text.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Button {
    text: String,
    text_color: Option<Srgba>,
    text_style: TextStyle,
    /// None means default for interact
    fill: Option<Srgba>,
    sense: Sense,
    small: bool,
    frame: bool,
}

impl Button {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            text_color: None,
            text_style: TextStyle::Button,
            fill: Default::default(),
            sense: Sense::click(),
            small: false,
            frame: true,
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

    /// Make this a small button, suitable for embedding into text.
    pub fn small(mut self) -> Self {
        self.text_style = TextStyle::Body;
        self.small = true;
        self
    }

    /// Turn off the frame
    pub fn frame(mut self, frame: bool) -> Self {
        self.frame = frame;
        self
    }

    /// By default, buttons senses clicks.
    /// Change this to a drag-button with `Sense::drag()`.
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// If you set this to `false`, the button will be grayed out and un-clickable.
    /// `enabled(false)` has the same effect as calling `sense(Sense::hover())`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        if !enabled {
            self.sense = Sense::hover();
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
            small,
            frame,
        } = self;
        let font = &ui.fonts()[text_style];

        let single_line = ui.layout().is_horizontal();
        let galley = if single_line {
            font.layout_single_line(text)
        } else {
            font.layout_multiline(text, ui.available_width())
        };

        let mut button_padding = ui.style().spacing.button_padding;
        if small {
            button_padding.y = 0.0;
        }

        let mut desired_size = galley.size + 2.0 * button_padding;
        if !small {
            desired_size.y = desired_size.y.at_least(ui.style().spacing.interact_size.y);
        }

        let response = ui.allocate_response(desired_size, sense);

        if ui.clip_rect().intersects(response.rect) {
            let visuals = ui.style().interact(&response);
            let text_cursor = ui
                .layout()
                .align_size_within_rect(galley.size, response.rect.shrink2(button_padding))
                .min;

            if frame {
                let fill = fill.unwrap_or(visuals.bg_fill);
                ui.painter().rect(
                    response.rect,
                    visuals.corner_radius,
                    fill,
                    visuals.bg_stroke,
                );
            }

            let text_color = text_color
                .or(ui.style().visuals.override_text_color)
                .unwrap_or_else(|| visuals.text_color());
            ui.painter()
                .galley(text_cursor, galley, text_style, text_color);
        }

        response
    }
}

// ----------------------------------------------------------------------------

// TODO: allow checkbox without a text label
/// Boolean on/off control with text label.
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

        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];

        let spacing = &ui.style().spacing;
        let icon_width = spacing.icon_width;
        let icon_spacing = ui.style().spacing.icon_spacing;
        let button_padding = spacing.button_padding;
        let total_extra = button_padding + vec2(icon_width + icon_spacing, 0.0) + button_padding;

        let single_line = ui.layout().is_horizontal();
        let galley = if single_line {
            font.layout_single_line(text)
        } else {
            font.layout_multiline(text, ui.available_width() - total_extra.x)
        };

        let mut desired_size = total_extra + galley.size;
        desired_size = desired_size.at_least(spacing.interact_size);
        desired_size.y = desired_size.y.max(icon_width);
        let response = ui.allocate_response(desired_size, Sense::click());
        let rect = ui
            .layout()
            .align_size_within_rect(desired_size, response.rect);
        if response.clicked {
            *checked = !*checked;
        }

        let visuals = ui.style().interact(&response);
        let text_cursor = pos2(
            rect.min.x + button_padding.x + icon_width + icon_spacing,
            rect.center().y - 0.5 * galley.size.y,
        );
        let (small_icon_rect, big_icon_rect) = ui.style().spacing.icon_rectangles(rect);
        ui.painter().add(PaintCmd::Rect {
            rect: big_icon_rect,
            corner_radius: visuals.corner_radius,
            fill: visuals.bg_fill,
            stroke: visuals.bg_stroke,
        });

        if *checked {
            // Check mark:
            ui.painter().add(PaintCmd::line(
                vec![
                    pos2(small_icon_rect.left(), small_icon_rect.center().y),
                    pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                    pos2(small_icon_rect.right(), small_icon_rect.top()),
                ],
                visuals.fg_stroke,
            ));
        }

        let text_color = text_color
            .or(ui.style().visuals.override_text_color)
            .unwrap_or_else(|| visuals.text_color());
        ui.painter()
            .galley(text_cursor, galley, text_style, text_color);
        response
    }
}

// ----------------------------------------------------------------------------

/// One out of several alternatives, either selected or not.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
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

        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];

        let icon_width = ui.style().spacing.icon_width;
        let icon_spacing = ui.style().spacing.icon_spacing;
        let button_padding = ui.style().spacing.button_padding;
        let total_extra = button_padding + vec2(icon_width + icon_spacing, 0.0) + button_padding;

        let single_line = ui.layout().is_horizontal();
        let galley = if single_line {
            font.layout_single_line(text)
        } else {
            font.layout_multiline(text, ui.available_width() - total_extra.x)
        };

        let mut desired_size = total_extra + galley.size;
        desired_size = desired_size.at_least(ui.style().spacing.interact_size);
        desired_size.y = desired_size.y.max(icon_width);
        let response = ui.allocate_response(desired_size, Sense::click());
        let rect = ui
            .layout()
            .align_size_within_rect(desired_size, response.rect);

        let text_cursor = pos2(
            rect.min.x + button_padding.x + icon_width + icon_spacing,
            rect.center().y - 0.5 * galley.size.y,
        );

        let visuals = ui.style().interact(&response);

        let (small_icon_rect, big_icon_rect) = ui.style().spacing.icon_rectangles(rect);

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

        let text_color = text_color
            .or(ui.style().visuals.override_text_color)
            .unwrap_or_else(|| visuals.text_color());
        painter.galley(text_cursor, galley, text_style, text_color);
        response
    }
}

// ----------------------------------------------------------------------------

/// One out of several alternatives, either selected or not.
/// Will mark selected items with a different background color
/// An alternative to [`RadioButton`] and [`Checkbox`].
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Debug)]
pub struct SelectableLabel {
    selected: bool,
    text: String,
}

impl SelectableLabel {
    pub fn new(selected: bool, text: impl Into<String>) -> Self {
        Self {
            selected,
            text: text.into(),
        }
    }
}

impl Widget for SelectableLabel {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { selected, text } = self;

        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];

        let button_padding = ui.style().spacing.button_padding;
        let total_extra = button_padding + button_padding;

        let galley = font.layout_multiline(text, ui.available_width() - total_extra.x);

        let mut desired_size = total_extra + galley.size;
        desired_size = desired_size.at_least(ui.style().spacing.interact_size);
        let response = ui.allocate_response(desired_size, Sense::click());

        let text_cursor = pos2(
            response.rect.min.x + button_padding.x,
            response.rect.center().y - 0.5 * galley.size.y,
        );

        let visuals = ui.style().interact(&response);

        if selected || response.hovered {
            let bg_fill = if selected {
                ui.style().visuals.selection.bg_fill
            } else {
                Default::default()
            };
            ui.painter()
                .rect(response.rect, 0.0, bg_fill, visuals.bg_stroke);
        }

        let text_color = ui
            .style()
            .visuals
            .override_text_color
            .unwrap_or_else(|| visuals.text_color());
        ui.painter()
            .galley(text_cursor, galley, text_style, text_color);
        response
    }
}

// ----------------------------------------------------------------------------

/// A visual separator. A horizontal or vertical line (depending on [`Layout`]).
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
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

        let available_space = ui.available_size_before_wrap_finite();

        let size = if ui.layout().main_dir().is_horizontal() {
            vec2(spacing, available_space.y)
        } else {
            vec2(available_space.x, spacing)
        };

        let response = ui.allocate_response(size, Sense::hover());
        let rect = response.rect;
        let points = if ui.layout().main_dir().is_horizontal() {
            [
                pos2(rect.center().x, rect.top()),
                pos2(rect.center().x, rect.bottom()),
            ]
        } else {
            [
                pos2(rect.left(), rect.center().y),
                pos2(rect.right(), rect.center().y),
            ]
        };
        let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
        ui.painter().line_segment(points, stroke);
        response
    }
}
