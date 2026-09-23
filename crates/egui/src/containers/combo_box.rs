use core::cell::RefCell;

use epaint::Shape;

use crate::{
    Align2, AsIdSalt, Atom, AtomExt as _, Atoms, Context, Id, IdSalt, InnerResponse, IntoAtoms,
    Margin, NumExt as _, Painter, Popup, PopupCloseBehavior, Rect, Response, Role, ScrollArea,
    Sense, TextStyle, TextWrapMode, Ui, Vec2, WidgetAtom, WidgetInfo,
    class::Classes,
    epaint,
    style::{StyleModifier, WidgetVisuals},
    vec2,
    widget_style::ButtonStyle,
};

#[expect(unused_imports)] // Documentation
use crate::style::Spacing;

/// A function that paints the [`ComboBox`] icon
pub type IconPainter = Box<dyn FnOnce(&Ui, Rect, &WidgetVisuals, bool)>;

/// A drop-down selection menu with a descriptive label.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # #[derive(Debug, PartialEq, Copy, Clone)]
/// # enum Enum { First, Second, Third }
/// # let mut selected = Enum::First;
/// let before = selected;
/// egui::ComboBox::from_label("Select one!")
///     .selected_text(format!("{:?}", selected))
///     .show_ui(ui, |ui| {
///         ui.selectable_value(&mut selected, Enum::First, "First");
///         ui.selectable_value(&mut selected, Enum::Second, "Second");
///         ui.selectable_value(&mut selected, Enum::Third, "Third");
///     }
/// );
///
/// if selected != before {
///     // Handle selection change
/// }
/// # });
/// ```
#[must_use = "You should call .show*"]
pub struct ComboBox<'a> {
    id_salt: IdSalt,
    label: Option<Atoms<'a>>,
    selected_text: Atoms<'a>,
    width: Option<f32>,
    height: Option<f32>,
    icon: Option<IconPainter>,
    wrap_mode: Option<TextWrapMode>,
    close_behavior: Option<PopupCloseBehavior>,
    popup_style: StyleModifier,
}

