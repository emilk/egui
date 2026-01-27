#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs, clippy::unwrap_used)] // it's an example

use eframe::{UserEvent, egui};
use std::{cell::Cell, rc::Rc};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let eventloop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    eventloop.set_control_flow(ControlFlow::Poll);

    let mut winit_app = eframe::create_native(
        "External Eventloop Application",
        options,
        Box::new(|_| Ok(Box::<MyApp>::default())),
        &eventloop,
    );

    eventloop.run_app(&mut winit_app)?;

    Ok(())
}

struct MyApp {
    value: Rc<Cell<u32>>,
    spin: bool,
    blinky: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            value: Rc::new(Cell::new(42)),
            spin: false,
            blinky: false,
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("My External Eventloop Application");

            ui.horizontal(|ui| {
                if ui.button("Increment Now").clicked() {
                    self.value.set(self.value.get() + 1);
                }
            });
            ui.label(format!("Value: {}", self.value.get()));

            if ui.button("Toggle Spinner").clicked() {
                self.spin = !self.spin;
            }

            if ui.button("Toggle Blinky").clicked() {
                self.blinky = !self.blinky;
            }

            if self.spin {
                ui.spinner();
            }

            if self.blinky {
                let now = ui.input(|i| i.time);
                let blink = now % 1.0 < 0.5;
                egui::Frame::new()
                    .inner_margin(3)
                    .corner_radius(5)
                    .fill(if blink {
                        egui::Color32::RED
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .show(ui, |ui| {
                        ui.label("Blinky!");
                    });

                ui.request_repaint_after_secs((0.5 - (now % 0.5)) as f32);
            }
        });
    }
}
