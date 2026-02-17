pub struct SvgTest {
    color: egui::Color32,
}

impl Default for SvgTest {
    fn default() -> Self {
        Self {
            color: egui::Color32::LIGHT_RED,
        }
    }
}

impl crate::Demo for SvgTest {
    fn name(&self) -> &'static str {
        "SVG Test"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for SvgTest {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self { color } = self;
        ui.color_edit_button_srgba(color);
        let img_src = egui::include_image!("../../../data/peace.svg");

        // First paint a small version, sized the same as the source…
        ui.add(
            egui::Image::new(img_src.clone())
                .fit_to_original_size(1.0)
                .tint(*color),
        );

        // …then a big one, to make sure they are both crisp
        ui.add(egui::Image::new(img_src).tint(*color));
    }
}
