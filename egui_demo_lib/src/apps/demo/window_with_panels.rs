use egui::{menu, Align, CentralPanel, Layout, ScrollArea, SidePanel, TopBottomPanel};

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
        let left_panel_min_width = 100.;
        let left_panel_max_width = left_panel_min_width * 4.;
        let bottom_height = 25.;

        ui.expand_to_include_rect(ui.max_rect()); // Expand frame to include it all

        let mut top_rect = ui.available_rect_before_wrap_finite();
        top_rect.min.y += ui.spacing().item_spacing.y;
        let mut top_ui = ui.child_ui(top_rect, Layout::top_down(Align::Max));

        let top_response = TopBottomPanel::top("window_menu")
            .resizable(false)
            .show_inside(&mut top_ui, |ui| {
                menu::bar(ui, |ui| {
                    menu::menu(ui, "Menu", |ui| {
                        if ui.button("Option 1").clicked() {}
                        if ui.button("Option 2").clicked() {}
                        if ui.button("Option 3").clicked() {}
                    });
                });
            });

        let mut left_rect = ui.available_rect_before_wrap_finite();
        left_rect.min.y = top_response.response.rect.max.y + ui.spacing().item_spacing.y;
        let mut left_ui = ui.child_ui(left_rect, Layout::top_down(Align::Max));

        let left_response = SidePanel::left("Folders")
            .resizable(true)
            .min_width(left_panel_min_width)
            .max_width(left_panel_max_width)
            .show_inside(&mut left_ui, |ui| {
                ScrollArea::auto_sized().show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Left Panel");
                    })
                })
            });

        let mut right_rect = ui.available_rect_before_wrap_finite();
        right_rect.min.x = left_response.response.rect.max.x;
        right_rect.min.y = top_response.response.rect.max.y + ui.spacing().item_spacing.y;
        let mut right_ui = ui.child_ui(right_rect, Layout::top_down(Align::Max));

        CentralPanel::default().show_inside(&mut right_ui, |ui| {
            let mut rect = ui.min_rect();
            let mut bottom_rect = rect;
            bottom_rect.min.y = ui.max_rect_finite().max.y - bottom_height;
            rect.max.y = bottom_rect.min.y - ui.spacing().indent;
            let mut child_ui = ui.child_ui(rect, Layout::top_down(Align::Min));
            let mut bottom_ui = ui.child_ui(bottom_rect, Layout::bottom_up(Align::Max));
            ScrollArea::auto_sized().show(&mut child_ui, |ui| {
                ui.vertical(|ui| {
                    ui.label("Central Panel");
                })
            });
            bottom_ui.vertical(|ui| {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Bottom Content");
                });
            });
        });
    }
}
