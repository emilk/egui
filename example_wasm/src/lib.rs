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

    frame_times: std::collections::VecDeque<f64>,
}

impl State {
    fn new(canvas_id: &str, pixels_per_point: f32) -> Result<State, JsValue> {
        Ok(State {
            example_app: Default::default(),
            emigui: Emigui::new(pixels_per_point),
            webgl_painter: emigui_wasm::webgl::Painter::new(canvas_id)?,
            frame_times: Default::default(),
        })
    }

    fn run(&mut self, raw_input: RawInput) -> Result<Output, JsValue> {
        let everything_start = now_sec();

        self.emigui.begin_frame(raw_input);

        let mut region = self.emigui.background_region();
        let mut region = region.centered_column(region.available_width().min(480.0));
        region.set_align(Align::Min);
        region.add(label!("Emigui!").text_style(TextStyle::Heading));
        region.add_label("Emigui is an immediate mode GUI written in Rust, compiled to WebAssembly, rendered with WebGL.");
        region.add_label(
            "Everything you see is rendered as textured triangles. There is no DOM. There are no HTML elements."
        );
        region.add_label("This is not JavaScript. This is Rust, running at 60 FPS. This is the web page, reinvented with game tech.");
        region.add_label("This is also work in progress, and not ready for production... yet :)");
        region.horizontal(Align::Min, |region| {
            region.add_label("Project home page:");
            region.add_hyperlink("https://github.com/emilk/emigui/");
        });
        region.add(Separator::new());

        region.set_align(Align::Min);
        region.add_label("WebGl painter info:");
        region.indent(Id::new("webgl region"), |region| {
            region.add_label(self.webgl_painter.debug_info());
        });

        let mean_frame_time = if self.frame_times.is_empty() {
            0.0
        } else {
            self.frame_times.iter().sum::<f64>() / (self.frame_times.len() as f64)
        };
        region.add(
            label!("Total CPU usage: {:.1} ms", 1e3 * mean_frame_time)
                .text_style(TextStyle::Monospace),
        );

        // TODO: Make it even simpler to show a window

        Window::new("Examples")
            .default_pos(pos2(32.0, 300.0))
            .default_size(vec2(300.0, 400.0))
            .show(region.ctx(), |region| {
                self.example_app.ui(region);
            });

        Window::new("Emigui settings")
            .default_pos(pos2(400.0, 300.0))
            .default_size(vec2(400.0, 400.0))
            .show(region.ctx(), |region| {
                self.emigui.ui(region);
            });

        let bg_color = srgba(16, 16, 16, 255);
        let (output, batches) = self.emigui.end_frame();
        self.webgl_painter.paint_batches(
            bg_color,
            batches,
            self.emigui.texture(),
            raw_input.pixels_per_point,
        )?;

        self.frame_times.push_back(now_sec() - everything_start);
        while self.frame_times.len() > 30 {
            self.frame_times.pop_front();
        }

        Ok(output)
    }
}

#[wasm_bindgen]
pub fn new_webgl_gui(canvas_id: &str, pixels_per_point: f32) -> Result<State, JsValue> {
    State::new(canvas_id, pixels_per_point)
}

#[wasm_bindgen]
pub fn run_gui(state: &mut State, raw_input_json: &str) -> Result<String, JsValue> {
    // TODO: nicer interface than JSON
    let raw_input: RawInput = serde_json::from_str(raw_input_json).unwrap();
    let output = state.run(raw_input)?;
    Ok(serde_json::to_string(&output).unwrap())
}
