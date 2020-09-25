use crate::{paint::PaintCmd, style::WidgetVisuals, *};

pub fn combo_box_with_label(
    ui: &mut Ui,
    label: impl Into<Label>,
    selected: impl Into<Label>,
    menu_contents: impl FnOnce(&mut Ui),
) -> Response {
    let label = label.into();
    let button_id = ui.make_unique_child_id(label.text());

    ui.horizontal(|ui| {
        let mut response = combo_box(ui, button_id, selected, menu_contents);
        response |= ui.add(label);
        response
    })
    .0
}

pub fn combo_box(
    ui: &mut Ui,
    button_id: Id,
    selected: impl Into<Label>,
    menu_contents: impl FnOnce(&mut Ui),
) -> Response {
    let popup_id = button_id.with("popup");
    let selected = selected.into();

    let button_active = ui.memory().is_popup_open(popup_id);
    let button_response = button_frame(ui, button_id, button_active, Sense::click(), |ui| {
        ui.horizontal(|ui| {
            // We don't want to change width when user selects something new
            let full_minimum_width = ui.style().spacing.slider_width;
            let icon_width = ui.style().spacing.icon_width;

            selected.ui(ui);

            let advance = full_minimum_width - icon_width - ui.child_bounds().width();
            ui.advance_cursor(advance.at_least(0.0));

            let icon_rect = ui.allocate_space(Vec2::splat(icon_width));
            let button_rect = ui.rect().expand2(ui.style().spacing.button_padding);
            let mut response = ui.interact(button_rect, button_id, Sense::click());
            response.active |= button_active;
            paint_icon(ui.painter(), icon_rect, ui.style().interact(&response));
        });
    });
    if button_response.clicked {
        ui.memory().toggle_popup(popup_id);
    }

    if ui.memory().is_popup_open(popup_id) {
        Area::new(popup_id)
            .order(Order::Foreground)
            .fixed_pos(button_response.rect.left_bottom())
            .show(ui.ctx(), |ui| {
                let frame = Frame::popup(ui.style());
                let frame_margin = frame.margin;
                frame.show(ui, |ui| {
                    ui.set_min_width(button_response.rect.width() - 2.0 * frame_margin.x);
                    ui.set_layout(Layout::justified(Direction::Vertical));
                    menu_contents(ui);
                })
            });

        if ui.input().key_pressed(Key::Escape) || ui.input().mouse.click && !button_response.clicked
        {
            ui.memory().close_popup();
        }
    }

    button_response
}

fn button_frame(
    ui: &mut Ui,
    id: Id,
    button_active: bool,
    sense: Sense,
    add_contents: impl FnOnce(&mut Ui),
) -> Response {
    let margin = ui.style().spacing.button_padding;
    let outer_rect_bounds = ui.available();
    let inner_rect = outer_rect_bounds.shrink2(margin);
    let where_to_put_background = ui.painter().add(PaintCmd::Noop);
    let mut content_ui = ui.child_ui(inner_rect);
    add_contents(&mut content_ui);

    let outer_rect = Rect::from_min_max(
        outer_rect_bounds.min,
        content_ui.child_bounds().max + margin,
    );

    let mut response = ui.interact(outer_rect, id, sense);
    response.active |= button_active;
    let style = ui.style().interact(&response);

    ui.painter().set(
        where_to_put_background,
        PaintCmd::Rect {
            rect: outer_rect,
            corner_radius: style.corner_radius,
            fill: style.bg_fill,
            stroke: style.bg_stroke,
        },
    );

    ui.allocate_space(outer_rect.size());

    response
}

fn paint_icon(painter: &Painter, rect: Rect, visuals: &WidgetVisuals) {
    let rect = Rect::from_center_size(
        rect.center(),
        vec2(rect.width() * 0.7, rect.height() * 0.45),
    );
    painter.add(PaintCmd::Path {
        points: vec![rect.left_top(), rect.right_top(), rect.center_bottom()],
        closed: true,
        fill: Default::default(),
        stroke: visuals.fg_stroke,
    });
}
