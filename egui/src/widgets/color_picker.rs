//! Color picker widgets.

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
    painter.add(PaintCmd::triangles(triangles));
}

pub fn show_color(ui: &mut Ui, color: impl Into<Srgba>, desired_size: Vec2) -> Response {
    show_srgba(ui, color.into(), desired_size)
}

fn show_srgba(ui: &mut Ui, srgba: Srgba, desired_size: Vec2) -> Response {
    let response = ui.allocate_response(desired_size, Sense::hover());
    background_checkers(ui.painter(), response.rect);
    ui.painter().add(PaintCmd::Rect {
        rect: response.rect,
        corner_radius: 2.0,
        fill: srgba,
        stroke: Stroke::new(3.0, srgba.to_opaque()),
    });
    response
}

fn color_button(ui: &mut Ui, color: Srgba) -> Response {
    let desired_size = ui.style().spacing.interact_size;
    let response = ui.allocate_response(desired_size, Sense::click());
    let visuals = ui.style().interact(&response);
    background_checkers(ui.painter(), response.rect);
    ui.painter().add(PaintCmd::Rect {
        rect: response.rect,
        corner_radius: visuals.corner_radius.at_most(2.0),
        fill: color,
        stroke: visuals.fg_stroke,
    });
    response
}

fn color_slider_1d(ui: &mut Ui, value: &mut f32, color_at: impl Fn(f32) -> Srgba) -> Response {
    #![allow(clippy::identity_op)]

    let desired_size = vec2(
        ui.style().spacing.slider_width,
        ui.style().spacing.interact_size.y * 2.0,
    );
    let response = ui.allocate_response(desired_size, Sense::click_and_drag());
    let rect = response.rect;

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
        ui.painter().add(PaintCmd::triangles(triangles));
    }

    ui.painter().rect_stroke(rect, 0.0, visuals.bg_stroke); // outline

    {
        // Show where the slider is at:
        let x = lerp(rect.left()..=rect.right(), *value);
        let r = rect.height() / 4.0;
        let picked_color = color_at(*value);
        ui.painter().add(PaintCmd::polygon(
            vec![
                pos2(x - r, rect.bottom()),
                pos2(x + r, rect.bottom()),
                pos2(x, rect.center().y),
            ],
            picked_color,
            Stroke::new(visuals.fg_stroke.width, contrast_color(picked_color)),
        ));
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
    let response = ui.allocate_response(desired_size, Sense::click_and_drag());
    let rect = response.rect;

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
    ui.painter().add(PaintCmd::triangles(triangles)); // fill

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

fn color_picker_hsvag_2d(ui: &mut Ui, hsva: &mut HsvaGamma) {
    ui.vertical(|ui| {
        let current_color_size = vec2(
            ui.style().spacing.slider_width,
            ui.style().spacing.interact_size.y * 2.0,
        );

        show_color(ui, *hsva, current_color_size).on_hover_text("Current color");

        show_color(ui, HsvaGamma { a: 1.0, ..*hsva }, current_color_size)
            .on_hover_text("Current color (opaque)");

        let opaque = HsvaGamma { a: 1.0, ..*hsva };
        let HsvaGamma { h, s, v, a } = hsva;
        color_slider_2d(ui, h, s, |h, s| HsvaGamma::new(h, s, 1.0, 1.0).into())
            .on_hover_text("Hue - Saturation");
        color_slider_2d(ui, v, s, |v, s| HsvaGamma { v, s, ..opaque }.into())
            .on_hover_text("Value - Saturation");
        color_slider_1d(ui, h, |h| HsvaGamma { h, ..opaque }.into()).on_hover_text("Hue");
        color_slider_1d(ui, s, |s| HsvaGamma { s, ..opaque }.into()).on_hover_text("Saturation");
        color_slider_1d(ui, v, |v| HsvaGamma { v, ..opaque }.into()).on_hover_text("Value");
        color_slider_1d(ui, a, |a| HsvaGamma { a, ..opaque }.into()).on_hover_text("Alpha");
    });
}

fn color_picker_hsva_2d(ui: &mut Ui, hsva: &mut Hsva) {
    let mut hsvag = HsvaGamma::from(*hsva);
    color_picker_hsvag_2d(ui, &mut hsvag);
    *hsva = Hsva::from(hsvag);
}

pub fn color_edit_button_hsva(ui: &mut Ui, hsva: &mut Hsva) -> Response {
    let pupup_id = ui.auto_id_with("popup");
    let button_response = color_button(ui, (*hsva).into()).on_hover_text("Click to edit color");

    if button_response.clicked {
        ui.memory().toggle_popup(pupup_id);
    }
    // TODO: make it easier to show a temporary popup that closes when you click outside it
    if ui.memory().is_popup_open(pupup_id) {
        let area_response = Area::new(pupup_id)
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
                ui.memory().close_popup();
            }
        }
    }

    button_response
}

/// Shows a button with the given color.
/// If the user clicks the button, a full color picker is shown.
pub fn color_edit_button_srgba(ui: &mut Ui, srgba: &mut Srgba) -> Response {
    // To ensure we keep hue slider when `srgba` is grey we store the
    // full `Hsva` in a cache:

    let mut hsva = ui
        .ctx()
        .memory()
        .color_cache
        .get(srgba)
        .cloned()
        .unwrap_or_else(|| Hsva::from(*srgba));

    let response = color_edit_button_hsva(ui, &mut hsva);

    *srgba = Srgba::from(hsva);

    ui.ctx().memory().color_cache.set(*srgba, hsva);

    response
}

// ----------------------------------------------------------------------------

/// Like Hsva but with the `v` (value/brightness) being gamma corrected
/// so that it is perceptually even in sliders.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct HsvaGamma {
    /// hue 0-1
    pub h: f32,
    /// saturation 0-1
    pub s: f32,
    /// value 0-1, in gamma-space (perceptually even)
    pub v: f32,
    /// alpha 0-1
    pub a: f32,
}

impl HsvaGamma {
    pub fn new(h: f32, s: f32, v: f32, a: f32) -> Self {
        Self { h, s, v, a }
    }
}

// const GAMMA: f32 = 2.2;

impl From<HsvaGamma> for Rgba {
    fn from(hsvag: HsvaGamma) -> Rgba {
        Hsva::from(hsvag).into()
    }
}

impl From<HsvaGamma> for Srgba {
    fn from(hsvag: HsvaGamma) -> Srgba {
        Rgba::from(hsvag).into()
    }
}

impl From<HsvaGamma> for Hsva {
    fn from(hsvag: HsvaGamma) -> Hsva {
        let HsvaGamma { h, s, v, a } = hsvag;
        Hsva {
            h,
            s,
            v: linear_from_srgb(v),
            a,
        }
    }
}

impl From<Hsva> for HsvaGamma {
    fn from(hsva: Hsva) -> HsvaGamma {
        let Hsva { h, s, v, a } = hsva;
        HsvaGamma {
            h,
            s,
            v: srgb_from_linear(v),
            a,
        }
    }
}

/// [0, 1] -> [0, 1]
fn linear_from_srgb(s: f32) -> f32 {
    if s < 0.0 {
        -linear_from_srgb(-s)
    } else if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// [0, 1] -> [0, 1]
fn srgb_from_linear(l: f32) -> f32 {
    if l < 0.0 {
        -srgb_from_linear(-l)
    } else if l <= 0.0031308 {
        12.92 * l
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    }
}
