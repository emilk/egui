use egui::{
    Align, Align2, Align4, ComboBox, Direction, Frame, Label, Layout, Popup, Rect, RichText, Sense,
    Ui, UiBuilder, Vec2, Widget,
};

/// Showcase [`egui::Ui::response`].
#[derive(PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// #[cfg_attr(feature = "serde", serde(default))]
pub struct PopupsDemo {
    align4: Align4,
    gap: f32,
}

impl Default for PopupsDemo {
    fn default() -> Self {
        Self {
            align4: Align4::default(),
            gap: 4.0,
        }
    }
}

impl crate::Demo for PopupsDemo {
    fn name(&self) -> &'static str {
        "\u{20E3} Popups"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .default_width(250.0)
            .constrain(false)
            .show(ctx, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for PopupsDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.horizontal(|ui| {
            let align_combobox = |ui: &mut Ui, label: &str, align: &mut Align2| {
                let aligns = [
                    (Align2::LEFT_TOP, "Left top"),
                    (Align2::LEFT_CENTER, "Left center"),
                    (Align2::LEFT_BOTTOM, "Left bottom"),
                    (Align2::CENTER_TOP, "Center top"),
                    (Align2::CENTER_CENTER, "Center center"),
                    (Align2::CENTER_BOTTOM, "Center bottom"),
                    (Align2::RIGHT_TOP, "Right top"),
                    (Align2::RIGHT_CENTER, "Right center"),
                    (Align2::RIGHT_BOTTOM, "Right bottom"),
                ];

                ComboBox::new(label, "")
                    .selected_text(aligns.iter().find(|(a, _)| a == align).unwrap().1)
                    .show_ui(ui, |ui| {
                        for (align2, name) in &aligns {
                            ui.selectable_value(align, *align2, *name);
                        }
                    });
            };

            ui.label("Align4(");
            align_combobox(ui, "align", &mut self.align4.align);
            ui.label(", ");
            align_combobox(ui, "focus", &mut self.align4.focus);
            ui.label(")");
        });
        ui.horizontal(|ui| {
            ui.label("Gap:");
            ui.add(egui::DragValue::new(&mut self.gap));
        });

        let response = Frame::group(ui.style())
            .outer_margin(150.0)
            .inner_margin(0.0)
            .show(ui, |ui| {
                ui.allocate_exact_size(Vec2::new(50.0, 50.0), Sense::hover())
                    .1
            })
            .inner;

        Popup::from_response(&response)
            .position(self.align4)
            .gap(self.gap)
            .show(ui.ctx(), |ui| {
                ui.set_min_size(Vec2::splat(100.0));
                ui.label("Hi!");
            });
    }
}
