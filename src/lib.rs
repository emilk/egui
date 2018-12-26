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

use crate::{math::Vec2, types::*};

pub mod app;
pub mod gui;
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
        static ref LAST_INPUT: Mutex<RawInput> = Default::default();
        static ref GUI_STATE: Mutex<gui::GuiState> = Default::default();
    }

    let gui_input = GuiInput::from_last_and_new(&LAST_INPUT.lock().unwrap(), &raw_input);
    *LAST_INPUT.lock().unwrap() = raw_input;

    let mut gui = gui::Gui {
        commands: Vec::new(),
        cursor: Vec2 { x: 32.0, y: 32.0 },
        input: gui_input,
        state: *GUI_STATE.lock().unwrap(),
    };
    if !gui_input.mouse_down {
        gui.state.active_id = None;
    }
    APP.lock().unwrap().show_gui(&mut gui);

    *GUI_STATE.lock().unwrap() = gui.state;

    let commands = style::into_paint_commands(gui.gui_commands());
    serde_json::to_string(&commands).unwrap()
}
