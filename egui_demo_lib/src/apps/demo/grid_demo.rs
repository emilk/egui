use egui::Color32;
use egui_extras::{GridBuilder, Padding, Size};

/// Shows off a table with dynamic layout
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default)]
pub struct GridDemo {}

impl super::Demo for GridDemo {
    fn name(&self) -> &'static str {
        "▣ Grid Demo"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
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

impl super::View for GridDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        GridBuilder::new(ui, Padding::new(0.0, 5.0))
            .size(Size::Absolute(50.0))
            .size(Size::Remainder)
            .size(Size::RelativeMinimum {
                relative: 0.5,
                minimum: 60.0,
            })
            .vertical(|mut grid| {
                grid.cell(|ui| {
                    ui.painter()
                        .rect_filled(ui.available_rect_before_wrap(), 0.0, Color32::BLUE);
                });
                grid.grid(|builder| {
                    builder.sizes(Size::Remainder, 2).horizontal(|mut grid| {
                        grid.cell(|ui| {
                            ui.painter().rect_filled(
                                ui.available_rect_before_wrap(),
                                0.0,
                                Color32::RED,
                            );
                        });
                        grid.grid(|builder| {
                            builder.sizes(Size::Remainder, 3).vertical(|mut grid| {
                                grid.empty();
                                grid.cell(|ui| {
                                    ui.painter().rect_filled(
                                        ui.available_rect_before_wrap(),
                                        0.0,
                                        Color32::YELLOW,
                                    );
                                });
                            });
                        });
                    });
                });
                grid.grid(|builder| {
                    builder
                        .size(Size::Remainder)
                        .size(Size::Absolute(50.0))
                        .size(Size::Remainder)
                        .size(Size::Absolute(70.0))
                        .horizontal(|mut grid| {
                            grid.empty();
                            grid.grid(|builder| {
                                builder
                                    .size(Size::Remainder)
                                    .size(Size::Absolute(50.0))
                                    .size(Size::Remainder)
                                    .vertical(|mut grid| {
                                        grid.empty();
                                        grid.cell(|ui| {
                                            ui.painter().rect_filled(
                                                ui.available_rect_before_wrap(),
                                                0.0,
                                                Color32::GOLD,
                                            );
                                        });
                                    });
                            });
                            grid.empty();
                            grid.cell(|ui| {
                                ui.painter().rect_filled(
                                    ui.available_rect_before_wrap(),
                                    0.0,
                                    Color32::GREEN,
                                );
                            });
                        });
                });
            });
    }
}
