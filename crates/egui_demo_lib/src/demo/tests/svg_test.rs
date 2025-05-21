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

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name()).open(open).show(ctx, |ui| {
            use crate::View as _;
            self.ui(ui);
        });
    }
}

impl crate::View for SvgTest {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self { color } = self;
        ui.color_edit_button_srgba(color);
        ui.add(egui::Image::new(egui::include_image!("../../../data/peace.svg")).tint(*color));
    }
}
