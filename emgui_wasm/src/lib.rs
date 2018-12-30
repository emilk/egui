#![deny(warnings)]

extern crate lazy_static;
extern crate serde_json;
extern crate wasm_bindgen;
// extern crate web_sys;

extern crate emgui;

use std::sync::Mutex;

use emgui::{Emgui, RawInput};

use wasm_bindgen::prelude::*;

pub mod app;

#[wasm_bindgen]
pub fn show_gui(raw_input_json: &str) -> String {
    // TODO: faster interface than JSON
    let raw_input: RawInput = serde_json::from_str(raw_input_json).unwrap();

    lazy_static::lazy_static! {
        static ref APP: Mutex<app::App> = Default::default();
        static ref EMGUI: Mutex<Emgui> = Default::default();
    }

    let mut emgui = EMGUI.lock().unwrap();
    emgui.new_frame(raw_input);

    use crate::app::GuiSettings;
    APP.lock().unwrap().show_gui(&mut emgui.layout);

    let mut style = emgui.style.clone();
    emgui.layout.foldable("Style", |gui| {
        style.show_gui(gui);
    });
    emgui.style = style;

    let commands = emgui.paint();
    serde_json::to_string(&commands).unwrap()
}
