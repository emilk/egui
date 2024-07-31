#[derive(Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ContextMenus {}

impl crate::Demo for ContextMenus {
    fn name(&self) -> &'static str {
        "☰ Context Menus"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        use crate::View;
        egui::Window::new(self.name())
            .vscroll(false)
            .resizable(false)
            .open(open)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl crate::View for ContextMenus {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.menu_button("Click for menu", Self::nested_menus);

            ui.button("Right-click for menu")
                .context_menu(Self::nested_menus);

            if ui.ctx().is_context_menu_open() {
                ui.label("Context menu is open");
            } else {
                ui.label("Context menu is closed");
            }
        });

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}

impl ContextMenus {
    fn nested_menus(ui: &mut egui::Ui) {
        ui.set_max_width(200.0); // To make sure we wrap long text

        if ui.button("Open…").clicked() {
            ui.close_menu();
        }
        ui.menu_button("SubMenu", |ui| {
            ui.menu_button("SubMenu", |ui| {
                if ui.button("Open…").clicked() {
                    ui.close_menu();
                }
                let _ = ui.button("Item");
            });
            ui.menu_button("SubMenu", |ui| {
                if ui.button("Open…").clicked() {
                    ui.close_menu();
                }
                let _ = ui.button("Item");
            });
            let _ = ui.button("Item");
            if ui.button("Open…").clicked() {
                ui.close_menu();
            }
        });
        ui.menu_button("SubMenu", |ui| {
            let _ = ui.button("Item1");
            let _ = ui.button("Item2");
            let _ = ui.button("Item3");
            let _ = ui.button("Item4");
            if ui.button("Open…").clicked() {
                ui.close_menu();
            }
        });
        let _ = ui.button("Very long text for this item that should be wrapped");
    }
}
