use crate::{
    Atom, AtomExt as _, AtomKind, AtomLayout, AtomLayoutResponse, Atoms, Color32, CornerRadius,
    Image, IntoAtoms, NumExt as _, Response, Sense, Stroke, TextStyle, TextWrapMode, Ui, Vec2,
    Widget, WidgetInfo, WidgetText, WidgetType,
    widget_style::{ButtonStyle, ClassName, Classes, HasClasses},
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
    layout: AtomLayout<'a>,
    fill: Option<Color32>,
    stroke: Option<Stroke>,
    min_size: Vec2,
    corner_radius: Option<CornerRadius>,
    selected: Option<bool>,
    image_tint_follows_text_color: bool,
    limit_image_size: bool,
    classes: Classes,
}

impl<'a> Button<'a> {
    /// Present on a selected button.
    pub const CLASS_SELECTED: ClassName = ClassName::from_static("egui::selected");

    /// Present on a small button.
    pub const CLASS_SMALL: ClassName = ClassName::from_static("egui::small");

    /// Present on a button that should have no frame at all.
    pub const CLASS_NO_FRAME: ClassName = ClassName::from_static("egui::no_frame");

    /// Present on a button that should have a frame, even when the global default is frameless.
    pub const CLASS_FRAME: ClassName = ClassName::from_static("egui::frame");