impl<'a> ComboBox<'a> {
    /// Create new [`ComboBox`] with id and label
    pub fn new(id_salt: impl AsIdSalt, label: impl IntoAtoms<'a>) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: Some(label.into_atoms()),
            selected_text: Default::default(),
            width: None,
            height: None,
            icon: None,
            wrap_mode: None,
            close_behavior: None,
            popup_style: StyleModifier::default(),
        }
    }

    /// Label shown next to the combo box.
    ///
    /// The text of the label is used as the id salt.
    pub fn from_label(label: impl IntoAtoms<'a>) -> Self {
        let label = label.into_atoms();
        Self {
            id_salt: label.salt(),
            label: Some(label),
            selected_text: Default::default(),
            width: None,
            height: None,
            icon: None,
            wrap_mode: None,
            close_behavior: None,
            popup_style: StyleModifier::default(),
        }
    }

    /// Without label.
    pub fn from_id_salt(id_salt: impl AsIdSalt) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: Default::default(),
            selected_text: Default::default(),
            width: None,
            height: None,
            icon: None,
            wrap_mode: None,
            close_behavior: None,
            popup_style: StyleModifier::default(),
        }
    }

    /// Set the outer width of the button and menu.
    ///
    /// Default is [`Spacing::combo_width`].
    #[inline]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the maximum outer height of the menu.
    ///
    /// Default is [`Spacing::combo_height`].
    #[inline]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// What we show as the currently selected value.
    ///
    /// This can be any [`IntoAtoms`], e.g. an image next to some text:
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let ferris = egui::include_image!("../../assets/ferris.png");
    /// egui::ComboBox::from_label("Crab")
    ///     .selected_text((egui::Image::new(ferris.clone()).max_height(16.0), "Ferris"))
    ///     .show_ui(ui, |ui| {
    ///         let _ = ui.selectable_label(true, (egui::Image::new(ferris).max_height(16.0), "Ferris"));
    ///     });
    /// # });
    /// ```
    #[inline]
    pub fn selected_text(mut self, selected_text: impl IntoAtoms<'a>) -> Self {
        self.selected_text = selected_text.into_atoms();
        self
    }

    /// Use the provided function to render a different [`ComboBox`] icon.
    /// Defaults to a triangle that expands when the cursor is hovering over the [`ComboBox`].
    ///
    /// For example:
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let text = "Selected text";
    /// pub fn filled_triangle(
    ///     ui: &egui::Ui,
    ///     rect: egui::Rect,
    ///     visuals: &egui::style::WidgetVisuals,
    ///     _is_open: bool,
    /// ) {
    ///     let rect = egui::Rect::from_center_size(
    ///         rect.center(),
    ///         egui::vec2(rect.width() * 0.6, rect.height() * 0.4),
    ///     );
    ///     ui.painter().add(egui::Shape::convex_polygon(
    ///         vec![rect.left_top(), rect.right_top(), rect.center_bottom()],
    ///         visuals.fg_stroke.color,
    ///         visuals.fg_stroke,
    ///     ));
    /// }
    ///
    /// egui::ComboBox::from_id_salt("my-combobox")
    ///     .selected_text(text)
    ///     .icon(filled_triangle)
    ///     .show_ui(ui, |_ui| {});
    /// # });
    /// ```
    #[inline]
    pub fn icon(mut self, icon_fn: impl FnOnce(&Ui, Rect, &WidgetVisuals, bool) + 'static) -> Self {
        self.icon = Some(Box::new(icon_fn));
        self
    }

    /// Controls the wrap mode used for the selected text.
    ///
    /// By default, [`Ui::wrap_mode`] will be used, which can be overridden with [`crate::Style::wrap_mode`].
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

    /// Controls the close behavior for the popup.
    ///
    /// By default, `PopupCloseBehavior::CloseOnClick` will be used.
    #[inline]
    pub fn close_behavior(mut self, close_behavior: PopupCloseBehavior) -> Self {
        self.close_behavior = Some(close_behavior);
        self
    }

    /// Set the style of the popup menu.
    ///
    /// Could for example be used with [`crate::containers::menu::menu_style`] to get the frame-less
    /// menu button style.
    #[inline]
    pub fn popup_style(mut self, popup_style: StyleModifier) -> Self {
        self.popup_style = popup_style;
        self
    }

    /// Show the combo box, with the given ui code for the menu contents.
    ///
    /// Returns `InnerResponse { inner: None }` if the combo box is closed.
    pub fn show_ui<R>(
        self,
        ui: &mut Ui,
        menu_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<Option<R>> {
        self.show_ui_dyn(ui, Box::new(menu_contents))
    }

    fn show_ui_dyn<'c, R>(
        self,
        ui: &mut Ui,
        menu_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<Option<R>> {
        let Self {
            id_salt,
            label,
            selected_text,
            width,
            height,
            icon,
            wrap_mode,
            close_behavior,
            popup_style,
        } = self;

        let button_id = ui.make_persistent_id(id_salt);

        combo_box_dyn(
            ui,
            button_id,
            label,
            selected_text,
            menu_contents,
            icon,
            wrap_mode,
            close_behavior,
            popup_style,
            (width, height),
        )
    }

    /// Show a list of items with the given selected index.
    ///
    ///
    /// ```
    /// # #[derive(Debug, PartialEq)]
    /// # enum Enum { First, Second, Third }
    /// # let mut selected = Enum::First;
    /// # egui::__run_test_ui(|ui| {
    /// let alternatives = ["a", "b", "c", "d"];
    /// let mut selected = 2;
    /// egui::ComboBox::from_label("Select one!").show_index(
    ///     ui,
    ///     &mut selected,
    ///     alternatives.len(),
    ///     |i| alternatives[i]
    /// );
    /// # });
    /// ```
    pub fn show_index<Text: IntoAtoms<'a>>(
        self,
        ui: &mut Ui,
        selected: &mut usize,
        len: usize,
        get: impl Fn(usize) -> Text,
    ) -> Response {
        let slf = self.selected_text(get(*selected));

        let mut changed = false;

        let mut response = slf
            .show_ui(ui, |ui| {
                for i in 0..len {
                    if ui.selectable_label(i == *selected, get(i)).clicked() {
                        *selected = i;
                        changed = true;
                    }
                }
            })
            .response;

        if changed {
            response.mark_changed();
        }
        response
    }

    /// Check if the [`ComboBox`] with the given id has its popup menu currently opened.
    pub fn is_open(ctx: &Context, id: Id) -> bool {
        Popup::is_id_open(ctx, Self::widget_to_popup_id(id))
    }

    /// Convert a [`ComboBox`] id to the id used to store it's popup state.
    fn widget_to_popup_id(widget_id: Id) -> Id {
        widget_id.with("popup")
    }
}

