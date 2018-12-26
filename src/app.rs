use crate::{gui::Gui, math::*, types::*};

pub struct App {
    count: i32,

    width: f32,
    height: f32,
    corner_radius: f32,
    stroke_width: f32,
}

impl App {
    pub fn new() -> App {
        App {
            count: 0,
            width: 100.0,
            height: 50.0,
            corner_radius: 5.0,
            stroke_width: 2.0,
        }
    }

    pub fn show_gui(&mut self, gui: &mut Gui) {
        if gui.button("Click me").clicked {
            self.count += 1;
        }

        gui.label(format!("The button have been clicked {} times", self.count));

        gui.slider_f32("width", &mut self.width, 0.0, 100.0);
        gui.slider_f32("height", &mut self.height, 0.0, 100.0);
        gui.slider_f32("corner_radius", &mut self.corner_radius, 0.0, 100.0);

        gui.commands
            .push(GuiCmd::PaintCommands(vec![PaintCmd::Rect {
                corner_radius: self.corner_radius,
                fill_style: Some("#888888ff".into()),
                pos: vec2(300.0, 100.0),
                size: vec2(self.width, self.height),
                outline: Some(Outline {
                    width: self.stroke_width,
                    style: "#ffffffff".into(),
                }),
            }]));

        let commands_json = format!("{:#?}", gui.gui_commands());
        gui.label(format!("All gui commands: {}", commands_json));
    }
}
