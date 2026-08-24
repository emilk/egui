//! Color picker widgets.

use crate::util::fixed_cache::FixedCache;
use crate::{
    Context, DragValue, Id, Painter, Popup, PopupCloseBehavior, Response, Sense, Ui, Widget as _,
    WidgetInfo, WidgetType, epaint, lerp, remap_clamp,
};
use epaint::{
    ColorImage, Rect, RectShape, RoundedRect, Shape, Stroke, StrokeKind, Vec2,
    ecolor::{Color32, Hsva, HsvaGamma, Rgba},
    pos2,
    textures::TextureOptions,
    vec2,
};

fn contrast_color(color: impl Into<Rgba>) -> Color32 {
    if color.into().intensity() < 0.5 {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

/// Resolution of the color slider gradients: the textures have `N + 1` texels per dimension.
/// We need at least 6 for hues, and more for smooth 2D areas.
/// Should always be a multiple of 6 to hit the peak hues in HSV/HSL (every 60°).
const N: u32 = 6 * 6;

fn background_checkers(painter: &Painter, bounds: RoundedRect) {
    // Shrink slightly, so the dark checkers don't peek through
    // the antialiased edge of the color painted on top:
    let pixel_width = 1.0 / painter.ctx().pixels_per_point();
    let (rect, corner_radius) = bounds.shrink(pixel_width).into_parts();

    if !rect.is_positive() || !rect.is_finite() {
        return;
    }

    let dark_color = Color32::from_gray(32);
    let bright_color = Color32::from_gray(128);

    // A single repeating 2x2 checker tile, stretched so that
    // each checker is half the rect height, like before:
    let image = ColorImage::new(
        [2, 2],
        vec![bright_color, dark_color, dark_color, bright_color],
    );
    let texture = painter.ctx().load_texture_cached(
        "color_picker_checkers",
        Id::new("color_picker_checkers"),
        image,
        TextureOptions::NEAREST_REPEAT,
    );

    // One tile (one uv unit) covers 2x2 checkers, i.e. the full rect height:
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(rect.width() / rect.height(), 1.0));
    painter
        .add(RectShape::filled(rect, corner_radius, Color32::WHITE).with_texture(texture.id(), uv));
}

/// Paint a gradient image over the given rounded rect, with an outline.
fn paint_gradient(ui: &Ui, id: Id, bounds: RoundedRect, outline: Stroke, image: ColorImage) {
    let (rect, corner_radius) = bounds.into_parts();
    let [width, height] = image.size;
    let texture = ui
        .ctx()
        .load_texture_cached("color_gradient", id, image, TextureOptions::LINEAR);

    // Inset the uv by half a texel, so the edges sample the exact edge colors:
    let inset = vec2(0.5 / width as f32, 0.5 / height as f32);
    let uv = Rect::from_min_max(inset.to_pos2(), pos2(1.0 - inset.x, 1.0 - inset.y));

    ui.painter()
        .add(RectShape::filled(rect, corner_radius, Color32::WHITE).with_texture(texture.id(), uv));

    // The outline must be a separate shape:
    // a textured `RectShape` cannot have a stroke,
    // since the stroke vertices would sample the same texture.
    ui.painter()
        .rect_stroke(rect, corner_radius, outline, StrokeKind::Inside);
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
        background_checkers(painter, rect.into());

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

/// Show a color with background checkers to demonstrate transparency (if any).
fn show_srgba_unmultiplied(ui: &mut Ui, srgba: [u8; 4], desired_size: Vec2) -> Response {
    let (rect, response) = ui.allocate_at_least(desired_size, Sense::hover());
    if ui.is_rect_visible(rect) {
        let corner_radius = ui.visuals().widgets.noninteractive.corner_radius;
        show_srgba_unmultiplied_at(ui.painter(), srgba, RoundedRect::new(rect, corner_radius));
    }
    response
}

/// Show a color with background checkers to demonstrate transparency (if any).
fn show_srgba_unmultiplied_at(painter: &Painter, [r, g, b, a]: [u8; 4], bounds: RoundedRect) {
    let (rect, corner_radius) = bounds.into_parts();
    if a == 255 {
        painter.rect_filled(rect, corner_radius, Color32::from_rgb(r, g, b));
    } else {
        let left = Rect::from_min_max(rect.left_top(), rect.center_bottom());
        let right = Rect::from_min_max(rect.center_top(), rect.right_bottom());

        // Clamp to what the half-width rects can express,
        // so the checkers and both halves all round their corners the same:
        let corner_radius = corner_radius.at_most(0.5 * left.size().min_elem());

        background_checkers(painter, RoundedRect::new(rect, corner_radius));

        let left_corner_radius = corner_radius.with_east(0.0);
        let right_corner_radius = corner_radius.with_west(0.0);
        painter.rect_filled(
            left,
            left_corner_radius,
            Color32::from_rgba_unmultiplied(r, g, b, a),
        );
        painter.rect_filled(right, right_corner_radius, Color32::from_rgb(r, g, b));
    }
}

fn color_button(ui: &mut Ui, srgba: [u8; 4], open: bool) -> Response {
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

        let stroke_width = 1.0;
        let corner_radius = visuals.corner_radius;
        show_srgba_unmultiplied_at(
            ui.painter(),
            srgba,
            // Shrink both the rect and the corner radius,
            // so the fill arcs stay concentric with the inside stroke:
            RoundedRect::new(rect, corner_radius).shrink(stroke_width),
        );

        ui.painter().rect_stroke(
            rect,
            corner_radius,
            (stroke_width, visuals.bg_fill), // Using fill for stroke is intentional, because default style has no border
            StrokeKind::Inside,
        );
    }

    response
}

fn color_slider_1d(ui: &mut Ui, value: &mut f32, color_at: impl Fn(f32) -> Color32) -> Response {
    let desired_size = vec2(ui.spacing().slider_width, ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());

    if let Some(mpos) = response.interact_pointer_pos() {
        *value = remap_clamp(mpos.x, rect.left()..=rect.right(), 0.0..=1.0);
    }

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let corner_radius = visuals.corner_radius;
        let bounds = RoundedRect::new(rect, corner_radius);

        background_checkers(ui.painter(), bounds); // for alpha:

        {
            // fill color gradient:
            let width = N as usize + 1;
            let pixels = (0..width).map(|i| color_at(i as f32 / N as f32)).collect();
            let image = ColorImage::new([width, 1], pixels);
            paint_gradient(
                ui,
                response.id.with("gradient"),
                bounds,
                visuals.bg_stroke,
                image,
            );
        }

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

/// # Arguments
/// * `x_value` - X axis, either saturation or value (0.0-1.0).
/// * `y_value` - Y axis, either saturation or value (0.0-1.0).
/// * `color_at` - A function that dictates how the mix of saturation and value will be displayed in the 2d slider.
///
/// e.g.: `|x_value, y_value| HsvaGamma { h: 1.0, s: x_value, v: y_value, a: 1.0 }.into()` displays the colors as follows:
/// * top-left: white `[s: 0.0, v: 1.0]`
/// * top-right: fully saturated color `[s: 1.0, v: 1.0]`
/// * bottom-right: black `[s: 0.0, v: 1.0].`
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
        let corner_radius = visuals.corner_radius;

        {
            // fill color gradient:
            let width = N as usize + 1;
            let mut pixels = Vec::with_capacity(width * width);
            for yi in 0..=N {
                let yt = 1.0 - yi as f32 / (N as f32); // texel rows go from top to bottom
                for xi in 0..=N {
                    let xt = xi as f32 / (N as f32);
                    pixels.push(color_at(xt, yt));
                }
            }
            let image = ColorImage::new([width, width], pixels);
            paint_gradient(
                ui,
                response.id.with("gradient"),
                RoundedRect::new(rect, corner_radius),
                visuals.bg_stroke,
                image,
            );
        }

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

/// We use a negative alpha for additive colors within this file (a bit ironic).
///
/// We use alpha=0 to mean "transparent".
fn is_additive_alpha(a: f32) -> bool {
    a < 0.0
}

/// What options to show for alpha
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Alpha {
    /// Set alpha to 1.0, and show no option for it.
    Opaque,

    /// Only show normal blend options for alpha.
    OnlyBlend,

    /// Show both blend and additive options.
    BlendOrAdditive,
}

fn color_picker_hsvag_2d(ui: &mut Ui, hsvag: &mut HsvaGamma, alpha: Alpha) {
    use crate::style::NumericColorSpace;

    let alpha_control = if is_additive_alpha(hsvag.a) {
        Alpha::Opaque // no alpha control for additive colors
    } else {
        alpha
    };

    match ui.style().visuals.numeric_color_space {
        NumericColorSpace::GammaByte => {
            let mut srgba_unmultiplied = Hsva::from(*hsvag).to_srgba_unmultiplied();
            // Only update if changed to avoid rounding issues.
            if srgba_edit_ui(ui, &mut srgba_unmultiplied, alpha_control) {
                if is_additive_alpha(hsvag.a) {
                    let alpha = hsvag.a;

                    *hsvag = HsvaGamma::from(Hsva::from_additive_srgb([
                        srgba_unmultiplied[0],
                        srgba_unmultiplied[1],
                        srgba_unmultiplied[2],
                    ]));

                    // Don't edit the alpha:
                    hsvag.a = alpha;
                } else {
                    // Normal blending.
                    *hsvag = HsvaGamma::from(Hsva::from_srgba_unmultiplied(srgba_unmultiplied));
                }
            }
        }

        NumericColorSpace::Linear => {
            let mut rgba_unmultiplied = Hsva::from(*hsvag).to_rgba_unmultiplied();
            // Only update if changed to avoid rounding issues.
            if rgba_edit_ui(ui, &mut rgba_unmultiplied, alpha_control) {
                if is_additive_alpha(hsvag.a) {
                    let alpha = hsvag.a;

                    *hsvag = HsvaGamma::from(Hsva::from_rgb([
                        rgba_unmultiplied[0],
                        rgba_unmultiplied[1],
                        rgba_unmultiplied[2],
                    ]));

                    // Don't edit the alpha:
                    hsvag.a = alpha;
                } else {
                    // Normal blending.
                    *hsvag = HsvaGamma::from(Hsva::from_rgba_unmultiplied(
                        rgba_unmultiplied[0],
                        rgba_unmultiplied[1],
                        rgba_unmultiplied[2],
                        rgba_unmultiplied[3],
                    ));
                }
            }
        }
    }

    let current_color_size = vec2(ui.spacing().slider_width, ui.spacing().interact_size.y);
    show_srgba_unmultiplied(
        ui,
        Hsva::from(*hsvag).to_srgba_unmultiplied(),
        current_color_size,
    )
    .on_hover_text("Selected color");

    if alpha == Alpha::BlendOrAdditive {
        let a = &mut hsvag.a;
        let mut additive = is_additive_alpha(*a);
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

    let opaque = HsvaGamma { a: 1.0, ..*hsvag };

    let HsvaGamma { h, s, v, a: _ } = hsvag;

    if false {
        color_slider_1d(ui, s, |s| HsvaGamma { s, ..opaque }.into()).on_hover_text("Saturation");
    }

    if false {
        color_slider_1d(ui, v, |v| HsvaGamma { v, ..opaque }.into()).on_hover_text("Value");
    }

    color_slider_2d(ui, s, v, |s, v| HsvaGamma { s, v, ..opaque }.into());

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

    let additive = is_additive_alpha(hsvag.a);

    if alpha == Alpha::Opaque {
        hsvag.a = 1.0;
    } else {
        let a = &mut hsvag.a;

        if alpha == Alpha::OnlyBlend {
            if is_additive_alpha(*a) {
                *a = 0.5; // was additive, but isn't allowed to be
            }
            color_slider_1d(ui, a, |a| HsvaGamma { a, ..opaque }.into()).on_hover_text("Alpha");
        } else if !additive {
            color_slider_1d(ui, a, |a| HsvaGamma { a, ..opaque }.into()).on_hover_text("Alpha");
        }
    }
}

fn input_type_button_ui(ui: &mut Ui) {
    let mut input_type = ui.global_style().visuals.numeric_color_space;
    if input_type.toggle_button_ui(ui).changed() {
        ui.ctx().all_styles_mut(|s| {
            s.visuals.numeric_color_space = input_type;
        });
    }
}

/// Shows 4 `DragValue` widgets to be used to edit the RGBA u8 values.
/// Alpha's `DragValue` is hidden when `Alpha::Opaque`.
///
/// Returns `true` on change.
fn srgba_edit_ui(ui: &mut Ui, [r, g, b, a]: &mut [u8; 4], alpha: Alpha) -> bool {
    let mut edited = false;

    ui.horizontal(|ui| {
        input_type_button_ui(ui);

        if ui
            .button("📋")
            .on_hover_text("Click to copy color values")
            .clicked()
        {
            if alpha == Alpha::Opaque {
                ui.copy_text(format!("{r}, {g}, {b}"));
            } else {
                ui.copy_text(format!("{r}, {g}, {b}, {a}"));
            }
        }
        edited |= DragValue::new(r).speed(0.5).prefix("R ").ui(ui).changed();
        edited |= DragValue::new(g).speed(0.5).prefix("G ").ui(ui).changed();
        edited |= DragValue::new(b).speed(0.5).prefix("B ").ui(ui).changed();
        if alpha != Alpha::Opaque {
            edited |= DragValue::new(a).speed(0.5).prefix("A ").ui(ui).changed();
        }
    });

    edited
}

/// Shows 4 `DragValue` widgets to be used to edit the RGBA f32 values.
/// Alpha's `DragValue` is hidden when `Alpha::Opaque`.
///
/// Returns `true` on change.
fn rgba_edit_ui(ui: &mut Ui, [r, g, b, a]: &mut [f32; 4], alpha: Alpha) -> bool {
    fn drag_value(ui: &mut Ui, prefix: &str, value: &mut f32) -> Response {
        DragValue::new(value)
            .speed(0.003)
            .prefix(prefix)
            .range(0.0..=1.0)
            .custom_formatter(|n, _| format!("{n:.03}"))
            .ui(ui)
    }

    let mut edited = false;

    ui.horizontal(|ui| {
        input_type_button_ui(ui);

        if ui
            .button("📋")
            .on_hover_text("Click to copy color values")
            .clicked()
        {
            if alpha == Alpha::Opaque {
                ui.copy_text(format!("{r:.03}, {g:.03}, {b:.03}"));
            } else {
                ui.copy_text(format!("{r:.03}, {g:.03}, {b:.03}, {a:.03}"));
            }
        }

        edited |= drag_value(ui, "R ", r).changed();
        edited |= drag_value(ui, "G ", g).changed();
        edited |= drag_value(ui, "B ", b).changed();
        if alpha != Alpha::Opaque {
            edited |= drag_value(ui, "A ", a).changed();
        }
    });

    edited
}

/// Shows a color picker where the user can change the given [`Hsva`] color.
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
    let open = Popup::is_id_open(ui.ctx(), popup_id);
    let mut button_response = color_button(ui, hsva.to_srgba_unmultiplied(), open);
    if ui.style().explanation_tooltips {
        button_response = button_response.on_hover_text("Click to edit color");
    }

    const COLOR_SLIDER_WIDTH: f32 = 275.0;

    Popup::menu(&button_response)
        .id(popup_id)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.spacing_mut().slider_width = COLOR_SLIDER_WIDTH;
            if color_picker_hsva_2d(ui, hsva, alpha) {
                button_response.mark_changed();
            }
        });

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
    ctx.data_mut(|d| f(d.get_temp_mut_or_default(Id::NULL)))
}
