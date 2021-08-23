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
            .scroll(false)
            .open(open);
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for WindowWithPanels {
    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::TopBottomPanel::top("top_panel")
            .resizable(false)
            .min_height(0.0)
            .show_inside(ui, |ui| {
                egui::menu::bar(ui, |ui| {
                    egui::menu::menu(ui, "Menu", |ui| {
                        if ui.button("Option 1").clicked() {}
                        if ui.button("Option 2").clicked() {}
                        if ui.button("Option 3").clicked() {}
                    });
                });
            });

        egui::TopBottomPanel::bottom("bottom_panel_A")
            .resizable(false)
            .min_height(0.0)
            .show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Outer Bottom Panel");
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
                egui::ScrollArea::auto_sized().show(ui, |ui| {
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
                egui::ScrollArea::auto_sized().show(ui, |ui| {
                    ui.add(egui::Label::new(crate::LOREM_IPSUM_LONG).small().weak());
                });
            });

        egui::TopBottomPanel::bottom("bottom_panel_B")
            .resizable(false)
            .min_height(0.0)
            .show_inside(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Inner Bottom Panel");
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Central Panel");
            });
            egui::ScrollArea::auto_sized().show(ui, |ui| {
                ui.add(egui::Label::new(crate::LOREM_IPSUM_LONG).small().weak());
            });
        });
    }
}
