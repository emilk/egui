#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! Render text glyphs from SVG files with [`egui_extras::SvgGlyph`].
//!
//! A gray SVG is tinted with the text color, like a normal glyph.
//! A colorful SVG keeps its colors, like a color emoji.

use eframe::egui::{self, Color32, RichText};
use egui_extras::SvgGlyph;

/// A private use character, rendered as Ferris.
const FERRIS: char = '\u{E000}';

/// A regular character whose glyph we override.
const HEART: char = '♥';

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([420.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui example: SVG glyphs",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)?))),
    )
}

/// Render `♥` and a private use character from SVG files.
fn install_svg_glyphs(ctx: &egui::Context) -> Result<(), String> {
    // The heart is white, so it is tinted with the text color.
    // It is a bit smaller than the font size, and lifted slightly above the baseline:
    let heart = SvgGlyph::from_bytes(include_bytes!("../heart.svg"))?
        .with_height(0.7)
        .with_baseline_offset(0.05);
    ctx.add_glyph_rasterizer(heart.into_rasterizer(HEART.to_string()));

    // Ferris is colorful, so the colors are kept:
    let ferris = SvgGlyph::from_bytes(include_bytes!("../../images/src/ferris.svg"))?;
    ctx.add_glyph_rasterizer(ferris.into_rasterizer(FERRIS.to_string()));

    Ok(())
}

struct MyApp {
    font_size: f32,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        install_svg_glyphs(&cc.egui_ctx)?;
        Ok(Self { font_size: 32.0 })
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(format!("I {HEART} Rust {FERRIS}"));
            ui.label(format!(
                "The heart is an SVG tinted with the text color. \
                 Ferris is a color SVG on the private use character U+{:04X}.",
                FERRIS as u32
            ));
            ui.add(egui::Slider::new(&mut self.font_size, 8.0..=128.0).text("Font size"));

            ui.horizontal_wrapped(|ui| {
                for color in [Color32::RED, Color32::GREEN, Color32::LIGHT_BLUE] {
                    ui.label(
                        RichText::new(format!("{HEART}{FERRIS}"))
                            .color(color)
                            .size(self.font_size),
                    );
                }
                ui.label(
                    RichText::new(format!("{HEART}{FERRIS}"))
                        .monospace()
                        .size(self.font_size),
                );
            });

            let _ = ui.button(format!("{HEART} Button"));
        });
    }
}
