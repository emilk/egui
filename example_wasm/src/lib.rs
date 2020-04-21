#![deny(warnings)]

extern crate serde_json;
extern crate wasm_bindgen;

extern crate emigui;
extern crate emigui_wasm;

use {
    emigui::{
        color::srgba,
        example_app::ExampleApp,
        label,
        widgets::{Label, Separator},
        Align, Emigui, RawInput, TextStyle, Window, *,
    },
    emigui_wasm::now_sec,
};

use wasm_bindgen::prelude::*;
#[wasm_bindgen]

pub struct State {
    example_app: ExampleApp,
    emigui: Emigui,
    webgl_painter: emigui_wasm::webgl::Painter,
    everything_ms: f64,
}

impl State {
    fn new(canvas_id: &str, pixels_per_point: f32) -> Result<State, JsValue> {
        Ok(State {
            example_app: Default::default(),
            emigui: Emigui::new(pixels_per_point),
            webgl_painter: emigui_wasm::webgl::Painter::new(canvas_id)?,
            everything_ms: 0.0,
        })
    }

    fn run(&mut self, raw_input: RawInput) -> Result<(), JsValue> {
        let everything_start = now_sec();

        self.emigui.new_frame(raw_input);

        let mut region = self.emigui.background_region();
        let mut region = region.centered_column(region.available_width().min(480.0));
        region.set_align(Align::Min);
        region.add(label!("Emigui!").text_style(TextStyle::Heading));
        region.add_label("Emigui is an immediate mode GUI written in Rust, compiled to WebAssembly, rendered with WebGL.");
        region.add_label(
            "Everything you see is rendered as textured triangles. There is no DOM. There are no HTML elements."
        );
        region.add_label("This not JavaScript. This is Rust code, running at 60 Hz. This is the web page, reinvented with game tech.");
        region.add_label("This is also work in progress, and not ready for production... yet :)");
        region.add(Separator::new());
        self.example_app.ui(&mut region);
        self.emigui.ui(&mut region);

        region.set_align(Align::Min);
        region.add_label("WebGl painter info:");
        region.indent(Id::new(&"webgl region"), |region| {
            region.add_label(self.webgl_painter.debug_info());
        });
        region.add(
            label!("Everything: {:.1} ms", self.everything_ms).text_style(TextStyle::Monospace),
        );

        // TODO: Make it even simpler to show a window

        Window::new("Test window").show(region.ctx(), |region| {
            region.add_label("Grab the window and move it around!");

            region.add_label("This window can be reisized, but not smaller than the contents.");
        });
        Window::new("Another test window")
            .default_pos(pos2(400.0, 100.0))
            .show(region.ctx(), |region| {
                region.add_label("This might be on top of the other window?");
                region.add_label("Second line of text");
            });

        let bg_color = srgba(16, 16, 16, 255);
        let batches = self.emigui.paint();
        let result = self.webgl_painter.paint_batches(
            bg_color,
            batches,
            self.emigui.texture(),
            raw_input.pixels_per_point,
        );

        self.everything_ms = 1000.0 * (now_sec() - everything_start);

        result
    }
}

#[wasm_bindgen]
pub fn new_webgl_gui(canvas_id: &str, pixels_per_point: f32) -> Result<State, JsValue> {
    State::new(canvas_id, pixels_per_point)
}

#[wasm_bindgen]
pub fn run_gui(state: &mut State, raw_input_json: &str) -> Result<(), JsValue> {
    // TODO: nicer interface than JSON
    let raw_input: RawInput = serde_json::from_str(raw_input_json).unwrap();
    state.run(raw_input)
}
