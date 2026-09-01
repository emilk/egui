use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Vec2, epaint, pos2};
use egui_kittest::Harness;

/// The band is 25 pixels long, and pinches smoothly from 5 pixels wide, to nothing, and back.
const LENGTH: f32 = 25.0;
const WIDTH: f32 = 5.0;

fn band_points(origin: Pos2) -> Vec<epaint::BandPoint> {
    let num_samples = 101;
    (0..num_samples)
        .map(|index| {
            let t = index as f32 / (num_samples - 1) as f32;
            let x = t * LENGTH;
            let radius = 0.5 * WIDTH * (core::f32::consts::PI * t).cos().powi(2);
            epaint::BandPoint::new(origin.x + x, (origin.y - radius)..=(origin.y + radius))
        })
        .collect()
}

/// A band that pinches down to zero width, with each of the [`StrokeKind`]s.
///
/// The pinch is the interesting part: the two boundaries meet there,
/// so both the fill and the stroke have to survive a zero-width band.
#[test]
fn band_pinch() {
    let padding = 6.0;
    let row_height = WIDTH + 2.0 * padding;
    let strokes = [
        ("no stroke", None),
        ("outside", Some(StrokeKind::Outside)),
        ("middle", Some(StrokeKind::Middle)),
        ("inside", Some(StrokeKind::Inside)),
    ];

    let size = Vec2::new(LENGTH + 2.0 * padding, row_height * strokes.len() as f32);
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(1.0)
        .build_ui(move |ui| {
            let painter = ui.painter();
            painter.rect_filled(
                Rect::from_min_size(Pos2::ZERO, size),
                0.0,
                Color32::from_gray(32),
            );

            for (row, (_name, stroke_kind)) in strokes.iter().enumerate() {
                let origin = pos2(padding, row as f32 * row_height + 0.5 * row_height);
                let stroke = match stroke_kind {
                    Some(_) => Stroke::new(1.0, Color32::WHITE),
                    None => Stroke::NONE,
                };
                let mut band = epaint::BandShape::new(
                    band_points(origin),
                    Color32::from_rgb(0, 181, 255),
                    stroke,
                );
                if let Some(stroke_kind) = stroke_kind {
                    band = band.with_stroke_kind(*stroke_kind);
                }
                painter.add(band);
            }
        });
    harness.run();
    harness.snapshot("band_pinch");
}
