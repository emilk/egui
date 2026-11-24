//! Snapshot test for the background of inline code, see [`egui::CodeVisuals`].

use egui::{CodeVisuals, Color32, CornerRadius, Margin, Visuals, vec2};
use egui_kittest::{Harness, SnapshotResults};

#[test]
fn code_background() {
    let mut results = SnapshotResults::new();

    let custom = Visuals {
        code: CodeVisuals {
            corner_radius: CornerRadius::ZERO,
            text_color: Some(Color32::LIGHT_BLUE),
            margin: Margin::same(1),
            ..CodeVisuals::dark()
        },
        ..Visuals::dark()
    };

    for (name, visuals) in [
        ("dark", Visuals::dark()),
        ("light", Visuals::light()),
        ("custom", custom),
    ] {
        let mut harness = Harness::builder()
            .with_size(vec2(240.0, 120.0))
            .build_ui(|ui| {
                *ui.visuals_mut() = visuals.clone();
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("Call ");
                        ui.code("foo()");
                        ui.label(" or ");
                        ui.code("bar");
                        ui.label(", then wrap some very long text with ");
                        ui.code("more_code_at_the_end");
                        ui.code("code_on_the_row_below");
                    });
                });
            });
        harness.fit_contents();
        harness.snapshot(format!("code_background_{name}"));
        results.extend_harness(&mut harness);
    }
}

/// A custom [`egui::text::TextFormat::line_height`] centers the text in a taller line. The
/// background, the underline and the strikethrough still follow the font, so they stay in their
/// row.
#[test]
fn code_background_tall_line_height() {
    use egui::text::{LayoutJob, TextFormat};
    use egui::{FontId, Stroke};

    let mut harness = Harness::builder()
        .with_size(vec2(240.0, 160.0))
        .build_ui(|ui| {
            *ui.visuals_mut() = Visuals::dark();
            egui::CentralPanel::default().show(ui, |ui| {
                let visuals = ui.visuals().clone();
                let line_height = Some(28.0);
                let text = TextFormat {
                    font_id: FontId::proportional(14.0),
                    color: visuals.text_color(),
                    line_height,
                    ..Default::default()
                };
                let code = TextFormat {
                    font_id: FontId::monospace(13.0),
                    background: visuals.code.bg_color,
                    background_margin: visuals.code.margin,
                    background_corner_radius: visuals.code.corner_radius,
                    background_stroke: visuals.code.stroke,
                    ..text.clone()
                };
                let stroke = Stroke::new(1.0, visuals.text_color());
                let mut job = LayoutJob::default();
                job.append("Call ", 0.0, text.clone());
                job.append("foo()", 0.0, code.clone());
                job.append(" and ", 0.0, text.clone());
                job.append(
                    "underline",
                    0.0,
                    TextFormat {
                        underline: stroke,
                        ..text.clone()
                    },
                );
                job.append(" or ", 0.0, text.clone());
                job.append(
                    "strike",
                    0.0,
                    TextFormat {
                        strikethrough: stroke,
                        ..text.clone()
                    },
                );
                job.append(", then wrap ", 0.0, text.clone());
                job.append("code_on_the_next_row", 0.0, code.clone());
                job.append(" and ", 0.0, text);
                job.append("more_code", 0.0, code);
                ui.label(job);
            });
        });
    harness.fit_contents();
    harness.snapshot("code_background_tall_line_height");
}
