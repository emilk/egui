use crate::{
    Atomic, AtomicKind, AtomicLayout, AtomicLayoutResponse, Atomics, Color32, CornerRadius, Frame,
    Image, IntoAtomics, NumExt as _, Response, Sense, SizedAtomicKind, Stroke, TextWrapMode, Ui,
    Vec2, Widget, WidgetInfo, WidgetText, WidgetType,
};

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
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Button<'a> {
    atomics: Atomics<'a>,
    wrap_mode: Option<TextWrapMode>,
    fill: Option<Color32>,
    stroke: Option<Stroke>,
    sense: Sense,
    small: bool,
    frame: Option<bool>,
    min_size: Vec2,
    corner_radius: Option<CornerRadius>,
    selected: bool,
    image_tint_follows_text_color: bool,
}

impl<'a> Button<'a> {
    pub fn new(content: impl IntoAtomics<'a>) -> Self {
        Self {
            atomics: content.into_atomics(),
            wrap_mode: None,
            fill: None,
            stroke: None,
            sense: Sense::click(),
            small: false,
            frame: None,
            min_size: Vec2::ZERO,
            corner_radius: None,
            selected: false,
            image_tint_follows_text_color: false,
        }
    }

    /// Creates a button with an image. The size of the image as displayed is defined by the provided size.
    pub fn image(image: impl Into<Image<'a>>) -> Self {
        Self::opt_image_and_text(Some(image.into()), None)
    }

    /// Creates a button with an image to the left of the text. The size of the image as displayed is defined by the provided size.
    pub fn image_and_text(image: impl Into<Image<'a>>, text: impl Into<WidgetText>) -> Self {
        Self::opt_image_and_text(Some(image.into()), Some(text.into()))
    }

    pub fn opt_image_and_text(image: Option<Image<'a>>, text: Option<WidgetText>) -> Self {
        let mut button = Self::new(());
        if let Some(image) = image {
            button.atomics.push_left(image);
        }
        if let Some(text) = text {
            button.atomics.push_left(text);
        }
        button
    }

    /// Set the wrap mode for the text.
    ///
    /// By default, [`crate::Ui::wrap_mode`] will be used, which can be overridden with [`crate::Style::wrap_mode`].
    ///
    /// Note that any `\n` in the text will always produce a new line.
    #[inline]
    pub fn wrap_mode(mut self, wrap_mode: TextWrapMode) -> Self {
        self.wrap_mode = Some(wrap_mode);
        self
    }

    /// Set [`Self::wrap_mode`] to [`TextWrapMode::Wrap`].
    #[inline]
    pub fn wrap(mut self) -> Self {
        self.wrap_mode = Some(TextWrapMode::Wrap);

        self
    }

    /// Set [`Self::wrap_mode`] to [`TextWrapMode::Truncate`].
    #[inline]
    pub fn truncate(mut self) -> Self {
        self.wrap_mode = Some(TextWrapMode::Truncate);
        self
    }

    /// Override background fill color. Note that this will override any on-hover effects.
    /// Calling this will also turn on the frame.
    #[inline]
    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = Some(fill.into());
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
    pub fn corner_radius(mut self, corner_radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius = Some(corner_radius.into());
        self
    }

    #[inline]
    #[deprecated = "Renamed to `corner_radius`"]
    pub fn rounding(self, corner_radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius(corner_radius)
    }

    /// If true, the tint of the image is multiplied by the widget text color.
    ///
    /// This makes sense for images that are white, that should have the same color as the text color.
    /// This will also make the icon color depend on hover state.
    ///
    /// Default: `false`.
    #[inline]
    pub fn image_tint_follows_text_color(mut self, image_tint_follows_text_color: bool) -> Self {
        self.image_tint_follows_text_color = image_tint_follows_text_color;
        self
    }

    /// Show some text on the right side of the button, in weak color.
    ///
    /// Designed for menu buttons, for setting a keyboard shortcut text (e.g. `Ctrl+S`).
    ///
    /// The text can be created with [`crate::Context::format_shortcut`].
    ///
    /// See also [`Self::right_text`].
    #[inline]
    pub fn shortcut_text(mut self, shortcut_text: impl Into<Atomic<'a>>) -> Self {
        let mut atomic = shortcut_text.into();
        atomic.kind = match atomic.kind {
            AtomicKind::Text(text) => AtomicKind::Text(text.weak()),
            other => other,
        };
        self.atomics.push_left(Atomic::grow());
        self.atomics.push_left(atomic);
        self
    }

    /// Show some text on the right side of the button.
    #[inline]
    pub fn right_text(mut self, right_text: impl Into<Atomic<'a>>) -> Self {
        self.atomics.push_left(Atomic::grow());
        self.atomics.push_left(right_text.into());
        self
    }

    /// If `true`, mark this button as "selected".
    #[inline]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Show the button and return a [`AtomicLayoutResponse`] for painting custom contents.
    pub fn atomic_ui(self, ui: &mut Ui) -> AtomicLayoutResponse {
        let Button {
            atomics,
            wrap_mode,
            fill,
            stroke,
            sense,
            small,
            frame,
            mut min_size,
            corner_radius,
            selected,
            image_tint_follows_text_color,
        } = self;

        if !small {
            min_size.y = min_size.y.at_least(ui.spacing().interact_size.y);
        }

        let text = atomics.text();

        let layout = AtomicLayout::new(atomics)
            .wrap_mode(wrap_mode.unwrap_or(ui.wrap_mode()))
            .sense(sense)
            .min_size(min_size);

        let has_frame = frame.unwrap_or_else(|| ui.visuals().button_frame);

        let mut button_padding = if has_frame {
            ui.spacing().button_padding
        } else {
            Vec2::ZERO
        };
        if small {
            button_padding.y = 0.0;
        }

        let mut prepared = layout
            .frame(Frame::new().inner_margin(button_padding))
            .allocate(ui);

        let response = if ui.is_rect_visible(prepared.response.rect) {
            let visuals = ui.style().interact_selectable(&prepared.response, selected);

            if image_tint_follows_text_color {
                prepared.sized_atomics.iter_mut().for_each(|a| {
                    a.kind = match std::mem::take(&mut a.kind) {
                        SizedAtomicKind::Image(image, size) => {
                            SizedAtomicKind::Image(image.tint(visuals.text_color()), size)
                        }
                        other => other,
                    }
                });
            }

            prepared.fallback_text_color = visuals.text_color();

            if has_frame {
                let stroke = stroke.unwrap_or(visuals.bg_stroke);
                let fill = fill.unwrap_or(visuals.weak_bg_fill);
                prepared.frame = prepared
                    .frame
                    .inner_margin(
                        button_padding + Vec2::splat(visuals.expansion) - Vec2::splat(stroke.width),
                    )
                    .outer_margin(-Vec2::splat(visuals.expansion))
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(corner_radius.unwrap_or(visuals.corner_radius));
            };

            prepared.paint(ui)
        } else {
            AtomicLayoutResponse::empty(prepared.response)
        };

        response.response.widget_info(|| {
            if let Some(text) = &text {
                WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), text)
            } else {
                WidgetInfo::new(WidgetType::Button)
            }
        });

        response
    }
}

impl Widget for Button<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.atomic_ui(ui).response
    }
}
