extern crate lazy_static;
extern crate serde;
extern crate serde_json;
extern crate wasm_bindgen;
extern crate web_sys;

#[macro_use]
extern crate serde_derive;

use std::sync::Mutex;

use wasm_bindgen::prelude::*;

mod types;

use types::*;

struct App {
    count: i32,
}

impl App {
    fn new() -> Self {
        App { count: 0 }
    }

    fn show_gui(&mut self, input: &Input) -> Vec<PaintCmd> {
        let rect = Rect {
            pos: Vec2 { x: 100.0, y: 100.0 },
            size: Vec2 { x: 200.0, y: 200.0 },
        };

        let is_hovering = rect.contains(&input.mouse_pos);

        vec![
            PaintCmd::Clear {
                fill_style: "#44444400".to_string(),
            },
            PaintCmd::Text {
                fill_style: "#11ff00".to_string(),
                font: "14px Palatino".to_string(),
                pos: Vec2 { x: 200.0, y: 32.0 },
                text: format!(
                    "Mouse pos: {} {}, is_hovering: {}",
                    input.mouse_pos.x, input.mouse_pos.y, is_hovering
                ),
                text_align: TextAlign::Center,
            },
            PaintCmd::Text {
                fill_style: "#11ff00".to_string(),
                font: "14px Palatino".to_string(),
                pos: Vec2 { x: 200.0, y: 64.0 },
                text: format!("Count: {}", self.count),
                text_align: TextAlign::Center,
            },
            PaintCmd::RoundedRect {
                fill_style: "#1111ff".to_string(),
                pos: rect.pos,
                corner_radius: 40.0,
                size: rect.size,
            },
        ]
    }
}

#[wasm_bindgen]
pub fn show_gui(input_json: &str) -> String {
    lazy_static::lazy_static! {
        static ref APP: Mutex<App> = Mutex::new(App::new());
    }

    // TODO: faster interface than JSON
    let input: Input = serde_json::from_str(input_json).unwrap();
    let commands = APP.lock().unwrap().show_gui(&input);
    serde_json::to_string(&commands).unwrap()
}
