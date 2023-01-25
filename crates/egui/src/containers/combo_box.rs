use epaint::Shape;

use crate::{style::WidgetVisuals, *};

/// Indicate wether or not a popup will be shown above or below the box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AboveOrBelow {
    Above,
    Below,
}

/// A function that paints the [`ComboBox`] icon
pub type IconPainter = Box<dyn FnOnce(&Ui, Rect, &WidgetVisuals, bool, AboveOrBelow)>;

/// A drop-down selection menu with a descriptive label.
///
/// ```
/// # #[derive(Debug, PartialEq)]
/// # enum Enum { First, Second, Third }
/// # let mut selected = Enum::First;
/// # egui::__run_test_ui(|ui| {
/// egui::ComboBox::from_label("Select one!")
///     .selected_text(format!("{:?}", selected))
///     .show_ui(ui, |ui| {
///         ui.selectable_value(&mut selected, Enum::First, "First");
///         ui.selectable_value(&mut selected, Enum::Second, "Second");
///         ui.selectable_value(&mut selected, Enum::Third, "Third");
///     }
/// );
/// # });
/// ```
#[must_use = "You should call .show*"]
pub struct ComboBox {
    id_source: Id,
    label: Option<WidgetText>,
    selected_text: WidgetText,
    width: Option<f32>,
    icon: Option<IconPainter>,
    wrap_enabled: bool,
}

impl ComboBox {
    /// Create new [`ComboBox`] with id and label
    pub fn new(id_source: impl std::hash::Hash, label: impl Into<WidgetText>) -> Self {
        Self {
            id_source: Id::new(id_source),
            label: Some(label.into()),
            selected_text: Default::default(),
            width: None,
            icon: None,
            wrap_enabled: false,
        }
    }

    /// Label shown next to the combo box
    pub fn from_label(label: impl Into<WidgetText>) -> Self {
        let label = label.into();
        Self {
            id_source: Id::new(label.text()),
            label: Some(label),
            selected_text: Default::default(),
            width: None,
            icon: None,
            wrap_enabled: false,
        }
    }

    /// Without label.
    pub fn from_id_source(id_source: impl std::hash::Hash) -> Self {
        Self {
            id_source: Id::new(id_source),
            label: Default::default(),
            selected_text: Default::default(),
            width: None,
            icon: None,
            wrap_enabled: false,
        }
    }

    /// Set the outer width of the button and menu.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// What we show as the currently selected value
    pub fn selected_text(mut self, selected_text: impl Into<WidgetText>) -> Self {
        self.selected_text = selected_text.into();
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
    ///     _above_or_below: egui::AboveOrBelow,
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
    /// egui::ComboBox::from_id_source("my-combobox")
    ///     .selected_text(text)
    ///     .icon(filled_triangle)
    ///     .show_ui(ui, |_ui| {});
    /// # });
    /// ```
    pub fn icon(
        mut self,
        icon_fn: impl FnOnce(&Ui, Rect, &WidgetVisuals, bool, AboveOrBelow) + 'static,
    ) -> Self {
        self.icon = Some(Box::new(icon_fn));
        self
    }

    /// Controls whether text wrap is used for the selected text
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap_enabled = wrap;
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
            id_source,
            label,
            selected_text,
            width,
            icon,
            wrap_enabled,
        } = self;

        let button_id = ui.make_persistent_id(id_source);

        ui.horizontal(|ui| {
            let mut ir = combo_box_dyn(
                ui,
                button_id,
                selected_text,
                menu_contents,
                icon,
                wrap_enabled,
                width,
            );
            if let Some(label) = label {
                ir.response
                    .widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, label.text()));
                ir.response |= ui.label(label);
            } else {
                ir.response
                    .widget_info(|| WidgetInfo::labeled(WidgetType::ComboBox, ""));
            }
            ir
        })
        .inner
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
    ///     |i| alternatives[i].to_owned()
    /// );
    /// # });
    /// ```
    pub fn show_index(
        self,
        ui: &mut Ui,
        selected: &mut usize,
        len: usize,
        get: impl Fn(usize) -> String,
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
}