#[expect(clippy::too_many_arguments)]
fn combo_box_dyn<'c, R>(
    ui: &mut Ui,
    button_id: Id,
    label: Option<Atoms<'_>>,
    selected_text: Atoms<'_>,
    menu_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    icon: Option<IconPainter>,
    wrap_mode: Option<TextWrapMode>,
    close_behavior: Option<PopupCloseBehavior>,
    popup_style: StyleModifier,
    (width, height): (Option<f32>, Option<f32>),
) -> InnerResponse<Option<R>> {
    let popup_id = ComboBox::widget_to_popup_id(button_id);

    let is_popup_open = Popup::is_id_open(ui.ctx(), popup_id);

    let wrap_mode = wrap_mode.unwrap_or_else(|| ui.wrap_mode());

    let close_behavior = close_behavior.unwrap_or(PopupCloseBehavior::CloseOnClick);

    // Built from the same atoms and style as a `Button`, so the two line up when put side by side.
    let ButtonStyle {
        atom_layout: mut atom_layout_style,
    } = ui.widget_style(button_id, &Classes::default());

    // Like `ui.widget_style`, use the interaction state from the start of this pass:
    let visuals = if is_popup_open {
        ui.visuals().widgets.open
    } else if let Some(response) = ui.ctx().read_response(button_id) {
        *ui.style().interact(&response)
    } else {
        ui.visuals().widgets.inactive
    };
    if is_popup_open {
        let frame = &mut atom_layout_style.frame;
        // The style already shrank the inner margin to make room for its stroke.
        // Undo that before applying our stroke, so the size doesn't depend on hover state:
        frame.inner_margin = frame.inner_margin + Margin::from(frame.stroke.width);
        *frame = frame
            .fill(visuals.weak_bg_fill)
            .apply_stroke_and_expansion_without_layout_shift(visuals.bg_stroke, 0.0);
        atom_layout_style.text_style.color = visuals.text_color();
    }

    // The combo box will always have at least this width.
    let min_width = width.unwrap_or_else(|| ui.spacing().combo_width);
    let min_size = Vec2::new(min_width, 0.0).at_least(atom_layout_style.min_size);

    let icon = RefCell::new(icon);
    let icon_atom = Atom::paint(Vec2::splat(ui.spacing().icon_width), move |ui, args| {
        let rect = args.rect.expand(visuals.expansion);
        if let Some(icon) = icon.borrow_mut().take() {
            icon(ui, rect, &visuals, is_popup_open);
        } else {
            paint_default_icon(ui.painter(), rect, &visuals);
        }
    })
    // Grow (instead of a separate `Atom::grow()`, which would add an extra gap),
    // and stick to the right edge:
    .atom_grow(true)
    .atom_align(Align2::RIGHT_CENTER);

    let accessible_text = selected_text.text().map(String::from).unwrap_or_default();
    let button = atom_layout_style
        .apply(
            WidgetAtom::new((selected_text, icon_atom))
                .id(button_id)
                .sense(Sense::click())
                .fallback_font(TextStyle::Button)
                .wrap_mode(wrap_mode),
        )
        .min_size(min_size);

    let (button_response, label_response) = match label {
        Some(label) => {
            // The label is just another atom, next to the button.
            let label_text = label.text().map(String::from).unwrap_or_default();
            let outer_response = WidgetAtom::new((Atom::widget(button), label))
                .gap(ui.spacing().item_spacing.x)
                .show(ui)
                .response;
            outer_response
                .widget_info(|| WidgetInfo::labeled(Role::Label, ui.is_enabled(), &label_text));
            let button_response = ui
                .ctx()
                .read_response(button_id)
                .unwrap_or_else(|| outer_response.clone());
            (
                button_response,
                Some((outer_response, !label_text.is_empty())),
            )
        }
        None => (button.show(ui).response, None),
    };

    button_response.widget_info(|| {
        let mut info = WidgetInfo::new(Role::ComboBox);
        info.enabled = ui.is_enabled();
        info.current_text_value = Some(accessible_text.clone());
        info
    });

    let height = height.unwrap_or_else(|| ui.spacing().combo_height);

    let inner = Popup::menu(&button_response)
        .id(popup_id)
        .width(button_response.rect.width())
        .close_behavior(close_behavior)
        .style(popup_style)
        .show(|ui| {
            ui.set_min_width(ui.available_width());

            ScrollArea::vertical()
                .max_height(height)
                .show(ui, |ui| {
                    // Often the button is very narrow, which means this popup
                    // is also very narrow. Having wrapping on would therefore
                    // result in labels that wrap very early.
                    // Instead, we turn it off by default so that the labels
                    // expand the width of the menu.
                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                    menu_contents(ui)
                })
                .inner
        })
        .map(|r| r.inner);

    if inner.is_some() {
        ui.ctx().accesskit_node_builder(popup_id, |node| {
            node.set_role(accesskit::Role::ListBox);
        });
    }

    InnerResponse {
        inner,
        response: match label_response {
            // An empty label names nothing, so leave the name to the caller
            // (e.g. `on_hover_text`).
            Some((label_response, true)) => {
                button_response.labelled_by(label_response.id) | label_response
            }
            Some((label_response, false)) => button_response | label_response,
            None => button_response,
        },
    }
}

fn paint_default_icon(painter: &Painter, rect: Rect, visuals: &WidgetVisuals) {
    let rect = Rect::from_center_size(
        rect.center(),
        vec2(rect.width() * 0.7, rect.height() * 0.45),
    );

    // Downward pointing triangle
    // Previously, we would show an up arrow when we expected the popup to open upwards
    // (due to lack of space below the button), but this could look weird in edge cases, so this
    // feature was removed. (See https://github.com/emilk/egui/pull/5713#issuecomment-2654420245)
    painter.add(Shape::rotated_triangle(rect, 0.0, visuals.fg_stroke.color));
}
