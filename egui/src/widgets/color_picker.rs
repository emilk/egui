use crate::{
    paint::{color::*, *},
    *,
};

fn contrast_color(color: impl Into<Rgba>) -> Srgba {
    if color.into().intensity() < 0.5 {
        color::WHITE
    } else {
        color::BLACK
    }
}

/// Number of vertices per dimension in the color sliders.
/// We need at least 6 for hues, and more for smooth 2D areas.
/// Should always be a multiple of 6 to hit the peak hues in HSV/HSL (every 60°).
const N: u32 = 6 * 3;

fn background_checkers(painter: &Painter, rect: Rect) {
    let mut top_color = Srgba::gray(128);
    let mut bottom_color = Srgba::gray(32);
    let checker_size = Vec2::splat(rect.height() / 2.0);
    let n = (rect.width() / checker_size.x).round() as u32;

    let mut triangles = Triangles::default();
    for i in 0..n {
        let x = lerp(rect.left()..=rect.right(), i as f32 / (n as f32));
        triangles.add_colored_rect(
            Rect::from_min_size(pos2(x, rect.top()), checker_size),
            top_color,
        );
        triangles.add_colored_rect(
            Rect::from_min_size(pos2(x, rect.center().y), checker_size),
            bottom_color,
        );
        std::mem::swap(&mut top_color, &mut bottom_color);
    }
    painter.add(PaintCmd::Triangles(triangles));
}

fn show_color(ui: &mut Ui, color: Srgba, desired_size: Vec2) -> Rect {
    let rect = ui.allocate_space(desired_size);
    background_checkers(ui.painter(), rect);
    ui.painter().add(PaintCmd::Rect {
        rect,
        corner_radius: 2.0,
        fill: color,
        stroke: Stroke::new(3.0, color.to_opaque()),
    });
    rect
}

fn color_button(ui: &mut Ui, color: Srgba) -> Response {
    let desired_size = Vec2::splat(ui.style().spacing.clickable_diameter);
    let rect = ui.allocate_space(desired_size);
    let rect = rect.expand2(ui.style().spacing.button_expand);
    let id = ui.make_position_id();
    let response = ui.interact(rect, id, Sense::click());
    let visuals = ui.style().interact(&response);
    background_checkers(ui.painter(), rect);
    ui.painter().add(PaintCmd::Rect {
        rect,
        corner_radius: visuals.corner_radius.min(2.0),
        fill: color,
        stroke: visuals.fg_stroke,
    });
    response
}

fn color_slider_1d(ui: &mut Ui, value: &mut f32, color_at: impl Fn(f32) -> Srgba) -> Response {
    #![allow(clippy::identity_op)]

    let desired_size = vec2(
        ui.style().spacing.slider_width,
        ui.style().spacing.clickable_diameter * 2.0,
    );
    let rect = ui.allocate_space(desired_size);

    let id = ui.make_position_id();
    let response = ui.interact(rect, id, Sense::click_and_drag());
    if response.active {
        if let Some(mpos) = ui.input().mouse.pos {
            *value = remap_clamp(mpos.x, rect.left()..=rect.right(), 0.0..=1.0);
        }
    }

    let visuals = ui.style().interact(&response);

    background_checkers(ui.painter(), rect); // for alpha:

    {
        // fill color:
        let mut triangles = Triangles::default();
        for i in 0..=N {
            let t = i as f32 / (N as f32);
            let color = color_at(t);
            let x = lerp(rect.left()..=rect.right(), t);
            triangles.colored_vertex(pos2(x, rect.top()), color);
            triangles.colored_vertex(pos2(x, rect.bottom()), color);
            if i < N {
                triangles.add_triangle(2 * i + 0, 2 * i + 1, 2 * i + 2);
                triangles.add_triangle(2 * i + 1, 2 * i + 2, 2 * i + 3);
            }
        }
        ui.painter().add(PaintCmd::Triangles(triangles));
    }

    ui.painter().rect_stroke(rect, 0.0, visuals.bg_stroke); // outline

    {
        // Show where the slider is at:
        let x = lerp(rect.left()..=rect.right(), *value);
        let r = rect.height() / 4.0;
        let picked_color = color_at(*value);
        ui.painter().add(PaintCmd::Path {
            points: vec![
                pos2(x - r, rect.bottom()),
                pos2(x + r, rect.bottom()),
                pos2(x, rect.center().y),
            ],
            closed: true,
            fill: picked_color,
            stroke: Stroke::new(visuals.fg_stroke.width, contrast_color(picked_color)),
        });
    }

    response
}

