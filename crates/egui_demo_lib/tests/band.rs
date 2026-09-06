use egui::{
    Color32, Pos2, Rect, Stroke, StrokeKind, Vec2,
    epaint::{self, BandPoint, BandShape},
    pos2,
};
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
            BandPoint::new(origin.x + x, (origin.y - radius)..=(origin.y + radius))
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
                let mut band =
                    BandShape::new(band_points(origin), Color32::from_rgb(0, 181, 255), stroke);
                if let Some(stroke_kind) = stroke_kind {
                    band = band.with_stroke_kind(*stroke_kind);
                }
                painter.add(band);
            }
        });
    harness.run();
    harness.snapshot("band_pinch");
}

/// A fixed-width, fill-only band should look like a path with a stroke of the same width.
#[test]
fn fixed_width_band_matches_path_stroke() {
    let padding = 6.0;
    let line_width = 2.5;
    let row_height = line_width + 2.0 * padding;
    let size = Vec2::new(LENGTH + 2.0 * padding, 6.0 * row_height);
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

            let band_center = pos2(padding, 0.5 * row_height);
            painter.add(BandShape::filled(
                vec![
                    BandPoint::from_pos_and_width(band_center, line_width),
                    BandPoint::from_pos_and_width(
                        pos2(band_center.x + LENGTH, band_center.y),
                        line_width,
                    ),
                ],
                Color32::WHITE,
            ));

            let path_center = pos2(padding, 1.5 * row_height);
            painter.add(epaint::PathShape::line(
                vec![path_center, pos2(path_center.x + LENGTH, path_center.y)],
                Stroke::new(line_width, Color32::WHITE),
            ));

            let rect_center = pos2(padding, 2.5 * row_height);
            painter.add(epaint::RectShape::filled(
                Rect::from_center_size(
                    rect_center + Vec2::new(0.5 * LENGTH, 0.0),
                    Vec2::new(LENGTH, line_width),
                ),
                0.0,
                Color32::WHITE,
            ));

            let angle = -0.4;
            let band_center = pos2(padding, 3.5 * row_height);
            painter.add(
                BandShape::filled(
                    vec![
                        BandPoint::from_pos_and_width(band_center, line_width),
                        BandPoint::from_pos_and_width(
                            pos2(band_center.x + LENGTH, band_center.y),
                            line_width,
                        ),
                    ],
                    Color32::WHITE,
                )
                .with_angle_and_pivot(angle, band_center),
            );

            let path_center = pos2(padding, 4.5 * row_height);
            let rotation = egui::emath::Rot2::from_angle(angle);
            painter.add(epaint::PathShape::line(
                vec![path_center, path_center + rotation * Vec2::new(LENGTH, 0.0)],
                Stroke::new(line_width, Color32::WHITE),
            ));

            let rect_center = pos2(padding, 5.5 * row_height);
            painter.add(
                epaint::RectShape::filled(
                    Rect::from_center_size(
                        rect_center + Vec2::new(0.5 * LENGTH, 0.0),
                        Vec2::new(LENGTH, line_width),
                    ),
                    0.0,
                    Color32::WHITE,
                )
                .with_angle_and_pivot(angle, rect_center),
            );
        });
    harness.run();
    harness.snapshot("fixed_width_band_matches_path_stroke");
}
