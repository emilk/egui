use egui_kittest::Harness;

#[test]
fn test_kerning() {
    for pixels_per_point in [1.0, 2.0] {
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            let mut harness = Harness::builder()
                .with_pixels_per_point(pixels_per_point)
                .with_theme(theme)
                .build_ui(|ui| {
                    ui.label("Thin spaces: −123 456 789");
                    ui.label("Ligature: fi :)");
                    ui.label("\ttabbed");
                });
            harness.run();
            harness.fit_contents();
            harness.snapshot(format!(
                "image_blending/image_{theme}_x{pixels_per_point}",
                theme = match theme {
                    egui::Theme::Dark => "dark",
                    egui::Theme::Light => "light",
                }
            ));
        }
    }
}
