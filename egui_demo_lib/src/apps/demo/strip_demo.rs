use egui::Color32;
use egui_extras::{Size, StripBuilder};

/// Shows off a table with dynamic layout
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default)]
pub struct StripDemo {}

impl super::Demo for StripDemo {
    fn name(&self) -> &'static str {
        "▣ Strip Demo"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                use super::View as _;
                self.ui(ui);
            });
    }
}

impl super::View for StripDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        StripBuilder::new(ui)
            .size(Size::exact(50.0))
            .size(Size::remainder())
            .size(Size::relative(0.5).at_least(60.0))
            .size(Size::exact(10.0))
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    ui.painter()
                        .rect_filled(ui.available_rect_before_wrap(), 0.0, Color32::BLUE);
                    ui.label("Full width and 50px height");
                });
                strip.strip(|builder| {
                    builder.sizes(Size::remainder(), 2).horizontal(|mut strip| {
                        strip.cell(|ui| {
                            ui.painter().rect_filled(
                                ui.available_rect_before_wrap(),
                                0.0,
                                Color32::RED,
                            );
                            ui.label("remaining height and 50% of the width");
                        });
                        strip.strip(|builder| {
                            builder.sizes(Size::remainder(), 3).vertical(|mut strip| {
                                strip.empty();
                                strip.cell(|ui| {
                                    ui.painter().rect_filled(
                                        ui.available_rect_before_wrap(),
                                        0.0,
                                        Color32::YELLOW,
                                    );
                                    ui.label("one third of the box left of me but same width");
                                });
                            });
                        });
                    });
                });
                strip.strip(|builder| {
                    builder
                        .size(Size::remainder())
                        .size(Size::exact(60.0))
                        .size(Size::remainder())
                        .size(Size::exact(70.0))
                        .horizontal(|mut strip| {
                            strip.empty();
                            strip.strip(|builder| {
                                builder
                                    .size(Size::remainder())
                                    .size(Size::exact(60.0))
                                    .size(Size::remainder())
                                    .vertical(|mut strip| {
                                        strip.empty();
                                        strip.cell(|ui| {
                                            ui.painter().rect_filled(
                                                ui.available_rect_before_wrap(),
                                                0.0,
                                                Color32::GOLD,
                                            );
                                            ui.label("60x60");
                                        });
                                    });
                            });
                            strip.empty();
                            strip.cell(|ui| {
                                ui.painter().rect_filled(
                                    ui.available_rect_before_wrap(),
                                    0.0,
                                    Color32::GREEN,
                                );
                                ui.label("height: half the available - at least 60px, width: 70px");
                            });
                        });
                });
                strip.cell(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(crate::__egui_github_link_file!());
                    });
                });
            });
    }
}
