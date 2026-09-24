use egui::{Align, Align2, Group, ScrollArea};

#[derive(PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct GroupDemo {
    align_x: Align,
    align_y: Align,
}

impl Default for GroupDemo {
    fn default() -> Self {
        Self {
            align_x: Align::Center,
            align_y: Align::Center,
        }
    }
}

impl crate::Demo for GroupDemo {
    fn name(&self) -> &'static str {
        "Group"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .default_width(300.0)
            .default_height(300.0)
            .resizable(true)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for GroupDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.horizontal(|ui| {
            ui.label("Horizontal:");
            ui.selectable_value(&mut self.align_x, Align::Min, "Left");
            ui.selectable_value(&mut self.align_x, Align::Center, "Center");
            ui.selectable_value(&mut self.align_x, Align::Max, "Right");
        });

        ui.horizontal(|ui| {
            ui.label("Vertical:");
            ui.selectable_value(&mut self.align_y, Align::Min, "Top");
            ui.selectable_value(&mut self.align_y, Align::Center, "Center");
            ui.selectable_value(&mut self.align_y, Align::Max, "Bottom");
        });

        ui.separator();

        let align2 = Align2([self.align_x, self.align_y]);


        Group::new("demo_group").align2(align2).ui(ui, |ui| {
            ui.label("Hello!");
            let _ = ui.button("A button");
            ui.label("More text");
            ScrollArea::vertical().max_height(50.0).show(ui, |ui| {
                for _ in 0..100 {
                    ui.label("Even more text");
                }
            });
        });
        ui.set_height(ui.available_height());
    }
}
