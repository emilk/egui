extern crate serde;
extern crate serde_json;
extern crate wasm_bindgen;
extern crate web_sys;
#[macro_use]
extern crate serde_derive;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Deserialize)]
pub struct Input {
    pub screen_width: f32,
    pub screen_height: f32,
    pub mouse_x: f32,
    pub mouse_y: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum TextAlign {
    Start,
    Center,
    End,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum PaintCmd {
    Clear {
        fill_style: String,
    },
    RoundedRect {
        fill_style: String,
        pos: [f32; 2],
        size: [f32; 2],
        radius: f32,
    },
    Text {
        fill_style: String,
        font: String,
        pos: [f32; 2],
        text: String,
        text_align: TextAlign,
    },
}

#[wasm_bindgen]
pub fn show_gui(input_json: &str) -> String {
    let input: Input = serde_json::from_str(input_json).unwrap();
    let commands = [
        PaintCmd::Clear {
            fill_style: "#44444400".to_string(),
        },
        PaintCmd::RoundedRect {
            fill_style: "#1111ff".to_string(),
            pos: [100.0, 100.0],
            radius: 40.0,
            size: [200.0, 200.0],
        },
        PaintCmd::Text {
            fill_style: "#11ff00".to_string(),
            font: "14px Palatino".to_string(),
            pos: [200.0, 32.0],
            text: format!("Mouse pos: {} {}", input.mouse_x, input.mouse_y),
            text_align: TextAlign::Center,
        },
    ];
    serde_json::to_string(&commands).unwrap()
}
