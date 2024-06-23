/// Shows off a table with dynamic layout
#[derive(PartialEq)]
pub struct FrameDemo {
    frame: egui::Frame,
}

impl Default for FrameDemo {
    fn default() -> Self {
        Self {
            frame: egui::Frame {
                inner_margin: 12.0.into(),
                outer_margin: 24.0.into(),
                rounding: 14.0.into(),
                shadow: egui::Shadow {
                    offset: [8.0, 12.0].into(),
                    blur: 16.0,
                    spread: 0.0,
                    color: egui::Color32::from_black_alpha(180),
                },
                fill: egui::Color32::from_rgba_unmultiplied(97, 0, 255, 128),
                stroke: egui::Stroke::new(1.0, egui::Color32::GRAY),
            },
        }
    }
}

impl crate::Demo for FrameDemo {
    fn name(&self) -> &'static str {
        "▣ Frame"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .show(ctx, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for FrameDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.add(&mut self.frame);

                ui.add_space(8.0);
                ui.set_max_width(ui.min_size().x);
                ui.vertical_centered(|ui| egui::reset_button(ui, self, "Reset"));
            });

            ui.separator();

            ui.vertical(|ui| {
                // We want to paint a background around the outer margin of the demonstration frame, so we use another frame around it:
                egui::Frame::default()
                    .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                    .rounding(ui.visuals().widgets.noninteractive.rounding)
                    .show(ui, |ui| {
                        self.frame.show(ui, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                            ui.label(egui::RichText::new("Content").color(egui::Color32::WHITE));
                        });
                    });
            });
        });

        ui.set_max_width(ui.min_size().x);
        ui.separator();
        ui.vertical_centered(|ui| ui.add(crate::egui_github_link_file!()));
    }
}
