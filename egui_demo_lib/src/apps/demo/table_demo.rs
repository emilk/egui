use egui_dynamic_grid::{Padding, Size, TableBuilder};

/// Shows off a table with dynamic layout
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default)]
pub struct TableDemo {}

impl super::Demo for TableDemo {
    fn name(&self) -> &'static str {
        "☰ Table"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                use super::View as _;
                self.ui(ui);
            });
    }
}

impl super::View for TableDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        TableBuilder::new(ui, Padding::new(2.0, 5.0))
            .striped(true)
            .column(Size::Absolute(100.0))
            .column(Size::RemainderMinimum(150.0))
            .column(Size::Absolute(50.0))
            .header(50.0, |mut header| {
                header.col(|ui| {
                    ui.label("Left");
                });
                header.col(|ui| {
                    ui.label("Middle");
                });
                header.col(|ui| {
                    ui.label("Right");
                });
            })
            .body(|mut body| {
                for i in 1..100 {
                    body.row(40.0, |mut row| {
                        row.col(|ui| {
                            ui.label(format!("{}", i));
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", i));
                        });
                        row.col(|ui| {
                            ui.label(format!("{}", i));
                        });
                    });
                }
            });
    }
}
