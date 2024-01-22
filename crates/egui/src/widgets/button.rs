use crate::*;

/// Clickable button with text.
///
/// See also [`Ui::button`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # fn do_stuff() {}
///
/// if ui.add(egui::Button::new("Click me")).clicked() {
///     do_stuff();
/// }
///
/// // A greyed-out and non-interactive button:
/// if ui.add_enabled(false, egui::Button::new("Can't click this")).clicked() {
///     unreachable!();
/// }
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Button<'a> {
    image: Option<Image<'a>>,
    text: Option<WidgetText>,
    shortcut_text: WidgetText,
    wrap: Option<bool>,

    /// None means default for interact
    fill: Option<Color32>,
    stroke: Option<Stroke>,
    sense: Sense,
    small: bool,
    frame: Option<bool>,
    min_size: Vec2,
    rounding: Option<Rounding>,
    selected: bool,
}

impl<'a> Button<'a> {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self::opt_image_and_text(None, Some(text.into()))
    }

    /// Creates a button with an image. The size of the image as displayed is defined by the provided size.
    #[allow(clippy::needless_pass_by_value)]
    pub fn image(image: impl Into<Image<'a>>) -> Self {
        Self::opt_image_and_text(Some(image.into()), None)
    }

    /// Creates a button with an image to the left of the text. The size of the image as displayed is defined by the provided size.
    #[allow(clippy::needless_pass_by_value)]
    pub fn image_and_text(image: impl Into<Image<'a>>, text: impl Into<WidgetText>) -> Self {
        Self::opt_image_and_text(Some(image.into()), Some(text.into()))
    }

    pub fn opt_image_and_text(image: Option<Image<'a>>, text: Option<WidgetText>) -> Self {
        Self {
            text,
            image,
            shortcut_text: Default::default(),
            wrap: None,
            fill: None,
            stroke: None,
            sense: Sense::click(),
            small: false,
            frame: None,
            min_size: Vec2::ZERO,
            rounding: None,
            selected: false,
        }
    }

    /// If `true`, the text will wrap to stay within the max width of the [`Ui`].
    ///
    /// By default [`Self::wrap`] will be true in vertical layouts
    /// and horizontal layouts with wrapping,
    /// and false on non-wrapping horizontal layouts.
    ///
    /// Note that any `\n` in the text will always produce a new line.
    #[inline]
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = Some(wrap);
        self
    }

    /// Override background fill color. Note that this will override any on-hover effects.
    /// Calling this will also turn on the frame.
    #[inline]
    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = Some(fill.into());
        self.frame = Some(true);
        self
    }

    /// Override button stroke. Note that this will override any on-hover effects.
    /// Calling this will also turn on the frame.
    #[inline]
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = Some(stroke.into());
        self.frame = Some(true);
        self
    }

    /// Make this a small button, suitable for embedding into text.
    #[inline]
    pub fn small(mut self) -> Self {
        if let Some(text) = self.text {
            self.text = Some(text.text_style(TextStyle::Body));
        }
        self.small = true;
        self
    }

    /// Turn off the frame
    #[inline]
    pub fn frame(mut self, frame: bool) -> Self {
        self.frame = Some(frame);
        self
    }

    /// By default, buttons senses clicks.
    /// Change this to a drag-button with `Sense::drag()`.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Set the minimum size of the button.
    #[inline]
    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    /// Set the rounding of the button.
    #[inline]
    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.rounding = Some(rounding.into());
        self
    }

    /// Show some text on the right side of the button, in weak color.
    ///
    /// Designed for menu buttons, for setting a keyboard shortcut text (e.g. `Ctrl+S`).
    ///
    /// The text can be created with [`Context::format_shortcut`].
    #[inline]
    pub fn shortcut_text(mut self, shortcut_text: impl Into<WidgetText>) -> Self {
        self.shortcut_text = shortcut_text.into();
        self
    }

    /// If `true`, mark this button as "selected".
    #[inline]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Widget for Button<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Button {
            text,
            image,
            shortcut_text,
            wrap,
            fill,
            stroke,
            sense,
            small,
            frame,
            min_size,
            rounding,
            selected,
        } = self;

        let frame = frame.unwrap_or_else(|| ui.visuals().button_frame);

        let mut button_padding = if frame {
            ui.spacing().button_padding
        } else {
            Vec2::ZERO
        };
        if small {
            button_padding.y = 0.0;
        }

        let space_available_for_image = if let Some(text) = &text {
            let font_height = ui.fonts(|fonts| text.font_height(fonts, ui.style()));
            Vec2::splat(font_height) // Reasonable?
        } else {
            ui.available_size() - 2.0 * button_padding
        };

        let image_size = if let Some(image) = &image {
            image
                .load_and_calc_size(ui, space_available_for_image)
                .unwrap_or(space_available_for_image)
        } else {
            Vec2::ZERO
        };

        let mut text_wrap_width = ui.available_width() - 2.0 * button_padding.x;
        if image.is_some() {
            text_wrap_width -= image_size.x + ui.spacing().icon_spacing;
        }
        if !shortcut_text.is_empty() {
            text_wrap_width -= 60.0; // Some space for the shortcut text (which we never wrap).
        }

        let galley =
            text.map(|text| text.into_galley(ui, wrap, text_wrap_width, TextStyle::Button));
        let shortcut_galley = (!shortcut_text.is_empty())
            .then(|| shortcut_text.into_galley(ui, Some(false), f32::INFINITY, TextStyle::Button));

        let mut desired_size = Vec2::ZERO;
        if image.is_some() {
            desired_size.x += image_size.x;
            desired_size.y = desired_size.y.max(image_size.y);
        }
        if image.is_some() && galley.is_some() {
            desired_size.x += ui.spacing().icon_spacing;
        }
        if let Some(text) = &galley {
            desired_size.x += text.size().x;
            desired_size.y = desired_size.y.max(text.size().y);
        }
        if let Some(shortcut_text) = &shortcut_galley {
            desired_size.x += ui.spacing().item_spacing.x + shortcut_text.size().x;
            desired_size.y = desired_size.y.max(shortcut_text.size().y);
        }
        desired_size += 2.0 * button_padding;
        if !small {
            desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);
        }
        desired_size = desired_size.at_least(min_size);

        let (rect, mut response) = ui.allocate_at_least(desired_size, sense);
        response.widget_info(|| {
            if let Some(galley) = &galley {
                WidgetInfo::labeled(WidgetType::Button, galley.text())
            } else {
                WidgetInfo::new(WidgetType::Button)
            }
        });

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);

            let (frame_expansion, frame_rounding, frame_fill, frame_stroke) = if selected {
                let selection = ui.visuals().selection;
                (
                    Vec2::ZERO,
                    Rounding::ZERO,
                    selection.bg_fill,
                    selection.stroke,
                )
            } else if frame {
                let expansion = Vec2::splat(visuals.expansion);
                (
                    expansion,
                    visuals.rounding,
                    visuals.weak_bg_fill,
                    visuals.bg_stroke,
                )
            } else {
                Default::default()
            };
            let frame_rounding = rounding.unwrap_or(frame_rounding);
            let frame_fill = fill.unwrap_or(frame_fill);
            let frame_stroke = stroke.unwrap_or(frame_stroke);
            ui.painter().rect(
                rect.expand2(frame_expansion),
                frame_rounding,
                frame_fill,
                frame_stroke,
            );

            let mut cursor_x = rect.min.x + button_padding.x;

            if let Some(image) = &image {
                let image_rect = Rect::from_min_size(
                    pos2(cursor_x, rect.center().y - 0.5 - (image_size.y / 2.0)),
                    image_size,
                );
                cursor_x += image_size.x;
                let tlr = image.load_for_size(ui.ctx(), image_size);
                widgets::image::paint_texture_load_result(
                    ui,
                    &tlr,
                    image_rect,
                    image.show_loading_spinner,
                    image.image_options(),
                );
                response =
                    widgets::image::texture_load_result_response(image.source(), &tlr, response);
            }

            if image.is_some() && galley.is_some() {
                cursor_x += ui.spacing().icon_spacing;
            }

            if let Some(galley) = galley {
                let text_pos = if image.is_some() || shortcut_galley.is_some() {
                    pos2(cursor_x, rect.center().y - 0.5 * galley.size().y)
                } else {
                    // Make sure button text is centered if within a centered layout
                    ui.layout()
                        .align_size_within_rect(galley.size(), rect.shrink2(button_padding))
                        .min
                };
                ui.painter().galley(text_pos, galley, visuals.text_color());
            }

            if let Some(shortcut_galley) = shortcut_galley {
                let shortcut_text_pos = pos2(
                    rect.max.x - button_padding.x - shortcut_galley.size().x,
                    rect.center().y - 0.5 * shortcut_galley.size().y,
                );
                ui.painter().galley(
                    shortcut_text_pos,
                    shortcut_galley,
                    ui.visuals().weak_text_color(),
                );
            }
        }

        if let Some(cursor) = ui.visuals().interact_cursor {
            if response.hovered {
                ui.ctx().set_cursor_icon(cursor);
            }
        }

        response
    }
}

