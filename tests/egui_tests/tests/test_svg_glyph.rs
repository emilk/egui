//! Snapshot test for [`egui_extras::SvgGlyph`].

use egui::{Color32, RichText, vec2};
use egui_extras::SvgGlyph;
use egui_kittest::Harness;

/// A private use character, rendered as Ferris.
const FERRIS: char = '\u{E000}';

/// A regular character whose glyph is overridden.
const HEART: char = '♥';

#[test]
fn svg_glyph() {
    let mut harness = Harness::builder()
        .with_size(vec2(560.0, 150.0))
        .build_ui(|ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                for size in [12.0, 24.0, 48.0] {
                    ui.horizontal(|ui| {
                        for color in [Color32::WHITE, Color32::RED, Color32::LIGHT_BLUE] {
                            ui.label(
                                RichText::new(format!("I{HEART}{FERRIS}"))
                                    .color(color)
                                    .size(size),
                            );
                        }
                        ui.label(
                            RichText::new(format!("I{HEART}{FERRIS}"))
                                .monospace()
                                .size(size),
                        );
                    });
                }
            });
        });

    let heart = SvgGlyph::from_bytes(include_bytes!("../../../examples/svg_glyph/heart.svg"))
        .unwrap()
        .with_height(0.7)
        .with_baseline_offset(0.05);
    harness
        .ctx
        .add_glyph_rasterizer(heart.into_rasterizer(HEART.to_string()));

    let ferris =
        SvgGlyph::from_bytes(include_bytes!("../../../examples/images/src/ferris.svg")).unwrap();
    harness
        .ctx
        .add_glyph_rasterizer(ferris.into_rasterizer(FERRIS.to_string()));

    harness.run();
    harness.snapshot("svg_glyph");
}
