#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's a test

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Use `RUST_LOG=debug` to see logs.

    let options = eframe::NativeOptions::default();
    eframe::run_ui_native("My egui App", options, move |ui, _frame| {
        // A bottom panel to force the tooltips to consider if the fit below or under the widget:
        egui::Panel::bottom("bottom").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Single tooltips:");
                    for i in 0..3 {
                        ui.label(format!("Hover label {i} for a tooltip"))
                            .on_hover_text("There is some text here");
                    }
                });
                ui.vertical(|ui| {
                    ui.label("Double tooltips:");
                    for i in 0..3 {
                        ui.label(format!("Hover label {i} for two tooltips"))
                            .on_hover_text("First tooltip")
                            .on_hover_text("Second tooltip");
                    }
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                ui.label("Hover for tooltip")
                    .on_hover_text("This is a rather long tooltip that needs careful positioning.");
            });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Reset egui memory").clicked() {
                    ui.memory_mut(|mem| *mem = Default::default());
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::BOTTOM), |ui| {
                    ui.label("Hover for tooltip").on_hover_text(
                        "This is a rather long tooltip that needs careful positioning.",
                    );
                    ui.label("Hover for interactive tooltip").on_hover_ui(|ui| {
                        ui.label("This tooltip has a button:");
                        let _ = ui.button("Clicking me does nothing");
                    });
                });
            });

            let has_tooltip = ui
                .label("This label has a tooltip at the mouse cursor")
                .on_hover_text_at_pointer("Told you!")
                .is_tooltip_open();

            let response = ui.label("This label gets a tooltip when the previous label is hovered");
            if has_tooltip {
                response.show_tooltip_text("The ever-present tooltip!");
            }

            ui.separator();

            ui.label("The menu should be as wide as the widest button");
            ui.menu_button("Click for menu", |ui| {
                let _ = ui.button("Narrow").clicked();
                let _ = ui.button("Very wide text").clicked();
                let _ = ui.button("Narrow").clicked();
            });

            ui.label("Hover for tooltip").on_hover_ui(|ui| {
                ui.label("A separator:");
                ui.separator();
            });

            ui.separator();

            let alternatives = [
                "Short",
                "Min",
                "Very very long text that will extend",
                "Short",
            ];
            let mut selected = 1;

            egui::ComboBox::from_label("ComboBox").show_index(
                ui,
                &mut selected,
                alternatives.len(),
                |i| alternatives[i],
            );

            egui::ComboBox::from_id_salt("combo")
                .selected_text("ComboBox")
                .width(100.0)
                .show_ui(ui, |ui| {
                    ui.debug_painter()
                        .debug_rect(ui.max_rect(), egui::Color32::RED, "");

                    ui.label("Hello");
                    ui.label("World");
                    ui.label("Hellooooooooooooooooooooooooo");
                });

            ui.separator();

            let time = ui.input(|i| i.time);
            ui.label("Hover for a tooltip with changing content")
                .on_hover_text(format!("A number: {}", time % 10.0));
        });
    })
}
