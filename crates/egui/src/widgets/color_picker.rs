//! Color picker widgets.

use crate::util::fixed_cache::FixedCache;
use crate::*;
use epaint::{ecolor::*, *};

fn contrast_color(color: impl Into<Rgba>) -> Color32 {
    if color.into().intensity() < 0.5 {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

/// Number of vertices per dimension in the color sliders.
/// We need at least 6 for hues, and more for smooth 2D areas.
/// Should always be a multiple of 6 to hit the peak hues in HSV/HSL (every 60°).
const N: u32 = 6 * 6;

fn background_checkers(painter: &Painter, rect: Rect) {
    let rect = rect.shrink(0.5); // Small hack to avoid the checkers from peeking through the sides
    if !rect.is_positive() {
        return;
    }

    let dark_color = Color32::from_gray(32);
    let bright_color = Color32::from_gray(128);

    let checker_size = Vec2::splat(rect.height() / 2.0);
    let n = (rect.width() / checker_size.x).round() as u32;

    let mut mesh = Mesh::default();
    mesh.add_colored_rect(rect, dark_color);

    let mut top = true;
    for i in 0..n {
        let x = lerp(rect.left()..=rect.right(), i as f32 / (n as f32));
        let small_rect = if top {
            Rect::from_min_size(pos2(x, rect.top()), checker_size)
        } else {
            Rect::from_min_size(pos2(x, rect.center().y), checker_size)
        };
        mesh.add_colored_rect(small_rect, bright_color);
        top = !top;
    }
    painter.add(Shape::mesh(mesh));
}

/// Show a color with background checkers to demonstrate transparency (if any).
pub fn show_color(ui: &mut Ui, color: impl Into<Color32>, desired_size: Vec2) -> Response {
    show_color32(ui, color.into(), desired_size)
}

fn show_color32(ui: &mut Ui, color: Color32, desired_size: Vec2) -> Response {
    let (rect, response) = ui.allocate_at_least(desired_size, Sense::hover());
    if ui.is_rect_visible(rect) {
        show_color_at(ui.painter(), color, rect);
    }
    response
}

/// Show a color with background checkers to demonstrate transparency (if any).
pub fn show_color_at(painter: &Painter, color: Color32, rect: Rect) {
    if color.is_opaque() {
        painter.rect_filled(rect, 0.0, color);
    } else {
        // Transparent: how both the transparent and opaque versions of the color
        background_checkers(painter, rect);

        if color == Color32::TRANSPARENT {
            // There is no opaque version, so just show the background checkers
        } else {
            let left = Rect::from_min_max(rect.left_top(), rect.center_bottom());
            let right = Rect::from_min_max(rect.center_top(), rect.right_bottom());
            painter.rect_filled(left, 0.0, color);
            painter.rect_filled(right, 0.0, color.to_opaque());
        }
    }
}

fn color_button(ui: &mut Ui, color: Color32, open: bool) -> Response {
    let size = ui.spacing().interact_size;
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    response.widget_info(|| WidgetInfo::new(WidgetType::ColorButton));

    if ui.is_rect_visible(rect) {
        let visuals = if open {
            &ui.visuals().widgets.open
        } else {
            ui.style().interact(&response)
        };
        let rect = rect.expand(visuals.expansion);

        show_color_at(ui.painter(), color, rect);

        let rounding = visuals.rounding.at_most(2.0);
        ui.painter()
            .rect_stroke(rect, rounding, (2.0, visuals.bg_fill)); // fill is intentional, because default style has no border
    }

    response
}

fn color_slider_1d(ui: &mut Ui, value: &mut f32, color_at: impl Fn(f32) -> Color32) -> Response {
    #![allow(clippy::identity_op)]

    let desired_size = vec2(ui.spacing().slider_width, ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());

    if let Some(mpos) = response.interact_pointer_pos() {
        *value = remap_clamp(mpos.x, rect.left()..=rect.right(), 0.0..=1.0);
    }

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);

        background_checkers(ui.painter(), rect); // for alpha:

        {
            // fill color:
            let mut mesh = Mesh::default();
            for i in 0..=N {
                let t = i as f32 / (N as f32);
                let color = color_at(t);
                let x = lerp(rect.left()..=rect.right(), t);
                mesh.colored_vertex(pos2(x, rect.top()), color);
                mesh.colored_vertex(pos2(x, rect.bottom()), color);
                if i < N {
                    mesh.add_triangle(2 * i + 0, 2 * i + 1, 2 * i + 2);
                    mesh.add_triangle(2 * i + 1, 2 * i + 2, 2 * i + 3);
                }
            }
            ui.painter().add(Shape::mesh(mesh));
        }

        ui.painter().rect_stroke(rect, 0.0, visuals.bg_stroke); // outline

        {
            // Show where the slider is at:
            let x = lerp(rect.left()..=rect.right(), *value);
            let r = rect.height() / 4.0;
            let picked_color = color_at(*value);
            ui.painter().add(Shape::convex_polygon(
                vec![
                    pos2(x, rect.center().y),   // tip
                    pos2(x + r, rect.bottom()), // right bottom
                    pos2(x - r, rect.bottom()), // left bottom
                ],
                picked_color,
                Stroke::new(visuals.fg_stroke.width, contrast_color(picked_color)),
            ));
        }
    }

    response
}

