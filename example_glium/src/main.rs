use egui_glium::{persistence::Persistence, RunMode, Runner};

const APP_KEY: &str = "app";

/// We dervive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
struct MyApp {
    counter: u64,
}

impl egui_glium::App for MyApp {
    /// This function will be called whenever the Ui needs to be shown,
    /// which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut Runner) {
        if ui.button("Increment").clicked {
            self.counter += 1;
        }
        if ui.button("Reset").clicked {
            self.counter = 0;
        }
        ui.label(format!("Counter: {}", self.counter));
    }

    fn on_exit(&mut self, persistence: &mut Persistence) {
        persistence.set_value(APP_KEY, self); // Save app state
    }
}

fn main() {
    let title = "My Egui Window";
    let persistence = Persistence::from_path(".egui_example_glium.json".into()); // Where to persist app state
    let app: MyApp = persistence.get_value(APP_KEY).unwrap_or_default(); // Restore `MyApp` from file, or create new `MyApp`.
    egui_glium::run(title, RunMode::Reactive, persistence, app);
}
