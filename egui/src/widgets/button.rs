use crate::*;

/// Clickable button with text.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Button {
    text: String,
    text_color: Option<Color32>,
    text_style: TextStyle,
    /// None means default for interact
    fill: Option<Color32>,
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

    pub fn text_color(mut self, text_color: Color32) -> Self {
        self.text_color = Some(text_color);
        self
    }

    pub fn text_color_opt(mut self, text_color: Option<Color32>) -> Self {
        self.text_color = text_color;
        self
    }

    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = text_style;
        self
    }

    pub fn fill(mut self, fill: Option<Color32>) -> Self {
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
    ///
    /// This is a convenience for [`Ui::set_enabled`].
    pub fn enabled(mut self, enabled: bool) -> Self {
        if !enabled {
            self.sense = Sense::hover();
        }
        self
    }
}

impl Button {
    fn enabled_ui(self, ui: &mut Ui) -> Response {
        let Button {
            text,
            text_color,
            text_style,
            fill,
            sense,
            small,
            frame,
        } = self;

        let mut button_padding = ui.spacing().button_padding;
        if small {
            button_padding.y = 0.0;
        }
        let total_extra = button_padding + button_padding;

        let font = &ui.fonts()[text_style];
        let galley = if ui.wrap_text() {
            font.layout_multiline(text, ui.available_width() - total_extra.x)
        } else {
            font.layout_no_wrap(text)
        };

        let mut desired_size = galley.size + 2.0 * button_padding;
        if !small {
            desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);
        }

        let (rect, response) = ui.allocate_at_least(desired_size, sense);

        if ui.clip_rect().intersects(rect) {
            let visuals = ui.style().interact(&response);
            let text_cursor = ui
                .layout()
                .align_size_within_rect(galley.size, rect.shrink2(button_padding))
                .min;

            if frame {
                let fill = fill.unwrap_or(visuals.bg_fill);
                ui.painter().rect(
                    rect.expand(visuals.expansion),
                    visuals.corner_radius,
                    fill,
                    visuals.bg_stroke,
                );
            }

            let text_color = text_color
                .or(ui.visuals().override_text_color)
                .unwrap_or_else(|| visuals.text_color());
            ui.painter()
                .galley(text_cursor, galley, text_style, text_color);
        }

        response
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let button_enabled = self.sense != Sense::hover();
        if button_enabled || !ui.enabled() {
            self.enabled_ui(ui)
        } else {
            // We need get a temporary disabled `Ui` to get that grayed out look:
            ui.wrap(|ui| {
                ui.set_enabled(false);
                self.enabled_ui(ui)
            })
            .inner
        }
    }
}

// ----------------------------------------------------------------------------

// TODO: allow checkbox without a text label
/// Boolean on/off control with text label.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Debug)]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    text: String,
    text_color: Option<Color32>,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, text: impl Into<String>) -> Self {
        Checkbox {
            checked,
            text: text.into(),
            text_color: None,
        }
    }

    pub fn text_color(mut self, text_color: Color32) -> Self {
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

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;
        let icon_spacing = ui.spacing().icon_spacing;
        let button_padding = spacing.button_padding;
        let total_extra = button_padding + vec2(icon_width + icon_spacing, 0.0) + button_padding;

        let galley = if ui.wrap_text() {
            font.layout_multiline(text, ui.available_width() - total_extra.x)
        } else {
            font.layout_no_wrap(text)
        };

        let mut desired_size = total_extra + galley.size;
        desired_size = desired_size.at_least(spacing.interact_size);
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());
        if response.clicked() {
            *checked = !*checked;
        }

        let visuals = ui.style().interact(&response);
        let text_cursor = pos2(
            rect.min.x + button_padding.x + icon_width + icon_spacing,
            rect.center().y - 0.5 * galley.size.y,
        );
        let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
        ui.painter().add(Shape::Rect {
            rect: big_icon_rect.expand(visuals.expansion),
            corner_radius: visuals.corner_radius,
            fill: visuals.bg_fill,
            stroke: visuals.bg_stroke,
        });

        if *checked {
            // Check mark:
            ui.painter().add(Shape::line(
                vec![
                    pos2(small_icon_rect.left(), small_icon_rect.center().y),
                    pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                    pos2(small_icon_rect.right(), small_icon_rect.top()),
                ],
                visuals.fg_stroke,
                // ui.visuals().selection.stroke, // too much color
            ));
        }

        let text_color = text_color
            .or(ui.visuals().override_text_color)
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
    text_color: Option<Color32>,
}

