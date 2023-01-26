#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{
    egui::{self, layers::ZOrder, Sense, Ui},
    epaint::{Color32, Rect, Rounding, Vec2},
};

fn main() -> Result<(), eframe::Error> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(220.0, 150.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Z Order Test",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    )
}

#[derive(Default)]
struct MyApp {}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("The left example should show blue over green. The right should show green over blue.");

            egui::Frame::default()
                .fill(ui.style().visuals.window_fill)
                .inner_margin(10.0)
                .show(ui, |ui| {
                    const SIZE: Vec2 = Vec2::new(50.0, 50.0);
                    const DELTA: Vec2 = Vec2::new(20.0, 20.0);

                    fn draw_squares(
                        ui: &mut Ui,
                        shift: Vec2,
                        order1: ZOrder,
                        order2: ZOrder,
                    ) {
                        let pos = ui.max_rect().left_top();

                        ui.with_z(order1, |ui| {
                            let response = ui.allocate_rect(
                                Rect::from_min_size(pos + shift, SIZE),
                                Sense::click(),
                            );

                            let painter = ui.painter_at(response.rect);
                            painter.rect_filled(response.rect, Rounding::none(), Color32::GREEN);

                            if response.clicked() {
                                log::info!("Clicked green");
                            }
                        });

                        ui.with_z(order2, |ui| {
                            let response = ui.allocate_rect(
                                Rect::from_min_size(pos + DELTA + shift, SIZE),
                                Sense::click(),
                            );
                            let painter = ui.painter_at(response.rect);
                            painter.rect_filled(response.rect, Rounding::none(), Color32::BLUE);

                            if response.clicked() {
                                log::info!("Clicked blue");
                            }
                        });
                    }

                    let base = ZOrder::BASE;
                    let above = base.in_front();

                    draw_squares(ui, Vec2::new(0.0, 0.0), base, above);
                    draw_squares(ui, Vec2::new(100.0, 0.0), above, base);
                })
        });
    }
}
