#![deny(warnings)]
#![warn(clippy::all)]

use egui::{examples::ExampleApp, label, widgets::Separator, Align, RawInput, TextStyle, *};

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct State {
    egui_web: egui_web::State,
    example_app: ExampleApp,
}

impl State {
    fn new(canvas_id: &str) -> Result<State, JsValue> {
        Ok(State {
            egui_web: egui_web::State::new(canvas_id)?,
            example_app: Default::default(),
        })
    }

    fn run(&mut self, raw_input: RawInput, web_location_hash: &str) -> Result<Output, JsValue> {
        let mut ui = self.egui_web.begin_frame(raw_input);
        self.ui(&mut ui, web_location_hash);
        self.egui_web.end_frame()
    }

    fn ui(&mut self, ui: &mut egui::Ui, web_location_hash: &str) {
        self.example_app.ui(ui, web_location_hash);
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
        ui.add(Separator::new());

        ui.label("WebGl painter info:");
        ui.indent("webgl region id", |ui| {
            ui.label(self.egui_web.painter_debug_info());
        });

        ui.add(
            label!(
                "CPU usage: {:.2} ms (excludes painting)",
                1e3 * self.egui_web.cpu_usage()
            )
            .text_style(TextStyle::Monospace),
        );
        ui.add(label!("FPS: {:.1}", self.egui_web.fps()).text_style(TextStyle::Monospace));
    }
}

#[wasm_bindgen]
pub fn new_webgl_gui(canvas_id: &str) -> Result<State, JsValue> {
    State::new(canvas_id)
}

#[wasm_bindgen]
pub fn resize_to_screen_size(canvas_id: &str) {
    egui_web::resize_to_screen_size(canvas_id);
}

#[wasm_bindgen]
pub fn run_gui(state: &mut State, web_input_json: &str) -> Result<(), JsValue> {
    // TODO: nicer interface than JSON
    let raw_input: RawInput = serde_json::from_str(web_input_json).unwrap();
    let web_location_hash = egui_web::location_hash().unwrap_or_default();
    let output = state.run(raw_input, &web_location_hash)?;
    egui_web::handle_output(&output);
    Ok(())
}
