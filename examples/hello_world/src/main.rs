#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::sync::Arc;

use eframe::{
    egui::{self, remap_clamp, vec2, Color32, Sense, Shape},
    epaint::PathStroke,
};

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::<MyApp>::default()
        }),
    )
}

struct MyApp {
    name: String,
    age: u32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                self.age += 1;
            }
            ui.label(format!("Hello '{}', age {}", self.name, self.age));

            ui.image(egui::include_image!(
                "../../../crates/egui/assets/ferris.png"
            ));

            let (rect, _) =
                ui.allocate_exact_size(vec2(100.0, 50.0), Sense::focusable_noninteractive());

            let stroke = PathStroke {
                width: 1.5,
                color: egui::epaint::ColorMode::UV(Arc::new(Box::new(|r, p| {
                    let t = remap_clamp(p.x, r.x_range(), 0.0..=1.0);
                    if t < 0.5 {
                        Color32::RED
                    } else {
                        Color32::GREEN
                    }
                }))),
            };

            ui.painter_at(rect).add(Shape::line_segment(
                [rect.left_center(), rect.right_center()],
                stroke.clone(),
            ));

            ui.painter_at(rect).add(Shape::line_segment(
                [
                    rect.left_center() + vec2(25.0, 15.0),
                    rect.right_center() + vec2(0.0, 15.0),
                ],
                stroke.clone(),
            ));
        });
    }
}
