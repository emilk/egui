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

/*

// Fast compilation, slow code:
fn foo(x: &dyn Trait);


// Fast code, slow compilation:
fn foo<T: Trait>(x: &dyn T);


// Compiles quickly in debug, fast in release:
#[dynimp(Trait)]
fn foo(x: &Trait);
*/

#[derive(Default)]
pub struct InteractInfo {
    pub is_hovering: bool,
}

// TODO: implement Gui on this so we can add children to a widget
// pub struct Widget {}

pub struct Gui {
    commands: Vec<PaintCmd>,
    input: Input,
}

impl Gui {
    pub fn new(input: Input) -> Self {
        Gui {
            commands: vec![PaintCmd::Clear {
                fill_style: "#44444400".to_string(),
            }],
            input,
        }
    }

    pub fn input(&self) -> &Input {
        &self.input
    }

    pub fn into_commands(self) -> Vec<PaintCmd> {
        self.commands
    }

    pub fn rect(&mut self, rect: Rect) -> InteractInfo {
        let ii = InteractInfo {
            is_hovering: rect.contains(&self.input.mouse_pos),
        };
        self.commands.push(PaintCmd::RoundedRect {
            fill_style: "#ffffff10".to_string(),
            pos: rect.pos,
            corner_radius: 40.0,
            size: rect.size,
        });
        ii
    }

    pub fn text(&mut self, pos: Vec2, text: String) {
        self.commands.push(PaintCmd::Text {
            fill_style: "#11ff00".to_string(),
            font: "14px Palatino".to_string(),
            pos,
            text,
            text_align: TextAlign::Start,
        });
    }
}

struct App {
    count: i32,
}

impl App {
    fn new() -> Self {
        App { count: 0 }
    }

    fn show_gui(&mut self, gui: &mut Gui, input: &Input) {
        gui.rect(Rect {
            pos: Vec2 { x: 0.0, y: 0.0 },
            size: input.screen_size,
        });

        gui.rect(Rect {
            pos: Vec2 { x: 50.0, y: 50.0 },
            size: Vec2 {
                x: (input.screen_size.x - 100.0) / 3.0,
                y: (input.screen_size.y - 100.0),
            },
        });

        let is_hovering = gui
            .rect(Rect {
                pos: Vec2 { x: 100.0, y: 100.0 },
                size: Vec2 { x: 200.0, y: 200.0 },
            }).is_hovering;

        if is_hovering {
            self.count += 1;
        }

        gui.text(
            Vec2 { x: 100.0, y: 350.0 },
            format!(
                "Mouse pos: {} {}, is_hovering: {}",
                input.mouse_pos.x, input.mouse_pos.y, is_hovering
            ),
        );

        let m = input.mouse_pos;
        let hw = 32.0;
        gui.rect(Rect {
            pos: Vec2 {
                x: m.x - hw,
                y: m.y - hw,
            },
            size: Vec2 {
                x: 2.0 * hw,
                y: 2.0 * hw,
            },
        });

        gui.text(
            Vec2 { x: 100.0, y: 400.0 },
            format!("Count: {}", self.count),
        );
    }
}

#[wasm_bindgen]
pub fn show_gui(input_json: &str) -> String {
    lazy_static::lazy_static! {
        static ref APP: Mutex<App> = Mutex::new(App::new());
    }

    // TODO: faster interface than JSON
    let input: Input = serde_json::from_str(input_json).unwrap();

    let mut gui = Gui::new(input);
    APP.lock().unwrap().show_gui(&mut gui, &input);
    let commands = gui.into_commands();
    serde_json::to_string(&commands).unwrap()
}