    /// Present on a button that should have no frame while it is inactive.
    pub const CLASS_HIDE_FRAME_WHEN_INACTIVE: ClassName =
        ClassName::from_static("egui::button::hide_frame_when_inactive");

    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        Self {
            layout: AtomLayout::new(atoms.into_atoms())
                .sense(Sense::click())
                .fallback_font(TextStyle::Button),
            fill: None,
            stroke: None,
            min_size: Vec2::ZERO,
            corner_radius: None,
            selected: None,
            image_tint_follows_text_color: false,
            limit_image_size: false,
            classes: Classes::default(),
        }
    }

    /// Show a selectable button.
    ///
    /// Equivalent to:
    /// ```rust
    /// # use egui::{Button, IntoAtoms, __run_test_ui};
    /// # __run_test_ui(|ui| {
    /// let selected = true;
    /// ui.add(Button::new("toggle me").selected(selected).frame_when_inactive(!selected).frame(true));
    /// # });
    /// ```
    ///
    /// When selected, [`Self::CLASS_SELECTED`] is added.
    ///
    /// See also:
    ///   - [`Ui::selectable_value`]
    ///   - [`Ui::selectable_label`]
    pub fn selectable(selected: bool, atoms: impl IntoAtoms<'a>) -> Self {
        Self::new(atoms)
            .selected(selected)
            .frame_when_inactive(selected)
            .frame(true)
    }

    /// Creates a button with an image. The size of the image as displayed is defined by the provided size.
    ///
    /// Note: In contrast to [`Button::new`], this limits the image size to the default font height
    /// (using [`crate::AtomExt::atom_max_height_font_size`]).
    pub fn image(image: impl Into<Image<'a>>) -> Self {
        Self::opt_image_and_text(Some(image.into()), None)
    }

    /// Creates a button with an image to the left of the text.
    ///
    /// Note: In contrast to [`Button::new`], this limits the image size to the default font height
    /// (using [`crate::AtomExt::atom_max_height_font_size`]).
    pub fn image_and_text(image: impl Into<Image<'a>>, text: impl Into<WidgetText>) -> Self {
        Self::opt_image_and_text(Some(image.into()), Some(text.into()))
    }

    /// Create a button with an optional image and optional text.
    ///
    /// Note: In contrast to [`Button::new`], this limits the image size to the default font height
    /// (using [`crate::AtomExt::atom_max_height_font_size`]).
    pub fn opt_image_and_text(image: Option<Image<'a>>, text: Option<WidgetText>) -> Self {
        let mut button = Self::new(());
        if let Some(image) = image {
            button.layout.push_right(image);
        }
        if let Some(text) = text {
            button.layout.push_right(text);
        }
        button.limit_image_size = true;
        button
    }

    /// Set the wrap mode for the text.
    ///
    /// By default, [`crate::Ui::wrap_mode`] will be used, which can be overridden with [`crate::Style::wrap_mode`].
    ///
    /// Note that any `\n` in the text will always produce a new line.
    #[inline]
    pub fn wrap_mode(mut self, wrap_mode: TextWrapMode) -> Self {
        self.layout = self.layout.wrap_mode(wrap_mode);
        self
    }

    /// Set [`Self::wrap_mode`] to [`TextWrapMode::Wrap`].
    #[inline]
    pub fn wrap(self) -> Self {
        self.wrap_mode(TextWrapMode::Wrap)
    }

    /// Set [`Self::wrap_mode`] to [`TextWrapMode::Truncate`].
    #[inline]
    pub fn truncate(self) -> Self {
        self.wrap_mode(TextWrapMode::Truncate)
    }

    /// Override background fill color. Note that this will override any on-hover effects.
    /// Calling this will also turn on the frame.
    #[inline]
    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = Some(fill.into());
        self.frame(true)
    }

    /// Override button stroke. Note that this will override any on-hover effects.
    /// Calling this will also turn on the frame.
    #[inline]
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = Some(stroke.into());
        self.frame(true)
    }

    /// Make this a small button, suitable for embedding into text.
    ///
    /// This adds the built-in [`Self::CLASS_SMALL`], which with the default style removes the top and
    /// bottom margin.
    #[inline]
    pub fn small(self) -> Self {
        self.with_class(Self::CLASS_SMALL)
    }

    /// Turn off the frame
    ///
    /// This adds either the built-in [`Self::CLASS_FRAME`] or [`Self::CLASS_NO_FRAME`] class.
    /// With the default style, the latter removes the fill, the stroke and the margin.
    ///
    /// Default: `ui.visuals().button_frame`.
    #[inline]
    pub fn frame(mut self, frame: bool) -> Self {
        self.set_class(Self::CLASS_FRAME, frame);
        self.set_class(Self::CLASS_NO_FRAME, !frame);
        self
    }

    /// If `false`, the button will not have a frame when inactive.
    ///
    /// This adds the built-in [`Self::CLASS_HIDE_FRAME_WHEN_INACTIVE`], which with the
    /// default style removes the fill and the stroke, but keeps the margin, so the button does
    /// not change size once the user interacts with it.
    ///
    /// Default: `true`.
    ///
    /// Note: When [`Self::frame`] (or `ui.visuals().button_frame`) is `false`, this setting
    /// has no effect.
    #[inline]
    pub fn frame_when_inactive(mut self, frame_when_inactive: bool) -> Self {
        self.set_class(Self::CLASS_HIDE_FRAME_WHEN_INACTIVE, !frame_when_inactive);
        self
    }

    /// By default, buttons senses clicks.
    /// Change this to a drag-button with `Sense::drag()`.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.layout = self.layout.sense(sense);
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
    pub fn shortcut_text(mut self, shortcut_text: impl IntoAtoms<'a>) -> Self {
        self.layout.push_right(Atom::grow());

        for mut atom in shortcut_text.into_atoms() {
            atom.kind = match atom.kind {
                AtomKind::Text(text) => AtomKind::Text(text.weak()),
                other => other,
            };
            self.layout.push_right(atom);
        }

        self
    }

    /// Show some text on the left side of the button.
    #[inline]
    pub fn left_text(mut self, left_text: impl IntoAtoms<'a>) -> Self {
        self.layout.push_left(Atom::grow());

        for atom in left_text.into_atoms() {
            self.layout.push_left(atom);
        }

        self
    }

    /// Show some text on the right side of the button.
    #[inline]
    pub fn right_text(mut self, right_text: impl IntoAtoms<'a>) -> Self {
        self.layout.push_right(Atom::grow());

        for atom in right_text.into_atoms() {
            self.layout.push_right(atom);
        }

        self
    }

    /// If `true`, mark this button as "selected".
    ///
    /// Calling this method opts the button into toggle semantics and the
    /// current pressed/not-pressed state will be reported to assistive
    /// technologies (e.g. screen readers). Plain buttons that never call
    /// `selected` are not announced as toggles.
    ///
    /// When selected, [`Self::CLASS_SELECTED`] is added. You should prefer calling this though over
    /// just adding [`Self::CLASS_SELECTED`] manually, since this also exposes accessibility information.
    #[inline]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self.set_class(Self::CLASS_SELECTED, selected);
        self
    }

    /// Set the gap between atoms.
    #[inline]
    pub fn gap(mut self, gap: f32) -> Self {
        self.layout = self.layout.gap(gap);
        self
    }

    /// Output the button's [`Atoms`].
    ///
    /// This includes any images you have on the button.
    pub fn atoms(&self) -> &Atoms<'a> {
        &self.layout.atoms
    }

    /// Show the button and return a [`AtomLayoutResponse`] for painting custom contents.
    pub fn atom_ui(self, ui: &mut Ui) -> AtomLayoutResponse {
        let Button {
            mut layout,
            fill,
            stroke,
            mut min_size,
            corner_radius,
            selected,
            image_tint_follows_text_color,
            limit_image_size,
            classes,
        } = self;

        if limit_image_size {
            layout.map_atoms(|atom| {
                if matches!(&atom.kind, AtomKind::Image(_)) {
                    atom.atom_max_height_font_size(ui)
                } else {
                    atom
                }
            });
        }

        let text = layout.text().map(String::from);

        let id = ui.next_auto_id();
        let ButtonStyle {
            mut frame,
            text_style,
            min_size: style_min_size,
        } = ui.widget_style(id, &classes);

        min_size = min_size.at_least(style_min_size);

        // Override global style by local style
        if let Some(fill) = fill {
            frame = frame.fill(fill);
        }
        if let Some(corner_radius) = corner_radius {
            frame = frame.corner_radius(corner_radius);
        }
        if let Some(stroke) = stroke {
            frame = frame.stroke(stroke);
        }

        // Apply the style font and color as fallback
        layout = layout
            .fallback_font(text_style.font_id.clone())
            .fallback_text_color(text_style.color);

        let mut prepared = layout.frame(frame).min_size(min_size).allocate(ui);

        // Get AtomLayoutResponse, empty if not visible
        let response = if ui.is_rect_visible(prepared.response.rect) {
            if image_tint_follows_text_color {
                prepared.map_images(|image| image.tint(text_style.color));
            }

            prepared.fallback_text_color = text_style.color;

            prepared.paint(ui)
        } else {
            AtomLayoutResponse::empty(prepared.response)
        };

        if let Some(cursor) = ui.visuals().interact_cursor
            && response.response.hovered()
        {
            ui.ctx().set_cursor_icon(cursor);
        }

        response.response.widget_info(|| match (selected, &text) {
            (Some(selected), Some(text)) => {
                WidgetInfo::selected(WidgetType::Button, ui.is_enabled(), selected, text)
            }
            (Some(selected), None) => {
                let mut info = WidgetInfo::new(WidgetType::Button);
                info.enabled = ui.is_enabled();
                info.selected = Some(selected);
                info
            }
            (None, Some(text)) => WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), text),
            (None, None) => WidgetInfo::new(WidgetType::Button),
        });

        response
    }
}

impl Widget for Button<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.atom_ui(ui).response
    }
}

impl HasClasses for Button<'_> {
    fn classes(&self) -> &Classes {
        &self.classes
    }

    fn classes_mut(&mut self) -> &mut Classes {
        &mut self.classes
    }
}
