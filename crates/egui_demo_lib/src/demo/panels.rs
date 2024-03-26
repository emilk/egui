#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Panels {}

impl super::Demo for Panels {
    fn name(&self) -> &'static str {
        "🗖 Panels"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        use super::View as _;
        let window = egui::Window::new("Panels")
            .default_width(600.0)
            .default_height(400.0)
            .vscroll(false)
            .open(open);
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for Panels {
    fn ui(&mut self, ui: &mut egui::Ui) {
        // Note that the order we add the panels is very important!

        egui::TopBottomPanel::top("top_panel")
            .resizable(true)
            .min_height(32.0)
            .show_inside(ui, |ui| {
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
            .show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Left Panel");
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    lorem_ipsum(ui);
                });
            });

        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=200.0)
            .show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Right Panel");
                });
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
