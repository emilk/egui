#![deny(warnings)]

extern crate lazy_static;
extern crate serde;
extern crate serde_json;
extern crate wasm_bindgen;
extern crate web_sys;

#[macro_use] // TODO: get rid of this
extern crate serde_derive;

use std::sync::Mutex;

use wasm_bindgen::prelude::*;

use crate::types::*;

pub mod app;
pub mod emgui;
pub mod layout;
pub mod math;
pub mod style;
pub mod types;

/*
// Fast compilation, slow code:
fn foo(x: &dyn Trait);


// Fast code, slow compilation:
fn foo<T: Trait>(x: &dyn T);


// Compiles quickly in debug, fast in release:
#[dynimp(Trait)]
fn foo(x: &Trait);
*/

#[wasm_bindgen]
pub fn show_gui(raw_input_json: &str) -> String {
    // TODO: faster interface than JSON
    let raw_input: RawInput = serde_json::from_str(raw_input_json).unwrap();

    lazy_static::lazy_static! {
        static ref APP: Mutex<app::App> = Default::default();
        static ref EMGUI: Mutex<crate::emgui::Emgui> = Default::default();
    }

    let mut emgui = EMGUI.lock().unwrap();
    emgui.new_frame(raw_input);

    use crate::app::GuiSettings;
    APP.lock().unwrap().show_gui(&mut emgui.layout);

    let commands = emgui.paint();
    serde_json::to_string(&commands).unwrap()
}
