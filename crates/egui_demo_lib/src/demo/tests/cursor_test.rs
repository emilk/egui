#[derive(Default)]
pub struct CursorTest {}

impl crate::Demo for CursorTest {
    fn name(&self) -> &'static str {
        "Cursor Test"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(crate::i18n::t(self.name()))
            .open(open)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for CursorTest {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered_justified(|ui| {
            ui.heading(crate::i18n::t("Hover to switch cursor icon:"));
            for &cursor_icon in &egui::CursorIcon::ALL {
                // The `{name:?}` debug spelling of each cursor icon is the label on purpose.
                let _ = ui
                    .button(format!("{cursor_icon:?}"))
                    .on_hover_cursor(cursor_icon);
            }
            ui.add(crate::egui_github_link_file!());
        });
    }
}
