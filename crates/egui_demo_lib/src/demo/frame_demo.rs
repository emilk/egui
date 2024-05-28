use egui::{epaint::PathStroke, lerp, remap_clamp, Color32};

/// Shows off a table with dynamic layout
#[derive(PartialEq)]
pub struct FrameDemo {
    frame: egui::Frame,
    fancy: bool,
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
                stroke: egui::Stroke::new(1.0, egui::Color32::GRAY).into(),
            },
            fancy: false,
        }
    }
}

impl super::Demo for FrameDemo {
    fn name(&self) -> &'static str {
        "▣ Frame"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .show(ctx, |ui| {
                use super::View as _;
                self.ui(ui);
            });
    }
}

impl super::View for FrameDemo {
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
                        if ui.checkbox(&mut self.fancy, "Fancy Stroke").clicked() {
                            let width = self.frame.stroke.width;
                            if self.fancy {
                                self.frame.stroke = PathStroke::new_uv(width, |rect, pos| {
                                    let x_t = remap_clamp(pos.x, rect.x_range(), 0.0..=1.0);
                                    let y_t = remap_clamp(pos.y, rect.y_range(), 0.0..=1.0);

                                    Color32::from_rgb(
                                        lerp(
                                            Color32::RED.r() as f32..=Color32::GREEN.r() as f32,
                                            x_t,
                                        ) as u8,
                                        lerp(
                                            Color32::RED.g() as f32..=Color32::GREEN.g() as f32,
                                            y_t,
                                        ) as u8,
                                        lerp(
                                            Color32::RED.b() as f32..=Color32::GREEN.b() as f32,
                                            x_t,
                                        ) as u8,
                                    )
                                });
                            } else {
                                self.frame.stroke =
                                    egui::Stroke::new(width, egui::Color32::GRAY).into();
                            }
                        }
                        self.frame.clone().show(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
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
