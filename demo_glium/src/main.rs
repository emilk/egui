#![deny(warnings)]
#![warn(clippy::all)]

use egui_glium::{persistence::Persistence, RunMode, Runner};

const APP_KEY: &str = "app";

#[derive(Default, serde::Deserialize, serde::Serialize)]
struct MyApp {
    egui_demo_app: egui::DemoApp,
    frames_painted: u64,
}

impl egui_glium::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, runner: &mut Runner) {
        self.egui_demo_app.ui(ui, "");

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
                "CPU usage: {:.2} ms / frame (excludes painting)",
                1e3 * runner.cpu_time()
            )
            .text_style(TextStyle::Monospace),
        );

        ui.separator();

        ui.horizontal(|ui| {
            let mut run_mode = runner.run_mode();
            ui.label("Run mode:");
            ui.radio_value("Continuous", &mut run_mode, RunMode::Continuous)
                .tooltip_text("Repaint everything each frame");
            ui.radio_value("Reactive", &mut run_mode, RunMode::Reactive)
                .tooltip_text("Repaint when there are animations or input (e.g. mouse movement)");
            runner.set_run_mode(run_mode);
        });

        if runner.run_mode() == RunMode::Continuous {
            ui.add(
                label!("Repainting the UI each frame. FPS: {:.1}", runner.fps())
                    .text_style(TextStyle::Monospace),
            );
        } else {
            ui.label("Only running UI code when there are animations or input");
        }

        self.frames_painted += 1;
        ui.label(format!("Total frames painted: {}", self.frames_painted));
    }

    fn on_exit(&mut self, persistence: &mut Persistence) {
        persistence.set_value(APP_KEY, self);
    }
}

fn main() {
    let title = "Egui glium demo";
    let persistence = Persistence::from_path(".egui_demo_glium.json".into());
    let app: MyApp = persistence.get_value(APP_KEY).unwrap_or_default();
    egui_glium::run(title, RunMode::Reactive, persistence, app);
}
