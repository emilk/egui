#![deny(warnings)]

extern crate serde_json;
extern crate wasm_bindgen;

extern crate emgui;

use std::sync::Arc;

use emgui::{Emgui, Font, RawInput};

use wasm_bindgen::prelude::*;

mod app;
mod webgl;

#[derive(Clone, Copy, Default)]
struct Stats {
    num_vertices: usize,
    num_triangles: usize,
    everything_ms: f64,
    webgl_ms: f64,
}

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
    emgui: Emgui,
    webgl_painter: webgl::Painter,
    stats: Stats,
}

impl State {
    fn new(canvas_id: &str) -> Result<State, JsValue> {
        let font = Arc::new(Font::new(20));
        let emgui = Emgui::new(font);
        let webgl_painter = webgl::Painter::new(canvas_id, emgui.texture())?;
        Ok(State {
            app: Default::default(),
            emgui,
            webgl_painter,
            stats: Default::default(),
        })
    }

    fn run(&mut self, raw_input: RawInput) -> Result<(), JsValue> {
        let everything_start = now_ms();

        self.emgui.new_frame(raw_input);

        use crate::app::GuiSettings;

        let mut style = self.emgui.style.clone();
        let mut region = self.emgui.whole_screen_region();
        let mut region = region.centered_column(300.0);
        self.app.show_gui(&mut region);

        // TODO: move this to some emgui::example module
        region.foldable("Style", |gui| {
            style.show_gui(gui);
        });

        let stats = self.stats; // TODO: avoid
        let webgl_info = self.webgl_painter.debug_info(); // TODO: avoid
        region.foldable("Stats", |gui| {
            gui.label(format!("num_vertices: {}", stats.num_vertices));
            gui.label(format!("num_triangles: {}", stats.num_triangles));

            gui.label("WebGl painter info:");
            gui.indent(|gui| {
                gui.label(webgl_info);
            });

            gui.label("Timings:");
            gui.indent(|gui| {
                gui.label(format!("Everything: {:.1} ms", stats.everything_ms));
                gui.label(format!("WebGL: {:.1} ms", stats.webgl_ms));
            });
        });

        self.emgui.style = style;
        let frame = self.emgui.paint();

        self.stats.num_vertices = frame.vertices.len();
        self.stats.num_triangles = frame.indices.len() / 3;

        let webgl_start = now_ms();
        let result = self.webgl_painter.paint(&frame);
        self.stats.webgl_ms = now_ms() - webgl_start;

        self.stats.everything_ms = now_ms() - everything_start;

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
