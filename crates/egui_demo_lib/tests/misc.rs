use egui_kittest::Harness;

#[test]
fn test_kerning() {
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
        }
    }
}

#[test]
fn test_italics() {
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
        }
    }
}
