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
            .scroll(false)
            .title_bar(true)
            .resizable(true)
            .collapsible(false)
            .enabled(true)
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
                ui.label("Bottom Panel A");
            });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .width_range(60.0..=200.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::auto_sized().show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Left Panel");
                        ui.small(crate::LOREM_IPSUM_LONG);
                    });
                });
            });

        egui::SidePanel::right("right_panel")
            .resizable(true)
            .width_range(60.0..=200.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::auto_sized().show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Right Panel");
                        ui.small(crate::LOREM_IPSUM_LONG);
                    });
                });
            });

        egui::TopBottomPanel::bottom("bottom_panel_B")
            .resizable(false)
            .min_height(0.0)
            .show_inside(ui, |ui| {
                ui.label("Bottom Panel B");
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::auto_sized().show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label("Central Panel");
                    ui.small(crate::LOREM_IPSUM_LONG);
                });
            });
        });
    }
}
