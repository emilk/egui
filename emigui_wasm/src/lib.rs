#![deny(warnings)]

extern crate serde_json;
extern crate wasm_bindgen;

extern crate emigui;

use std::sync::Arc;

use emigui::{widgets::label, Emigui, Font, RawInput};

use wasm_bindgen::prelude::*;

mod app;
mod webgl;

fn now_ms() -> f64 {
    web_sys::window()
        .expect("should have a Window")
        .performance()
        .expect("should have a Performance")
        .now()
}

#[wasm_bindgen]
pub struct State {
    app: app::App,
    emigui: Emigui,
    webgl_painter: webgl::Painter,
    everything_ms: f64,
}

impl State {
    fn new(canvas_id: &str) -> Result<State, JsValue> {
        let font = Arc::new(Font::new(20));
        let emigui = Emigui::new(font);
        let webgl_painter = webgl::Painter::new(canvas_id, emigui.texture())?;
        Ok(State {
            app: Default::default(),
            emigui,
            webgl_painter,
            everything_ms: 0.0,
        })
    }

    fn run(&mut self, raw_input: RawInput) -> Result<(), JsValue> {
        let everything_start = now_ms();

        self.emigui.new_frame(raw_input);

        let mut region = self.emigui.whole_screen_region();
        let mut region = region.centered_column(480.0);
        self.app.show_gui(&mut region);
        self.emigui.example(&mut region);

        region.add(label("WebGl painter info:"));
        region.indent(|region| {
            region.add(label(self.webgl_painter.debug_info()));
        });

        region.add(label(format!("Everything: {:.1} ms", self.everything_ms)));

        let frame = self.emigui.paint();
        let result = self.webgl_painter.paint(&frame);

        self.everything_ms = now_ms() - everything_start;

        result
    }
}

#[wasm_bindgen]
pub fn new_webgl_gui(canvas_id: &str) -> Result<State, JsValue> {
    State::new(canvas_id)
}

#[wasm_bindgen]
pub fn run_gui(state: &mut State, raw_input_json: &str) -> Result<(), JsValue> {
    // TODO: nicer interface than JSON
    let raw_input: RawInput = serde_json::from_str(raw_input_json).unwrap();
    state.run(raw_input)
}
