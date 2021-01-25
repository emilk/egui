use crate::{paint::Shape, style::WidgetVisuals, *};

/// A drop-down selection menu with a descriptive label.
///
/// See also [`combo_box`].
///
/// ```
/// # #[derive(Debug, PartialEq)]
/// # enum Enum { First, Second, Third }
/// # let mut selected = Enum::First;
/// # let mut ui = &mut egui::Ui::__test();
/// egui::combo_box_with_label(ui, "Select one!", format!("{:?}", selected), |ui| {
///     ui.selectable_value(&mut selected, Enum::First, "First");
///     ui.selectable_value(&mut selected, Enum::Second, "Second");
///     ui.selectable_value(&mut selected, Enum::Third, "Third");
/// });
/// ```
pub fn combo_box_with_label(
    ui: &mut Ui,
    label: impl Into<Label>,
    selected: impl Into<String>,
    menu_contents: impl FnOnce(&mut Ui),
) -> Response {
    let label = label.into();
    let button_id = ui.make_persistent_id(label.text());

    ui.horizontal(|ui| {
        let mut response = combo_box(ui, button_id, selected, menu_contents);
        response |= ui.add(label);
        response
    })
    .0
}

/// A drop-down selection menu.
///
/// See also [`combo_box_with_label`].
///
/// ```
/// # #[derive(Debug, PartialEq)]
/// # enum Enum { First, Second, Third }
/// # let mut selected = Enum::First;
/// # let mut ui = &mut egui::Ui::__test();
/// let id = ui.make_persistent_id("my_combo_box");
/// egui::combo_box(ui, id, format!("{:?}", selected), |ui| {
///     ui.selectable_value(&mut selected, Enum::First, "First");
///     ui.selectable_value(&mut selected, Enum::Second, "Second");
///     ui.selectable_value(&mut selected, Enum::Third, "Third");
/// });
/// ```
pub fn combo_box(
    ui: &mut Ui,
    button_id: Id,
    selected: impl Into<String>,
    menu_contents: impl FnOnce(&mut Ui),
) -> Response {
    let popup_id = button_id.with("popup");

    let button_active = ui.memory().is_popup_open(popup_id);
    let button_response = button_frame(ui, button_id, button_active, Sense::click(), |ui| {
        // We don't want to change width when user selects something new
        let full_minimum_width = ui.style().spacing.slider_width;
        let icon_size = Vec2::splat(ui.style().spacing.icon_width);

        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];
        let galley = font.layout_single_line(selected.into());

        let width = galley.size.x + ui.style().spacing.item_spacing.x + icon_size.x;
        let width = width.at_least(full_minimum_width);
        let height = galley.size.y.max(icon_size.y);

        let (_, rect) = ui.allocate_space(Vec2::new(width, height));
        let button_rect = ui.min_rect().expand2(ui.style().spacing.button_padding);
        let response = ui.interact(button_rect, button_id, Sense::click());
        // response.active |= button_active;

        let icon_rect = Align2::RIGHT_CENTER.align_size_within_rect(icon_size, rect);
        let visuals = ui.style().interact(&response);
        paint_icon(ui.painter(), icon_rect.expand(visuals.expansion), visuals);

        let text_rect = Align2::LEFT_CENTER.align_size_within_rect(galley.size, rect);
        ui.painter()
            .galley(text_rect.min, galley, text_style, visuals.text_color());
    });

    if button_response.clicked() {
        ui.memory().toggle_popup(popup_id);
    }
    const MAX_COMBO_HEIGHT: f32 = 128.0;
    crate::popup::popup_below_widget(ui, popup_id, &button_response, |ui| {
        ScrollArea::from_max_height(MAX_COMBO_HEIGHT).show(ui, menu_contents)
    });

    button_response
}

fn button_frame(
    ui: &mut Ui,
    id: Id,
    button_active: bool,
    sense: Sense,
    add_contents: impl FnOnce(&mut Ui),
) -> Response {
    let where_to_put_background = ui.painter().add(Shape::Noop);

    let margin = ui.style().spacing.button_padding;
    let interact_size = ui.style().spacing.interact_size;

    let mut outer_rect = ui.available_rect_before_wrap();
    outer_rect.set_height(outer_rect.height().at_least(interact_size.y));

    let inner_rect = outer_rect.shrink2(margin);
    let mut content_ui = ui.child_ui(inner_rect, Layout::left_to_right());
    add_contents(&mut content_ui);

    let mut outer_rect = content_ui.min_rect().expand2(margin);
    outer_rect.set_height(outer_rect.height().at_least(interact_size.y));

    let mut response = ui.interact(outer_rect, id, sense);
    response.is_pointer_button_down_on |= button_active;
    let visuals = ui.style().interact(&response);

    ui.painter().set(
        where_to_put_background,
        Shape::Rect {
            rect: outer_rect.expand(visuals.expansion),
            corner_radius: visuals.corner_radius,
            fill: visuals.bg_fill,
            stroke: visuals.bg_stroke,
        },
    );

    ui.advance_cursor_after_rect(outer_rect);

    response
}

fn paint_icon(painter: &Painter, rect: Rect, visuals: &WidgetVisuals) {
    let rect = Rect::from_center_size(
        rect.center(),
        vec2(rect.width() * 0.7, rect.height() * 0.45),
    );
    painter.add(Shape::closed_line(
        vec![rect.left_top(), rect.right_top(), rect.center_bottom()],
        visuals.fg_stroke,
    ));
}
