#![deny(warnings)]
#![warn(clippy::all)]

use egui_glium::{persistence::Persistence, Runner};

const APP_KEY: &str = "app";

#[derive(Default, serde::Deserialize, serde::Serialize)]
struct MyApp {
    egui_example_app: egui::ExampleApp,
}

impl egui_glium::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, runner: &mut Runner) {
        self.egui_example_app.ui(ui, "");

        use egui::*;
        let mut ui = ui.centered_column(ui.available().width().min(480.0));
        ui.set_layout(Layout::vertical(Align::Min));
        ui.add(label!("Egui inside of Glium").text_style(TextStyle::Heading));
        if ui.add(Button::new("Quit")).clicked {
            runner.quit();
            return;
        }

        ui.add(
            label!(
                "CPU usage: {:.2} ms (excludes painting)",
                1e3 * runner.cpu_usage()
            )
            .text_style(TextStyle::Monospace),
        );
        ui.add(label!("FPS: {:.1}", runner.fps()).text_style(TextStyle::Monospace));
    }

    fn on_exit(&mut self, persistence: &mut Persistence) {
        persistence.set_value(APP_KEY, self);
    }
}

fn main() {
    let title = "Egui glium example";
    let persistence = Persistence::from_path(".egui_example_glium.json".into());
    let app: MyApp = persistence.get_value(APP_KEY).unwrap_or_default();
    egui_glium::run(title, persistence, app);
}