// ----------------------------------------------------------------------------

// TODO(emilk): allow checkbox without a text label
/// Boolean on/off control with text label.
///
/// Usually you'd use [`Ui::checkbox`] instead.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_bool = true;
/// // These are equivalent:
/// ui.checkbox(&mut my_bool, "Checked");
/// ui.add(egui::Checkbox::new(&mut my_bool, "Checked"));
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    text: WidgetText,
    indeterminate: bool,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, text: impl Into<WidgetText>) -> Self {
        Checkbox {
            checked,
            text: text.into(),
            indeterminate: false,
        }
    }

    pub fn without_text(checked: &'a mut bool) -> Self {
        Self::new(checked, WidgetText::default())
    }

    /// Display an indeterminate state (neither checked nor unchecked)
    ///
    /// This only affects the checkbox's appearance. It will still toggle its boolean value when
    /// clicked.
    #[inline]
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }
}

impl<'a> Widget for Checkbox<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Checkbox {
            checked,
            text,
            indeterminate,
        } = self;

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;
        let icon_spacing = spacing.icon_spacing;

        let (galley, mut desired_size) = if text.is_empty() {
            (None, vec2(icon_width, 0.0))
        } else {
            let total_extra = vec2(icon_width + icon_spacing, 0.0);

            let wrap_width = ui.available_width() - total_extra.x;
            let galley = text.into_galley(ui, None, wrap_width, TextStyle::Button);

            let mut desired_size = total_extra + galley.size();
            desired_size = desired_size.at_least(spacing.interact_size);

            (Some(galley), desired_size)
        };

        desired_size = desired_size.at_least(Vec2::splat(spacing.interact_size.y));
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

        if response.clicked() {
            *checked = !*checked;
            response.mark_changed();
        }
        response.widget_info(|| {
            if indeterminate {
                WidgetInfo::labeled(
                    WidgetType::Checkbox,
                    galley.as_ref().map_or("", |x| x.text()),
                )
            } else {
                WidgetInfo::selected(
                    WidgetType::Checkbox,
                    *checked,
                    galley.as_ref().map_or("", |x| x.text()),
                )
            }
        });

        if ui.is_rect_visible(rect) {
            // let visuals = ui.style().interact_selectable(&response, *checked); // too colorful
            let visuals = ui.style().interact(&response);
            let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);
            ui.painter().add(epaint::RectShape::new(
                big_icon_rect.expand(visuals.expansion),
                visuals.rounding,
                visuals.bg_fill,
                visuals.bg_stroke,
            ));

            if indeterminate {
                // Horizontal line:
                ui.painter().add(Shape::hline(
                    small_icon_rect.x_range(),
                    small_icon_rect.center().y,
                    visuals.fg_stroke,
                ));
            } else if *checked {
                // Check mark:
                ui.painter().add(Shape::line(
                    vec![
                        pos2(small_icon_rect.left(), small_icon_rect.center().y),
                        pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                        pos2(small_icon_rect.right(), small_icon_rect.top()),
                    ],
                    visuals.fg_stroke,
                ));
            }
            if let Some(galley) = galley {
                let text_pos = pos2(
                    rect.min.x + icon_width + icon_spacing,
                    rect.center().y - 0.5 * galley.size().y,
                );
                ui.painter().galley(text_pos, galley, visuals.text_color());
            }
        }

        response
    }
}

