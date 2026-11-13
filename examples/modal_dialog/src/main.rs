#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([480.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Modal dialog example",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

#[derive(Default)]
struct MyApp {
    /// How many frames the main window has painted.
    /// On Windows, while the dialog is open the main window ignores input, and the deferred
    /// dialog repaints on its own, so this stays still however much the dialog is used.
    frames: u64,

    /// Shared with the deferred dialog viewport, hence the `Arc`.
    show_dialog: Arc<AtomicBool>,

    /// What the user answered the last time.
    answer: Arc<egui::mutex::Mutex<Option<bool>>>,
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frames += 1;

        egui::CentralPanel::default().show(ui, |ui| {
            ui.label(format!("Frames painted by this window: {}", self.frames));

            if ui.button("Open modal dialog").clicked() {
                self.show_dialog.store(true, Ordering::Relaxed);
            }

            match *self.answer.lock() {
                Some(true) => ui.label("The dialog was answered with OK."),
                Some(false) => ui.label("The dialog was cancelled."),
                None => ui.label("No answer yet."),
            };
        });

        if self.show_dialog.load(Ordering::Relaxed) {
            let show_dialog = Arc::clone(&self.show_dialog);
            let answer = Arc::clone(&self.answer);
            let owner = ui.viewport_id();
            // A deferred viewport is repainted independently of this window:
            ui.ctx().show_viewport_deferred(
                egui::ViewportId::from_hash_of("modal_dialog"),
                egui::ViewportBuilder::default()
                    .with_title("Modal dialog")
                    .with_inner_size([320.0, 140.0])
                    .with_resizable(false)
                    .with_owner(owner)
                    .with_modal(true),
                move |ui, class| {
                    let ctx = ui.ctx().clone();
                    let mut close = |result: Option<bool>| {
                        if result.is_some() {
                            *answer.lock() = result;
                        }
                        show_dialog.store(false, Ordering::Relaxed);
                        // The owner stops showing the dialog (and shows the answer) when it repaints:
                        ctx.request_repaint_of(owner);
                    };

                    if class == egui::ViewportClass::EmbeddedWindow {
                        // No native window available: block the main window with a `Modal` instead.
                        let response = egui::Modal::new(egui::Id::unique("modal_dialog_embedded"))
                            .show(ui.ctx(), |ui| dialog_contents(ui, &mut close));
                        if response.should_close() {
                            close(None);
                        }
                    } else {
                        egui::CentralPanel::default().show(ui, |ui| {
                            dialog_contents(ui, &mut close);
                        });
                        if ui.input(|i| i.viewport().close_requested()) {
                            // The OS close button: stop showing the dialog.
                            close(None);
                        }
                    }
                },
            );
        }
    }
}

/// The question and the buttons. Calls `close` with the answer when a button is clicked.
fn dialog_contents(ui: &mut egui::Ui, close: &mut impl FnMut(Option<bool>)) {
    ui.label("Do you want to continue?");
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        if ui.button("OK").clicked() {
            close(Some(true));
        }
        if ui.button("Cancel").clicked() {
            close(Some(false));
        }
    });
}