impl RadioButton {
    pub fn new(checked: bool, text: impl Into<String>) -> Self {
        Self {
            checked,
            text: text.into(),
            text_color: None,
        }
    }

    pub fn text_color(mut self, text_color: Color32) -> Self {
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

        let icon_width = ui.spacing().icon_width;
        let icon_spacing = ui.spacing().icon_spacing;
        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + vec2(icon_width + icon_spacing, 0.0) + button_padding;

        let galley = if ui.wrap_text() {
            font.layout_multiline(text, ui.available_width() - total_extra.x)
        } else {
            font.layout_no_wrap(text)
        };

        let mut desired_size = total_extra + galley.size;
        desired_size = desired_size.at_least(ui.spacing().interact_size);
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        let text_cursor = pos2(
            rect.min.x + button_padding.x + icon_width + icon_spacing,
            rect.center().y - 0.5 * galley.size.y,
        );

        let visuals = ui.style().interact(&response);

        let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);

        let painter = ui.painter();

        painter.add(Shape::Circle {
            center: big_icon_rect.center(),
            radius: big_icon_rect.width() / 2.0 + visuals.expansion,
            fill: visuals.bg_fill,
            stroke: visuals.bg_stroke,
        });

        if checked {
            painter.add(Shape::Circle {
                center: small_icon_rect.center(),
                radius: small_icon_rect.width() / 3.0,
                fill: visuals.fg_stroke.color, // Intentional to use stroke and not fill
                // fill: ui.visuals().selection.stroke.color, // too much color
                stroke: Default::default(),
            });
        }

        let text_color = text_color
            .or(ui.visuals().override_text_color)
            .unwrap_or_else(|| visuals.text_color());
        painter.galley(text_cursor, galley, text_style, text_color);
        response
    }
}

// ----------------------------------------------------------------------------

/// A clickable image within a frame.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Clone, Debug)]
pub struct ImageButton {
    image: widgets::Image,
    sense: Sense,
    frame: bool,
    selected: bool,
}

impl ImageButton {
    pub fn new(texture_id: TextureId, size: impl Into<Vec2>) -> Self {
        Self {
            image: widgets::Image::new(texture_id, size),
            sense: Sense::click(),
            frame: true,
            selected: false,
        }
    }

    /// Select UV range. Default is (0,0) in top-left, (1,1) bottom right.
    pub fn uv(mut self, uv: impl Into<Rect>) -> Self {
        self.image = self.image.uv(uv);
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
    pub fn tint(mut self, tint: impl Into<Color32>) -> Self {
        self.image = self.image.tint(tint);
        self
    }

    /// If `true`, mark this button as "selected".
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
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
}

impl Widget for ImageButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            image,
            sense,
            frame,
            selected,
        } = self;

        let button_padding = ui.spacing().button_padding;
        let size = image.size() + 2.0 * button_padding;
        let (rect, response) = ui.allocate_exact_size(size, sense);

        if ui.clip_rect().intersects(rect) {
            let visuals = ui.style().interact(&response);

            if selected {
                let selection = ui.visuals().selection;
                ui.painter()
                    .rect(rect, 0.0, selection.bg_fill, selection.stroke);
            } else if frame {
                ui.painter().rect(
                    rect.expand(visuals.expansion),
                    visuals.corner_radius,
                    visuals.bg_fill,
                    visuals.bg_stroke,
                );
            }

            let image_rect = ui
                .layout()
                .align_size_within_rect(image.size(), rect.shrink2(button_padding));
            image.paint_at(ui, image_rect);
        }

        response
    }
}
