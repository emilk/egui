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
        let img_src = egui::include_image!("../../../data/peace.svg");

        // First paint a small version…
        ui.add_sized(
            egui::Vec2 { x: 20.0, y: 20.0 },
            egui::Image::new(img_src.clone()).tint(*color),
        );

        // …then a big one, to make sure they are both crisp
        ui.add(egui::Image::new(img_src).tint(*color));
    }
}