// ----------------------------------------------------------------------------

/// One out of several alternatives, either selected or not.
///
/// Usually you'd use [`Ui::radio_value`] or [`Ui::radio`] instead.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// #[derive(PartialEq)]
/// enum Enum { First, Second, Third }
/// let mut my_enum = Enum::First;
///
/// ui.radio_value(&mut my_enum, Enum::First, "First");
///
/// // is equivalent to:
///
/// if ui.add(egui::RadioButton::new(my_enum == Enum::First, "First")).clicked() {
///     my_enum = Enum::First
/// }
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct RadioButton {
    checked: bool,
    text: WidgetText,
}

impl RadioButton {
    pub fn new(checked: bool, text: impl Into<WidgetText>) -> Self {
        Self {
            checked,
            text: text.into(),
        }
    }
}

impl Widget for RadioButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { checked, text } = self;

        let spacing = &ui.spacing();
        let icon_width = spacing.icon_width;
        let icon_spacing = spacing.icon_spacing;

        let (galley, mut desired_size) = if text.is_empty() {
            (None, vec2(icon_width, 0.0))
        } else {
            let total_extra = vec2(icon_width + icon_spacing, 0.0);

            let wrap_width = ui.available_width() - total_extra.x;
            let text = text.into_galley(ui, None, wrap_width, TextStyle::Button);

            let mut desired_size = total_extra + text.size();
            desired_size = desired_size.at_least(spacing.interact_size);

            (Some(text), desired_size)
        };

        desired_size = desired_size.at_least(Vec2::splat(spacing.interact_size.y));
        desired_size.y = desired_size.y.max(icon_width);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::RadioButton,
                checked,
                galley.as_ref().map_or("", |x| x.text()),
            )
        });

        if ui.is_rect_visible(rect) {
            // let visuals = ui.style().interact_selectable(&response, checked); // too colorful
            let visuals = ui.style().interact(&response);

            let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);

            let painter = ui.painter();

            painter.add(epaint::CircleShape {
                center: big_icon_rect.center(),
                radius: big_icon_rect.width() / 2.0 + visuals.expansion,
                fill: visuals.bg_fill,
                stroke: visuals.bg_stroke,
            });

            if checked {
                painter.add(epaint::CircleShape {
                    center: small_icon_rect.center(),
                    radius: small_icon_rect.width() / 3.0,
                    fill: visuals.fg_stroke.color, // Intentional to use stroke and not fill
                    // fill: ui.visuals().selection.stroke.color, // too much color
                    stroke: Default::default(),
                });
            }

            if let Some(galley) = galley {
                let text_pos = pos2(
                    rect.min.x + icon_width + icon_spacing,
                    rect.center().y - 0.5 * galley.size().y,
                );
                ui.painter().galley(text_pos, galley, visuals.text_color());
            }
        }

        response
    }
}