fn combo_box_dyn<'c, R>(
    ui: &mut Ui,
    button_id: Id,
    selected_text: WidgetText,
    menu_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    icon: Option<IconPainter>,
    wrap_enabled: bool,
    width: Option<f32>,
) -> InnerResponse<Option<R>> {
    let popup_id = button_id.with("popup");

    let is_popup_open = ui.memory(|m| m.is_popup_open(popup_id));

    let popup_height = ui.memory(|m| m.areas.get(popup_id).map_or(100.0, |state| state.size.y));

    let above_or_below =
        if ui.next_widget_position().y + ui.spacing().interact_size.y + popup_height
            < ui.ctx().screen_rect().bottom()
        {
            AboveOrBelow::Below
        } else {
            AboveOrBelow::Above
        };

    let margin = ui.spacing().button_padding;
    let button_response = button_frame(ui, button_id, is_popup_open, Sense::click(), |ui| {
        let icon_spacing = ui.spacing().icon_spacing;
        // We don't want to change width when user selects something new
        let full_minimum_width = if wrap_enabled {
            // Currently selected value's text will be wrapped if needed, so occupy the available width.
            ui.available_width()
        } else {
            // Occupy at least the minimum width assigned to ComboBox.
            let width = width.unwrap_or_else(|| ui.spacing().combo_width);
            width - 2.0 * margin.x
        };
        let icon_size = Vec2::splat(ui.spacing().icon_width);
        let wrap_width = if wrap_enabled {
            // Use the available width, currently selected value's text will be wrapped if exceeds this value.
            ui.available_width() - icon_spacing - icon_size.x
        } else {
            // Use all the width necessary to display the currently selected value's text.
            f32::INFINITY
        };

        let galley =
            selected_text.into_galley(ui, Some(wrap_enabled), wrap_width, TextStyle::Button);

        // The width necessary to contain the whole widget with the currently selected value's text.
        let width = if wrap_enabled {
            full_minimum_width
        } else {
            // Occupy at least the minimum width needed to contain the widget with the currently selected value's text.
            galley.size().x + icon_spacing + icon_size.x
        };

        // Case : wrap_enabled : occupy all the available width.
        // Case : !wrap_enabled : occupy at least the minimum width assigned to Slider and ComboBox,
        // increase if the currently selected value needs additional horizontal space to fully display its text (up to wrap_width (f32::INFINITY)).
        let width = width.at_least(full_minimum_width);
        let height = galley.size().y.max(icon_size.y);

        let (_, rect) = ui.allocate_space(Vec2::new(width, height));
        let button_rect = ui.min_rect().expand2(ui.spacing().button_padding);
        let response = ui.interact(button_rect, button_id, Sense::click());
        // response.active |= is_popup_open;

        if ui.is_rect_visible(rect) {
            let icon_rect = Align2::RIGHT_CENTER.align_size_within_rect(icon_size, rect);
            let visuals = if is_popup_open {
                &ui.visuals().widgets.open
            } else {
                ui.style().interact(&response)
            };

            if let Some(icon) = icon {
                icon(
                    ui,
                    icon_rect.expand(visuals.expansion),
                    visuals,
                    is_popup_open,
                    above_or_below,
                );
            } else {
                paint_default_icon(
                    ui.painter(),
                    icon_rect.expand(visuals.expansion),
                    visuals,
                    above_or_below,
                );
            }

            let text_rect = Align2::LEFT_CENTER.align_size_within_rect(galley.size(), rect);
            galley.paint_with_visuals(ui.painter(), text_rect.min, visuals);
        }
    });

    if button_response.clicked() {
        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
    }
    let inner = crate::popup::popup_above_or_below_widget(
        ui,
        popup_id,
        &button_response,
        above_or_below,
        |ui| {
            ScrollArea::vertical()
                .max_height(ui.spacing().combo_height)
                .show(ui, menu_contents)
                .inner
        },
    );

    InnerResponse {
        inner,
        response: button_response,
    }
}

fn button_frame(
    ui: &mut Ui,
    id: Id,
    is_popup_open: bool,
    sense: Sense,
    add_contents: impl FnOnce(&mut Ui),
) -> Response {
    let where_to_put_background = ui.painter().add(Shape::Noop);

    let margin = ui.spacing().button_padding;
    let interact_size = ui.spacing().interact_size;

    let mut outer_rect = ui.available_rect_before_wrap();
    outer_rect.set_height(outer_rect.height().at_least(interact_size.y));

    let inner_rect = outer_rect.shrink2(margin);
    let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
    add_contents(&mut content_ui);

    let mut outer_rect = content_ui.min_rect().expand2(margin);
    outer_rect.set_height(outer_rect.height().at_least(interact_size.y));

    let response = ui.interact(outer_rect, id, sense);

    if ui.is_rect_visible(outer_rect) {
        let visuals = if is_popup_open {
            &ui.visuals().widgets.open
        } else {
            ui.style().interact(&response)
        };

        ui.painter().set(
            where_to_put_background,
            epaint::RectShape {
                rect: outer_rect.expand(visuals.expansion),
                rounding: visuals.rounding,
                fill: visuals.weak_bg_fill,
                stroke: visuals.bg_stroke,
            },
        );
    }

    ui.advance_cursor_after_rect(outer_rect);

    response
}

fn paint_default_icon(
    painter: &Painter,
    rect: Rect,
    visuals: &WidgetVisuals,
    above_or_below: AboveOrBelow,
) {
    let rect = Rect::from_center_size(
        rect.center(),
        vec2(rect.width() * 0.7, rect.height() * 0.45),
    );

    match above_or_below {
        AboveOrBelow::Above => {
            // Upward pointing triangle
            painter.add(Shape::convex_polygon(
                vec![rect.left_bottom(), rect.right_bottom(), rect.center_top()],
                visuals.fg_stroke.color,
                Stroke::NONE,
            ));
        }
        AboveOrBelow::Below => {
            // Downward pointing triangle
            painter.add(Shape::convex_polygon(
                vec![rect.left_top(), rect.right_top(), rect.center_bottom()],
                visuals.fg_stroke.color,
                Stroke::NONE,
            ));
        }
    }
}