fn color_slider_2d(
    ui: &mut Ui,
    x_value: &mut f32,
    y_value: &mut f32,
    color_at: impl Fn(f32, f32) -> Color32,
) -> Response {
    let desired_size = Vec2::splat(ui.spacing().slider_width);
    let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());

    if let Some(mpos) = response.interact_pointer_pos() {
        *x_value = remap_clamp(mpos.x, rect.left()..=rect.right(), 0.0..=1.0);
        *y_value = remap_clamp(mpos.y, rect.bottom()..=rect.top(), 0.0..=1.0);
    }

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let mut mesh = Mesh::default();

        for xi in 0..=N {
            for yi in 0..=N {
                let xt = xi as f32 / (N as f32);
                let yt = yi as f32 / (N as f32);
                let color = color_at(xt, yt);
                let x = lerp(rect.left()..=rect.right(), xt);
                let y = lerp(rect.bottom()..=rect.top(), yt);
                mesh.colored_vertex(pos2(x, y), color);

                if xi < N && yi < N {
                    let x_offset = 1;
                    let y_offset = N + 1;
                    let tl = yi * y_offset + xi;
                    mesh.add_triangle(tl, tl + x_offset, tl + y_offset);
                    mesh.add_triangle(tl + x_offset, tl + y_offset, tl + y_offset + x_offset);
                }
            }
        }
        ui.painter().add(Shape::mesh(mesh)); // fill

        ui.painter().rect_stroke(rect, 0.0, visuals.bg_stroke); // outline

        // Show where the slider is at:
        let x = lerp(rect.left()..=rect.right(), *x_value);
        let y = lerp(rect.bottom()..=rect.top(), *y_value);
        let picked_color = color_at(*x_value, *y_value);
        ui.painter().add(epaint::CircleShape {
            center: pos2(x, y),
            radius: rect.width() / 12.0,
            fill: picked_color,
            stroke: Stroke::new(visuals.fg_stroke.width, contrast_color(picked_color)),
        });
    }

    response
}

/// What options to show for alpha
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Alpha {
    // Set alpha to 1.0, and show no option for it.
    Opaque,
    // Only show normal blend options for it.
    OnlyBlend,
    // Show both blend and additive options.
    BlendOrAdditive,
}

fn color_text_ui(ui: &mut Ui, color: impl Into<Color32>, alpha: Alpha) {
    let color = color.into();
    ui.horizontal(|ui| {
        let [r, g, b, a] = color.to_array();

        if ui.button("📋").on_hover_text("Click to copy").clicked() {
            if alpha == Alpha::Opaque {
                ui.output().copied_text = format!("{}, {}, {}", r, g, b);
            } else {
                ui.output().copied_text = format!("{}, {}, {}, {}", r, g, b, a);
            }
        }

        if alpha == Alpha::Opaque {
            ui.label(format!("rgb({}, {}, {})", r, g, b))
                .on_hover_text("Red Green Blue");
        } else {
            ui.label(format!("rgba({}, {}, {}, {})", r, g, b, a))
                .on_hover_text("Red Green Blue with premultiplied Alpha");
        }
    });
}