fn color_slider_2d(
    ui: &mut Ui,
    x_value: &mut f32,
    y_value: &mut f32,
    color_at: impl Fn(f32, f32) -> Srgba,
) -> Response {
    let desired_size = Vec2::splat(ui.style().spacing.slider_width);
    let rect = ui.allocate_space(desired_size);

    let id = ui.make_position_id();
    let response = ui.interact(rect, id, Sense::click_and_drag());
    if response.active {
        if let Some(mpos) = ui.input().mouse.pos {
            *x_value = remap_clamp(mpos.x, rect.left()..=rect.right(), 0.0..=1.0);
            *y_value = remap_clamp(mpos.y, rect.bottom()..=rect.top(), 0.0..=1.0);
        }
    }

    let visuals = ui.style().interact(&response);
    let mut triangles = Triangles::default();

    for xi in 0..=N {
        for yi in 0..=N {
            let xt = xi as f32 / (N as f32);
            let yt = yi as f32 / (N as f32);
            let color = color_at(xt, yt);
            let x = lerp(rect.left()..=rect.right(), xt);
            let y = lerp(rect.bottom()..=rect.top(), yt);
            triangles.colored_vertex(pos2(x, y), color);

            if xi < N && yi < N {
                let x_offset = 1;
                let y_offset = N + 1;
                let tl = yi * y_offset + xi;
                triangles.add_triangle(tl, tl + x_offset, tl + y_offset);
                triangles.add_triangle(tl + x_offset, tl + y_offset, tl + y_offset + x_offset);
            }
        }
    }
    ui.painter().add(PaintCmd::Triangles(triangles)); // fill

    ui.painter().rect_stroke(rect, 0.0, visuals.bg_stroke); // outline

    // Show where the slider is at:
    let x = lerp(rect.left()..=rect.right(), *x_value);
    let y = lerp(rect.bottom()..=rect.top(), *y_value);
    let picked_color = color_at(*x_value, *y_value);
    ui.painter().add(PaintCmd::Circle {
        center: pos2(x, y),
        radius: rect.width() / 12.0,
        fill: picked_color,
        stroke: Stroke::new(visuals.fg_stroke.width, contrast_color(picked_color)),
    });

    response
}

fn color_picker_hsva_2d(ui: &mut Ui, hsva: &mut Hsva) {
    ui.vertical_centered(|ui| {
        let current_color_size = vec2(
            ui.style().spacing.slider_width,
            ui.style().spacing.clickable_diameter * 2.0,
        );
        let current_color_rect = show_color(ui, (*hsva).into(), current_color_size);
        if ui.hovered(current_color_rect) {
            show_tooltip_text(ui.ctx(), "Current color");
        }

        let opaque = Hsva { a: 1.0, ..*hsva };
        let Hsva { h, s, v, a } = hsva;
        color_slider_2d(ui, h, s, |h, s| Hsva::new(h, s, 1.0, 1.0).into())
            .tooltip_text("Hue - Saturation");
        color_slider_2d(ui, v, s, |v, s| Hsva { v, s, ..opaque }.into())
            .tooltip_text("Value - Saturation");
        ui.label("Alpha:");
        color_slider_1d(ui, a, |a| Hsva { a, ..opaque }.into()).tooltip_text("Alpha");
    });
}

fn color_picker_hsva(ui: &mut Ui, hsva: &mut Hsva) {
    let id = ui.make_position_id().with("foo");
    let button_response = color_button(ui, (*hsva).into()).tooltip_text("Click to edit color");

    if button_response.clicked {
        ui.memory().popup = Some(id);
    }
    // TODO: make it easier to show a temporary popup that closes when you click outside it
    if ui.memory().popup == Some(id) {
        let area_response = Area::new(id)
            .order(Order::Foreground)
            .default_pos(button_response.rect.max)
            .show(ui.ctx(), |ui| {
                Frame::popup(ui.style()).show(ui, |ui| {
                    color_picker_hsva_2d(ui, hsva);
                })
            });

        if !button_response.clicked {
            let clicked_outside = ui.input().mouse.click && !area_response.hovered;
            if clicked_outside || ui.input().key_pressed(Key::Escape) {
                ui.memory().popup = None;
            }
        }
    }
}

// TODO: return Response so user can show a tooltip
/// Shows a button with the given color.
/// If the user clicks the button, a full color picker is shown.
pub fn color_edit_button(ui: &mut Ui, srgba: &mut Srgba) {
    // To ensure we keep hue slider when `srgba` is grey we store the
    // full `Hsva` in a cache:

    let mut hsva = ui
        .ctx()
        .memory()
        .color_cache
        .get(srgba)
        .cloned()
        .unwrap_or_else(|| Hsva::from(*srgba));

    color_picker_hsva(ui, &mut hsva);

    *srgba = Srgba::from(hsva);

    ui.ctx().memory().color_cache.set(*srgba, hsva);
}
