use egui::{Drawer, DrawerEdge, Frame, Ui, vec2};

/// Demonstrates drawers attached to edges of a resizable parent rectangle.
#[derive(Default)]
pub struct DrawerDemo {
    edge: DrawerEdge,
    open: bool,
}

impl crate::Demo for DrawerDemo {
    fn name(&self) -> &'static str {
        "Drawer"
    }

    fn show(&mut self, ui: &mut Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .default_size(vec2(520.0, 360.0))
            .resizable(true)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for DrawerDemo {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("Resize this window to see the drawer follow its parent rectangle.");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.edge, DrawerEdge::Left, "Left");
            ui.selectable_value(&mut self.edge, DrawerEdge::Right, "Right");
            ui.selectable_value(&mut self.edge, DrawerEdge::Top, "Top");
            ui.selectable_value(&mut self.edge, DrawerEdge::Bottom, "Bottom");
        });
        ui.checkbox(&mut self.open, "Open drawer");

        let width = ui.available_width().min(480.0);
        let (_, target_rect) = ui.allocate_space(vec2(width, 220.0));
        ui.painter()
            .rect_filled(target_rect, 4.0, ui.visuals().faint_bg_color);
        ui.painter().text(
            target_rect.center(),
            egui::Align2::CENTER_CENTER,
            "Parent area",
            egui::FontId::proportional(16.0),
            ui.visuals().text_color(),
        );

        let _ = Drawer::new(egui::Id::unique("drawer_demo"))
            .edge(self.edge)
            .extent(140.0)
            .parent_rect(target_rect)
            .open(self.open)
            .show(ui.ctx(), |ui| {
                Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(format!("{:?} drawer", self.edge));
                    ui.label("Attached to the parent area");
                });
            });
    }
}