fn color_picker_hsvag_2d(ui: &mut Ui, hsva: &mut HsvaGamma, alpha: Alpha) {
    let current_color_size = vec2(ui.spacing().slider_width, ui.spacing().interact_size.y);
    show_color(ui, *hsva, current_color_size).on_hover_text("Selected color");

    color_text_ui(ui, *hsva, alpha);

    if alpha == Alpha::BlendOrAdditive {
        // We signal additive blending by storing a negative alpha (a bit ironic).
        let a = &mut hsva.a;
        let mut additive = *a < 0.0;
        ui.horizontal(|ui| {
            ui.label("Blending:");
            ui.radio_value(&mut additive, false, "Normal");
            ui.radio_value(&mut additive, true, "Additive");

            if additive {
                *a = -a.abs();
            }

            if !additive {
                *a = a.abs();
            }
        });
    }
    let additive = hsva.a < 0.0;

    let opaque = HsvaGamma { a: 1.0, ..*hsva };

    if alpha == Alpha::Opaque {
        hsva.a = 1.0;
    } else {
        let a = &mut hsva.a;

        if alpha == Alpha::OnlyBlend {
            if *a < 0.0 {
                *a = 0.5; // was additive, but isn't allowed to be
            }
            color_slider_1d(ui, a, |a| HsvaGamma { a, ..opaque }.into()).on_hover_text("Alpha");
        } else if !additive {
            color_slider_1d(ui, a, |a| HsvaGamma { a, ..opaque }.into()).on_hover_text("Alpha");
        }
    }

    let HsvaGamma { h, s, v, a: _ } = hsva;

    color_slider_1d(ui, h, |h| {
        HsvaGamma {
            h,
            s: 1.0,
            v: 1.0,
            a: 1.0,
        }
        .into()
    })
    .on_hover_text("Hue");

    if false {
        color_slider_1d(ui, s, |s| HsvaGamma { s, ..opaque }.into()).on_hover_text("Saturation");
    }

    if false {
        color_slider_1d(ui, v, |v| HsvaGamma { v, ..opaque }.into()).on_hover_text("Value");
    }

    color_slider_2d(ui, v, s, |v, s| HsvaGamma { s, v, ..opaque }.into());
}

//// Shows a color picker where the user can change the given [`Hsva`] color.
///
/// Returns `true` on change.
pub fn color_picker_hsva_2d(ui: &mut Ui, hsva: &mut Hsva, alpha: Alpha) -> bool {
    let mut hsvag = HsvaGamma::from(*hsva);
    ui.vertical(|ui| {
        color_picker_hsvag_2d(ui, &mut hsvag, alpha);
    });
    let new_hasva = Hsva::from(hsvag);
    if *hsva == new_hasva {
        false
    } else {
        *hsva = new_hasva;
        true
    }
}

/// Shows a color picker where the user can change the given [`Color32`] color.
///
/// Returns `true` on change.
pub fn color_picker_color32(ui: &mut Ui, srgba: &mut Color32, alpha: Alpha) -> bool {
    let mut hsva = color_cache_get(ui.ctx(), *srgba);
    let changed = color_picker_hsva_2d(ui, &mut hsva, alpha);
    *srgba = Color32::from(hsva);
    color_cache_set(ui.ctx(), *srgba, hsva);
    changed
}

