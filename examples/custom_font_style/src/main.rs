#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::{
    self,
    text::style::{FontId, GenericFamily},
};
use egui::{RichText, TextStyle};
use std::collections::BTreeMap;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "egui example: global font style",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}

#[inline]
fn heading2() -> TextStyle {
    TextStyle::Name("Heading2".into())
}

#[inline]
fn heading3() -> TextStyle {
    TextStyle::Name("ContextHeading".into())
}

fn configure_text_styles(ctx: &egui::Context) {
    use GenericFamily::{Monospace, SystemUi};

    let text_styles: BTreeMap<TextStyle, FontId> = [
        (TextStyle::Heading, FontId::simple(25.0, SystemUi)),
        (heading2(), FontId::simple(22.0, SystemUi)),
        (heading3(), FontId::simple(19.0, SystemUi)),
        (TextStyle::Body, FontId::simple(16.0, SystemUi)),
        (TextStyle::Monospace, FontId::simple(12.0, Monospace)),
        (TextStyle::Button, FontId::simple(12.0, SystemUi)),
        (TextStyle::Small, FontId::simple(8.0, SystemUi)),
    ]
    .into();
    ctx.all_styles_mut(move |style| style.text_styles = text_styles.clone());
}

fn content(ui: &mut egui::Ui) {
    ui.heading("Top Heading");
    ui.add_space(5.);
    ui.label(LOREM_IPSUM);
    ui.add_space(15.);
    ui.label(RichText::new("Sub Heading").text_style(heading2()).strong());
    ui.monospace(LOREM_IPSUM);
    ui.add_space(15.);
    ui.label(RichText::new("Context").text_style(heading3()).strong());
    ui.add_space(5.);
    ui.label(LOREM_IPSUM);
}

struct MyApp;

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_text_styles(&cc.egui_ctx);
        Self
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, content);
    }
}

pub const LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";
