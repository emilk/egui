#!/usr/bin/env cargo
//! This demonstrates the new galley-free selection cursor API for labels.
//!
//! This example shows how to get detailed cursor and selection information
//! without needing to create or manage galleys, making it much more efficient.
//!
//! Run with:
//! ```sh
//! cargo run -p text_selection_api
//! ```

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Galley-Free Selection Cursor API Demo",
        options,
        Box::new(|_cc| Ok(Box::<SelectionCursorDemo>::default())),
    )
}

#[derive(Default)]
struct SelectionCursorDemo;

impl eframe::App for SelectionCursorDemo {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Text Selection Cursor API Demo");
            ui.separator();

            ui.label("Select text in the labels below to see cursor information:");
            ui.label("This demo demonstrates the API!");
            ui.separator();

            let label_texts = [
                "This is the first selectable label",
                "And this is the second one",
                "You can select across multiple labels!",
            ];

            ui.separator();

            ui.label("Selection Information:");

            let responses: Vec<_> = label_texts.iter().map(|text| {
                    ui.label(*text)
                })
                .collect();
            let any_selected = responses.iter().enumerate().fold(false, |acc, (i, r)| {
                    if r.has_text_selection() {
                        let label_num = i + 1;
                        ui.label(format!("Label {label_num}:"));
                        if let Some(cursor_range) = r.selected_cursor_range() {
                            ui.label(format!("  • Primary cursor: {}, Secondary cursor: {}",
                                cursor_range.primary.index, cursor_range.secondary.index));
                        }

                        if let Some(char_range) = r.selected_char_range() {
                            ui.label(format!("  • Character range: {}..{}", char_range.start, char_range.end));
                        }
                        if let Some(selected_text) = r.selected_text() {
                            ui.label(format!("  • Selected text: '{selected_text}'"));
                        }
                        true
                    } else {
                        acc
                    }
                });
            if !any_selected {
                ui.label("Select some text to see cursor information");
            }

            ui.separator();
            ui.label("Tips:");
            ui.label("• Try selecting text within a single label");
            ui.label("• Try selecting text across multiple labels");
            ui.label("• The primary cursor shows where selection ended");
            ui.label("• The secondary cursor shows where selection started");
            ui.label("• Partial selection types: Start (extends to end), End (extends from beginning)");
        });
    }
}