pub fn color_edit_button_hsva(ui: &mut Ui, hsva: &mut Hsva, alpha: Alpha) -> Response {
    let popup_id = ui.auto_id_with("popup");
    let open = ui.memory().is_popup_open(popup_id);
    let mut button_response = color_button(ui, (*hsva).into(), open);
    if ui.style().explanation_tooltips {
        button_response = button_response.on_hover_text("Click to edit color");
    }

    if button_response.clicked() {
        ui.memory().toggle_popup(popup_id);
    }

    const COLOR_SLIDER_WIDTH: f32 = 210.0;

    // TODO(emilk): make it easier to show a temporary popup that closes when you click outside it
    if ui.memory().is_popup_open(popup_id) {
        let area_response = Area::new(popup_id)
            .order(Order::Foreground)
            .fixed_pos(button_response.rect.max)
            .constrain(true)
            .show(ui.ctx(), |ui| {
                ui.spacing_mut().slider_width = COLOR_SLIDER_WIDTH;
                Frame::popup(ui.style()).show(ui, |ui| {
                    if color_picker_hsva_2d(ui, hsva, alpha) {
                        button_response.mark_changed();
                    }
                });
            })
            .response;

        if !button_response.clicked()
            && (ui.input().key_pressed(Key::Escape) || area_response.clicked_elsewhere())
        {
            ui.memory().close_popup();
        }
    }

    button_response
}

/// Shows a button with the given color.
/// If the user clicks the button, a full color picker is shown.
pub fn color_edit_button_srgba(ui: &mut Ui, srgba: &mut Color32, alpha: Alpha) -> Response {
    let mut hsva = color_cache_get(ui.ctx(), *srgba);
    let response = color_edit_button_hsva(ui, &mut hsva, alpha);
    *srgba = Color32::from(hsva);
    color_cache_set(ui.ctx(), *srgba, hsva);
    response
}

/// Shows a button with the given color.
/// If the user clicks the button, a full color picker is shown.
/// The given color is in `sRGB` space.
pub fn color_edit_button_srgb(ui: &mut Ui, srgb: &mut [u8; 3]) -> Response {
    let mut srgba = Color32::from_rgb(srgb[0], srgb[1], srgb[2]);
    let response = color_edit_button_srgba(ui, &mut srgba, Alpha::Opaque);
    srgb[0] = srgba[0];
    srgb[1] = srgba[1];
    srgb[2] = srgba[2];
    response
}

/// Shows a button with the given color.
/// If the user clicks the button, a full color picker is shown.
pub fn color_edit_button_rgba(ui: &mut Ui, rgba: &mut Rgba, alpha: Alpha) -> Response {
    let mut hsva = color_cache_get(ui.ctx(), *rgba);
    let response = color_edit_button_hsva(ui, &mut hsva, alpha);
    *rgba = Rgba::from(hsva);
    color_cache_set(ui.ctx(), *rgba, hsva);
    response
}

/// Shows a button with the given color.
/// If the user clicks the button, a full color picker is shown.
pub fn color_edit_button_rgb(ui: &mut Ui, rgb: &mut [f32; 3]) -> Response {
    let mut rgba = Rgba::from_rgb(rgb[0], rgb[1], rgb[2]);
    let response = color_edit_button_rgba(ui, &mut rgba, Alpha::Opaque);
    rgb[0] = rgba[0];
    rgb[1] = rgba[1];
    rgb[2] = rgba[2];
    response
}

// To ensure we keep hue slider when `srgba` is gray we store the full [`Hsva`] in a cache:
fn color_cache_get(ctx: &Context, rgba: impl Into<Rgba>) -> Hsva {
    let rgba = rgba.into();
    use_color_cache(ctx, |cc| cc.get(&rgba).copied()).unwrap_or_else(|| Hsva::from(rgba))
}

// To ensure we keep hue slider when `srgba` is gray we store the full [`Hsva`] in a cache:
fn color_cache_set(ctx: &Context, rgba: impl Into<Rgba>, hsva: Hsva) {
    let rgba = rgba.into();
    use_color_cache(ctx, |cc| cc.set(rgba, hsva));
}

// To ensure we keep hue slider when `srgba` is gray we store the full [`Hsva`] in a cache:
fn use_color_cache<R>(ctx: &Context, f: impl FnOnce(&mut FixedCache<Rgba, Hsva>) -> R) -> R {
    f(ctx.data().get_temp_mut_or_default(Id::null()))
}
