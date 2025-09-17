use eframe::egui::{Context, ImageSource};
use eframe::{Frame, NativeOptions, egui};
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "kitdiff",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App {}))),
    )
}

struct Snapshot {
    name: String,
    current: ImageSource<'static>,
    old: Option<ImageSource<'static>>,
    new: Option<ImageSource<'static>>,
}

struct App {
    snapshots: Vec<Snapshot>,
}

impl App {
    pub fn new() -> Self {
        /// TODO
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello, World!");
        });
    }
}
