#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eframe::epaint::text::{FontInsert, InsertFontFamily};

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui example: font variations",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}

struct MyApp {
    /// Weight axis (wght): 300..1000
    weight: f32,
    /// Casual axis (CASL): 0..1
    casual: f32,
    /// Monospace axis (MONO): 0..1
    mono: f32,
    /// Slant axis (slnt): -15..0
    slant: f32,
    /// Cursive axis (CRSV): 0..1
    cursive: f32,

    preview_text: String,
    font_size: f32,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.add_font(FontInsert::new(
            "Recursive",
            egui::FontData::from_static(include_bytes!("../data/Recursive-VariableFont.ttf")),
            vec![
                InsertFontFamily {
                    family: egui::FontFamily::Proportional,
                    priority: egui::epaint::text::FontPriority::Highest,
                },
                InsertFontFamily {
                    family: egui::FontFamily::Monospace,
                    priority: egui::epaint::text::FontPriority::Highest,
                },
            ],
        ));

        Self {
            weight: 400.0,
            casual: 0.0,
            mono: 0.0,
            slant: 0.0,
            cursive: 0.5,
            preview_text: "The quick brown fox jumps over the lazy dog.\n\
                           ABCDEFGHIJKLMNOPQRSTUVWXYZ\n\
                           abcdefghijklmnopqrstuvwxyz\n\
                           0123456789 !@#$%^&*()"
                .to_owned(),
            font_size: 24.0,
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Font Variations (Recursive)");
            ui.add_space(4.0);

            egui::Grid::new("variation_sliders")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Weight (wght):");
                    ui.add(egui::Slider::new(&mut self.weight, 300.0..=1000.0));
                    ui.end_row();

                    ui.label("Casual (CASL):");
                    ui.add(egui::Slider::new(&mut self.casual, 0.0..=1.0));
                    ui.end_row();

                    ui.label("Monospace (MONO):");
                    ui.add(egui::Slider::new(&mut self.mono, 0.0..=1.0));
                    ui.end_row();

                    ui.label("Slant (slnt):");
                    ui.add(egui::Slider::new(&mut self.slant, -15.0..=0.0));
                    ui.end_row();

                    ui.label("Cursive (CRSV):");
                    ui.add(egui::Slider::new(&mut self.cursive, 0.0..=1.0));
                    ui.end_row();

                    ui.label("Font size:");
                    ui.add(egui::Slider::new(&mut self.font_size, 8.0..=80.0));
                    ui.end_row();
                });

            ui.separator();

            let rich = egui::RichText::new(&self.preview_text)
                .size(self.font_size)
                .variation("wght", self.weight)
                .variation("CASL", self.casual)
                .variation("MONO", self.mono)
                .variation("slnt", self.slant)
                .variation("CRSV", self.cursive);

            ui.label(rich);

            ui.add_space(8.0);
            ui.text_edit_multiline(&mut self.preview_text);
        });
    }
}
