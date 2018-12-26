use crate::gui::Gui;

pub struct App {
    count: i32,
}

impl App {
    pub fn new() -> Self {
        App { count: 0 }
    }

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

        let commands_json = serde_json::to_string_pretty(&gui.paint_commands()).unwrap();
        gui.label(format!("All paint commands:\n{}", commands_json));
    }
}
