extern crate lazy_static;
extern crate serde;
extern crate serde_json;
extern crate wasm_bindgen;
extern crate web_sys;

#[macro_use]
extern crate serde_derive;

use std::sync::Mutex;

use wasm_bindgen::prelude::*;

use types::*;

pub mod app;
pub mod gui;
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
        static ref APP: Mutex<app::App> = Mutex::new(app::App::new());
        static ref LAST_INPUT: Mutex<RawInput> = Default::default();
    }

    let gui_input = GuiInput::from_last_and_new(&LAST_INPUT.lock().unwrap(), &raw_input);
    *LAST_INPUT.lock().unwrap() = raw_input;

    let mut gui = gui::Gui::new(gui_input);
    APP.lock().unwrap().show_gui(&mut gui);
    let commands = gui.into_commands();
    serde_json::to_string(&commands).unwrap()
}
