#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default)]
pub struct InputTest {
    info: String,
}

impl super::Demo for InputTest {
    fn name(&self) -> &str {
        "🖱 Input Test"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .show(ctx, |ui| {
                use super::View;
                self.ui(ui);
            });
    }
}

impl super::View for InputTest {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let response = ui.add(
            egui::Button::new("Click, double-click or drag me with any mouse button")
                .sense(egui::Sense::click_and_drag()),
        );

        let mut new_info = String::new();
        for &button in &[
            egui::PointerButton::Primary,
            egui::PointerButton::Secondary,
            egui::PointerButton::Middle,
        ] {
            if response.clicked_by(button) {
                new_info += &format!("Clicked by {:?}\n", button);
            }
            if response.double_clicked_by(button) {
                new_info += &format!("Double-clicked by {:?}\n", button);
            }
            if response.dragged() && ui.input().pointer.button_down(button) {
                new_info += &format!("Dragged by {:?}\n", button);
            }
        }
        if !new_info.is_empty() {
            self.info = new_info;
        }

        ui.label(&self.info);
    }
}
