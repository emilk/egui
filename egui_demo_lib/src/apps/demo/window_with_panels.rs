#[derive(Clone, PartialEq, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowWithPanels {}

impl super::Demo for WindowWithPanels {
    fn name(&self) -> &'static str {
        "🗖 Window With Panels"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        use super::View;
        let window = egui::Window::new("Window with Panels")
            .default_width(600.0)
            .default_height(400.0)
            .vscroll(false)
            .open(open);
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for WindowWithPanels {
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
                    ui.add(egui::Label::new(crate::LOREM_IPSUM_LONG).small().weak());
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
                    ui.add(egui::Label::new(crate::LOREM_IPSUM_LONG).small().weak());
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
                    ui.add(egui::Label::new(crate::LOREM_IPSUM_LONG).small().weak());
                });
            });

        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(0.0)
            .show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Bottom Panel");
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Central Panel");
            });
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(egui::Label::new(crate::LOREM_IPSUM_LONG).small().weak());
            });
        });
    }
}
