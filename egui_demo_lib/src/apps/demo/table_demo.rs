use egui::RichText;
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
        // TODO: Fix table as a padding smaller than 16 grows the window
        TableBuilder::new(ui, Padding::new(3.0, 16.0))
            .striped(true)
            .column(Size::Absolute(100.0))
            .column(Size::RemainderMinimum(150.0))
            .column(Size::Absolute(50.0))
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new("Left").heading());
                });
                header.col(|ui| {
                    ui.label(RichText::new("Middle").heading());
                });
                header.col(|ui| {
                    ui.label(RichText::new("Right").heading());
                });
            })
            .body(|mut body| {
                for i in 0..100 {
                    body.row(20.0, |mut row| {
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
