#![deny(warnings)]
#![warn(clippy::all)]

use egui::{label, Align, Layout, TextStyle};

use wasm_bindgen::prelude::*;

// ----------------------------------------------------------------------------

/// This is the entry-point for all the web-assembly.
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
    let backend = egui_web::Backend::new(canvas_id, egui_web::RunMode::Reactive)?;
    let app = Box::new(MyApp::default());
    let runner = egui_web::AppRunner::new(backend, app)?;
    egui_web::run(runner)?;
    Ok(())
}

// ----------------------------------------------------------------------------

#[derive(Default)]
pub struct MyApp {
    example_app: egui::examples::ExampleApp,
    frames_painted: u64,
}

impl egui_web::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, backend: &mut egui_web::Backend, info: &egui_web::WebInfo) {
        self.example_app.ui(ui, &info.web_location_hash);

        let mut ui = ui.centered_column(ui.available().width().min(480.0));
        ui.set_layout(Layout::vertical(Align::Min));
        ui.add(label!("Egui!").text_style(TextStyle::Heading));
        ui.label("Egui is an immediate mode GUI written in Rust, compiled to WebAssembly, rendered with WebGL.");
        ui.label(
            "Everything you see is rendered as textured triangles. There is no DOM. There are no HTML elements."
        );
        ui.label("This is not JavaScript. This is Rust, running at 60 FPS. This is the web page, reinvented with game tech.");
        ui.label("This is also work in progress, and not ready for production... yet :)");
        ui.horizontal(|ui| {
            ui.label("Project home page:");
            ui.hyperlink("https://github.com/emilk/emigui/");
        });
        ui.separator();

        ui.label("WebGl painter info:");
        ui.indent("webgl region id", |ui| {
            ui.label(&backend.painter_debug_info());
        });

        ui.add(
            label!(
                "CPU usage: {:.2} ms / frame (excludes painting)",
                1e3 * backend.cpu_time()
            )
            .text_style(TextStyle::Monospace),
        );

        ui.separator();

        ui.horizontal(|ui| {
            let mut run_mode = backend.run_mode();
            ui.label("Run mode:");
            ui.radio_value("Continuous", &mut run_mode, egui_web::RunMode::Continuous)
                .tooltip_text("Repaint everything each frame");
            ui.radio_value("Reactive", &mut run_mode, egui_web::RunMode::Reactive)
                .tooltip_text("Repaint when there are animations or input (e.g. mouse movement)");
            backend.set_run_mode(run_mode);
        });

        if backend.run_mode() == egui_web::RunMode::Continuous {
            ui.add(
                label!("Repainting the UI each frame. FPS: {:.1}", backend.fps())
                    .text_style(TextStyle::Monospace),
            );
        } else {
            ui.label("Only running UI code when there are animations or input");
        }

        self.frames_painted += 1;
        ui.label(format!("Total frames painted: {}", self.frames_painted));
    }
}
