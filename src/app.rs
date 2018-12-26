use crate::gui::Gui;

#[derive(Default)]
pub struct App {
    count: i32,
}

impl App {
    pub fn show_gui(&mut self, gui: &mut Gui) {
        if gui.button("Click me").clicked {
            self.count += 1;
        }
        if gui.button("Or click me instead!").clicked {
            self.count += 1;
        }

        gui.label(format!(
            "The buttons have been clicked {} times",
            self.count
        ));

        let commands_json = format!("{:#?}", gui.gui_commands());
        gui.label(format!("All gui commands: {}", commands_json));
    }
}
