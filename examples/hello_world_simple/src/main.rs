#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use eframe::egui::containers::menu::{MenuButton, MenuConfig};
use eframe::egui::{Align, Button, Layout, Popup, PopupCloseBehavior, TextWrapMode, Widget};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    // Our application state:
    let mut name = "Arthur".to_owned();
    let mut age = 42;

    let mut checked = true;

    let mut flag = false;
    let mut row_count = 5;

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        ctx.options_mut(|o| o.max_passes = 1.try_into().unwrap());
        egui::CentralPanel::default().show(ctx, |ui| {
            // std::thread::sleep(std::time::Duration::from_millis(100));
            // let response = ui.button("Hiiii");
            //
            // let text = if checked {
            //     "short"
            // } else {
            //     "Very long text for this item that should be wrapped"
            // };
            // Popup::from_response(&response)
            //     .layout(Layout::top_down_justified(Align::Min))
            //     .show(|ui| {
            //         // ui.checkbox(&mut checked, text);
            //
            //         ui.button("Hiiii");
            //
            //         ui.horizontal(|ui| {
            //             ui.vertical(|ui| {
            //                 if Button::new(text)
            //                     // .wrap_mode(TextWrapMode::Extend)
            //                     .ui(ui)
            //                     .clicked()
            //                 {
            //                     checked = !checked;
            //                 }
            //                 ui.button("Button1");
            //             });
            //
            //             ui.button("Button2")
            //         });
            //
            //         ui.button("Some other button");
            //     });

            // Antoines example

            //     ui.label(format!("Showing {} rows", row_count));
            //     if MenuButton::new("BTN")
            //         .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            //         .ui(ui, |ui| {
            //             ui.radio_value(&mut flag, false, "False");
            //             ui.radio_value(&mut flag, true, "True");
            //
            //             if flag {
            //                 egui::ScrollArea::vertical().show(ui, |ui| {
            //                     for _ in 0..row_count {
            //                         ui.add_space(30.0);
            //                         ui.label("Veeeeeeeeeeeery long text.");
            //                     }
            //                 });
            //             }
            //         })
            //         .0
            //         .clicked()
            //     {
            //         if row_count % 2 == 1 {
            //             row_count -= 3;
            //         } else {
            //             row_count += 5;
            //         }
            //     }

            MenuButton::new("Menu")
                .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
                .ui(ui, |ui| {
                    ui.set_max_width(180.0);
                    if ui.button("Close menu").clicked() {
                        ui.close_menu();
                    }
                    ui.collapsing("Collapsing", |ui| {
                        egui::ScrollArea::both().show(ui, |ui| {
                            // ui.set_width(ui.available_width());
                            for _ in 0..10 {
                                ui.label(
                                    "This is a long text label containing \
                                a lot of words and spans many lines",
                                );
                            }
                        });
                    });
                });
        });
    })
}