// ----------------------------------------------------------------------------

/// A clickable image within a frame.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Clone, Debug)]
pub struct ImageButton<'a> {
    image: Image<'a>,
    sense: Sense,
    frame: bool,
    selected: bool,
}

impl<'a> ImageButton<'a> {
    pub fn new(image: impl Into<Image<'a>>) -> Self {
        Self {
            image: image.into(),
            sense: Sense::click(),
            frame: true,
            selected: false,
        }
    }

    /// Select UV range. Default is (0,0) in top-left, (1,1) bottom right.
    #[inline]
    pub fn uv(mut self, uv: impl Into<Rect>) -> Self {
        self.image = self.image.uv(uv);
        self
    }

    /// Multiply image color with this. Default is WHITE (no tint).
    #[inline]
    pub fn tint(mut self, tint: impl Into<Color32>) -> Self {
        self.image = self.image.tint(tint);
        self
    }

    /// If `true`, mark this button as "selected".
    #[inline]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Turn off the frame
    #[inline]
    pub fn frame(mut self, frame: bool) -> Self {
        self.frame = frame;
        self
    }

    /// By default, buttons senses clicks.
    /// Change this to a drag-button with `Sense::drag()`.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Set rounding for the `ImageButton`.
    /// If the underlying image already has rounding, this
    /// will override that value.
    #[inline]
    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.image = self.image.rounding(rounding.into());
        self
    }
}

impl<'a> Widget for ImageButton<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let padding = if self.frame {
            // so we can see that it is a button:
            Vec2::splat(ui.spacing().button_padding.x)
        } else {
            Vec2::ZERO
        };

        let available_size_for_image = ui.available_size() - 2.0 * padding;
        let tlr = self.image.load_for_size(ui.ctx(), available_size_for_image);
        let original_image_size = tlr.as_ref().ok().and_then(|t| t.size());
        let image_size = self
            .image
            .calc_size(available_size_for_image, original_image_size);

        let padded_size = image_size + 2.0 * padding;
        let (rect, response) = ui.allocate_exact_size(padded_size, self.sense);
        response.widget_info(|| WidgetInfo::new(WidgetType::ImageButton));

        if ui.is_rect_visible(rect) {
            let (expansion, rounding, fill, stroke) = if self.selected {
                let selection = ui.visuals().selection;
                (
                    Vec2::ZERO,
                    self.image.image_options().rounding,
                    selection.bg_fill,
                    selection.stroke,
                )
            } else if self.frame {
                let visuals = ui.style().interact(&response);
                let expansion = Vec2::splat(visuals.expansion);
                (
                    expansion,
                    self.image.image_options().rounding,
                    visuals.weak_bg_fill,
                    visuals.bg_stroke,
                )
            } else {
                Default::default()
            };

            // Draw frame background (for transparent images):
            ui.painter()
                .rect_filled(rect.expand2(expansion), rounding, fill);

            let image_rect = ui
                .layout()
                .align_size_within_rect(image_size, rect.shrink2(padding));
            // let image_rect = image_rect.expand2(expansion); // can make it blurry, so let's not
            let image_options = self.image.image_options().clone();

            widgets::image::paint_texture_load_result(ui, &tlr, image_rect, None, &image_options);

            // Draw frame outline:
            ui.painter()
                .rect_stroke(rect.expand2(expansion), rounding, stroke);
        }

        widgets::image::texture_load_result_response(self.image.source(), &tlr, response)
    }
}
