#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Panels {
    top: bool,
    left: bool,
    right: bool,
}

impl Default for Panels {
    fn default() -> Self {
        Self {
            top: true,
            left: true,
            right: true,
        }
    }
}

impl crate::Demo for Panels {
    fn name(&self) -> &'static str {
        "🗖 Panels"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        use crate::View as _;
        let window = egui::Window::new("Panels")
            .default_width(600.0)
            .default_height(400.0)
            .vscroll(false)
            .open(open);
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl crate::View for Panels {
    fn ui(&mut self, ui: &mut egui::Ui) {
        // Note that the order we add the panels is very important!

        let Self { top, left, right } = self;

        ui.horizontal(|ui| {
            ui.toggle_value(left, "Left");
            ui.toggle_value(top, "Top");
            ui.toggle_value(right, "Right");
        });
        ui.separator();

        egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .min_height(32.0)
            .show_animated_inside(ui, top, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Expandable Upper Panel");
                    });
                    lorem_ipsum(ui);
                });
            });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0)
            .show_animated_inside(ui, left, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Left Panel");
                });
                if ui.button("Close").clicked() {
                    ui.close();
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    lorem_ipsum(ui);
                });
            });

        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0)
            .show_animated_inside(ui, right, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Right Panel");
                });
                if ui.button("Close").clicked() {
                    ui.close();
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    lorem_ipsum(ui);
                });
            });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(0.0)
            .show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Bottom Panel");
                });
                ui.vertical_centered(|ui| {
                    ui.add(crate::egui_github_link_file!());
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Central Panel");
            });
            egui::ScrollArea::vertical().show(ui, |ui| {
                lorem_ipsum(ui);
            });
        });
    }
}

fn lorem_ipsum(ui: &mut egui::Ui) {
    ui.with_layout(
        egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
        |ui| {
            ui.label(egui::RichText::new(crate::LOREM_IPSUM_LONG).small().weak());
            ui.add(egui::Separator::default().grow(8.0));
            ui.label(egui::RichText::new(crate::LOREM_IPSUM_LONG).small().weak());
        },
    );
}
