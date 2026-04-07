use egui::{Color32, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable as _};

#[test]
fn test_kerning() {
    let mut results = egui_kittest::SnapshotResults::new();
    for pixels_per_point in [1.0, 2.0] {
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let mut harness = Harness::builder()
                .with_pixels_per_point(pixels_per_point)
                .with_theme(theme)
                .build_ui(|ui| {
                    ui.label("Hello world!");
                    ui.label("Repeated characters: iiiiiiiiiiiii lllllllll mmmmmmmmmmmmmmmm");
                    ui.label("Thin spaces: −123 456 789");
                    ui.label("Ligature: fi :)");
                    ui.label("\ttabbed");
                });
            harness.run();
            harness.fit_contents();
            harness.snapshot(format!(
                "image_kerning/image_{theme}_x{pixels_per_point}",
                theme = match theme {
                    egui::Theme::Dark => "dark",
                    egui::Theme::Light => "light",
                }
            ));
            results.extend_harness(&mut harness);
        }
    }
}

#[test]
fn test_italics() {
    let mut results = egui_kittest::SnapshotResults::new();
    for pixels_per_point in [1.0, 2.0_f32.sqrt(), 2.0] {
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let mut harness = Harness::builder()
                .with_pixels_per_point(pixels_per_point)
                .with_theme(theme)
                .build_ui(|ui| {
                    ui.label(egui::RichText::new("Small italics").italics().small());
                    ui.label(egui::RichText::new("Normal italics").italics());
                    ui.label(egui::RichText::new("Large italics").italics().size(22.0));
                });
            harness.run();
            harness.fit_contents();
            harness.snapshot(format!(
                "italics/image_{theme}_x{pixels_per_point:.2}",
                theme = match theme {
                    egui::Theme::Dark => "dark",
                    egui::Theme::Light => "light",
                }
            ));
            results.extend_harness(&mut harness);
        }
    }
}

#[test]
fn test_text_selection() {
    let mut results = egui_kittest::SnapshotResults::new();

    for (test_idx, drag_start_x) in [0.2_f32, 0.95].into_iter().enumerate() {
        let mut harness = Harness::builder()
            .with_pixels_per_point(1.0) // TODO(emilk): why does this test fail with 2.0?
            .build_ui(|ui| {
                let visuals = ui.visuals_mut();
                visuals.selection.bg_fill = Color32::LIGHT_GREEN;
                visuals.selection.stroke.color = Color32::RED;

                ui.vertical_centered(|ui| {
                    ui.label("Some varied ☺ text :)\nAnd it has a second line!\nAnd a third!");
                });
            });
        harness.run();
        harness.fit_contents();

        // Drag to select text:
        let label = harness.get_by_role(Role::Label);
        harness.drag_at(label.rect().lerp_inside([drag_start_x, 0.25]));
        harness.drop_at(label.rect().lerp_inside([0.5, 0.75]));
        harness.run();

        harness.snapshot(format!("text_selection_{test_idx}"));

        results.extend_harness(&mut harness);
    }
}
