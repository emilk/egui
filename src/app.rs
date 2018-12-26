use crate::gui::Gui;

#[derive(Default)]
pub struct App {
    count: i32,
    slider_value: f32,
}

impl App {
    pub fn show_gui(&mut self, gui: &mut Gui) {
        if gui.button("Click me").clicked {
            self.count += 1;
        }

        gui.label(format!("The button have been clicked {} times", self.count));

        gui.slider_f32("Slider", &mut self.slider_value, 0.0, 10.0);

        let commands_json = format!("{:#?}", gui.gui_commands());
        gui.label(format!("All gui commands: {}", commands_json));
    }
}
