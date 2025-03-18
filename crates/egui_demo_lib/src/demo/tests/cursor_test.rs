#[derive(Default)]
pub struct CursorTest {}

impl crate::Demo for CursorTest {
    fn name(&self) -> &'static str {
        "Cursor Test"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name()).open(open).show(ctx, |ui| {
            use crate::View as _;
            self.ui(ui);
        });
    }
}

impl crate::View for CursorTest {
    fn ui(&mut self, ui: &mut egui::Ui) {
        if ui
            .button("Center pointer in window")
            .on_hover_text("The platform may not support this.")
            .clicked()
        {
            let position = ui.ctx().available_rect().center();
            ui.ctx().set_pointer_position(position);
        }
        ui.vertical_centered_justified(|ui| {
            ui.heading("Hover to switch cursor icon:");
            for &cursor_icon in &egui::CursorIcon::ALL {
                let _ = ui
                    .button(format!("{cursor_icon:?}"))
                    .on_hover_cursor(cursor_icon);
            }
            ui.add(crate::egui_github_link_file!());
        });
    }
}
