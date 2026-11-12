use egui::{AdaptiveLayout, Ui, vec2};

/// Demonstrates width-based selection between expanded and compact compositions.
#[derive(Default)]
pub struct AdaptiveLayoutDemo;

impl crate::Demo for AdaptiveLayoutDemo {
    fn name(&self) -> &'static str {
        "Adaptive Layout"
    }

    fn show(&mut self, ui: &mut Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .default_size(vec2(520.0, 220.0))
            .resizable(true)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for AdaptiveLayoutDemo {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("Resize this window to switch between expanded and compact toolbars.");
        ui.label(format!(
            "Available width: {:.0} points",
            ui.available_width()
        ));

        AdaptiveLayout::breakpoint(430.0).show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    let _ = ui.button("New document");
                    let _ = ui.button("Open");
                    let _ = ui.button("Save");
                    let _ = ui.button("Export");
                });
            },
            |ui| {
                ui.horizontal(|ui| {
                    let _ = ui.button("New");
                    let _ = ui.button("Open");
                    ui.menu_button("More", |ui| {
                        let _ = ui.button("Save");
                        let _ = ui.button("Export");
                    });
                });
            },
        );

        ui.separator();
        ui.label("Fit strategy (expanded minimum width: 360 points):");
        AdaptiveLayout::fit(360.0, 120.0).show(
            ui,
            |ui| {
                ui.horizontal(|ui| {
                    ui.label("Project: egui");
                    let _ = ui.button("Configure");
                });
            },
            |ui| {
                ui.vertical(|ui| {
                    ui.label("Project: egui");
                    let _ = ui.button("Configure");
                });
            },
        );
    }
}
