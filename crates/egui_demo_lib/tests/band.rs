use egui::{
    Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2,
    epaint::{self, BandPoint, BandShape},
    pos2,
};
use egui_kittest::Harness;

/// The band is 25 pixels long, and pinches smoothly from 5 pixels wide, to nothing, and back.
const LENGTH: f32 = 25.0;
const WIDTH: f32 = 5.0;

/// Room for the row labels, to the left of the shapes.
const LABEL_WIDTH: f32 = 120.0;

/// Room for the line that says what the snapshot should show.
const HEADER_HEIGHT: f32 = 16.0;

/// Wide enough for the header line, which is longer than the labels plus the shapes.
const IMAGE_WIDTH: f32 = 220.0;

const TEXT_COLOR: Color32 = Color32::from_gray(200);

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

fn text(painter: &egui::Painter, left_center: Pos2, text: &str) {
    painter.text(
        left_center,
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(10.0),
        TEXT_COLOR,
    );
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

    let size = Vec2::new(
        IMAGE_WIDTH,
        HEADER_HEIGHT + row_height * strokes.len() as f32,
    );
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
            text(
                painter,
                pos2(padding, 0.5 * HEADER_HEIGHT),
                "Every row should survive the pinch",
            );

            for (row, (name, stroke_kind)) in strokes.iter().enumerate() {
                let y = HEADER_HEIGHT + (row as f32 + 0.5) * row_height;
                text(painter, pos2(padding, y), name);

                let origin = pos2(LABEL_WIDTH + padding, y);
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
    let size = Vec2::new(IMAGE_WIDTH, HEADER_HEIGHT + 6.0 * row_height);
    let left = LABEL_WIDTH + padding;
    let row_center = |row: f32| pos2(left, HEADER_HEIGHT + (row + 0.5) * row_height);

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
            text(
                painter,
                pos2(padding, 0.5 * HEADER_HEIGHT),
                "Each group of three should look the same",
            );

            for (row, name) in [
                "band",
                "path stroke",
                "rect",
                "band, rotated",
                "path stroke, rotated",
                "rect, rotated",
            ]
            .iter()
            .enumerate()
            {
                text(painter, pos2(padding, row_center(row as f32).y), name);
            }

            let band_center = row_center(0.0);
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

            let path_center = row_center(1.0);
            painter.add(epaint::PathShape::line(
                vec![path_center, pos2(path_center.x + LENGTH, path_center.y)],
                Stroke::new(line_width, Color32::WHITE),
            ));

            let rect_center = row_center(2.0);
            painter.add(epaint::RectShape::filled(
                Rect::from_center_size(
                    rect_center + Vec2::new(0.5 * LENGTH, 0.0),
                    Vec2::new(LENGTH, line_width),
                ),
                0.0,
                Color32::WHITE,
            ));

            let angle = -0.4;
            let band_center = row_center(3.0);
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

            let path_center = row_center(4.0);
            let rotation = egui::emath::Rot2::from_angle(angle);
            painter.add(epaint::PathShape::line(
                vec![path_center, path_center + rotation * Vec2::new(LENGTH, 0.0)],
                Stroke::new(line_width, Color32::WHITE),
            ));

            let rect_center = row_center(5.0);
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
