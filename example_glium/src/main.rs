//! Example of how to use Egui

#![deny(warnings)]
#![warn(clippy::all)]

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
struct MyApp {
    name: String,
    age: u32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
        }
    }
}

impl egui::app::App for MyApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn ui(
        &mut self,
        ctx: &std::sync::Arc<egui::Context>,
        integration_context: &mut egui::app::IntegrationContext,
    ) {
        let MyApp { name, age } = self;

        // Example used in `README.md`.
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My Egui Application");

            ui.horizontal(|ui| {
                ui.label("Your name: ");
                ui.text_edit(name);
            });

            ui.add(egui::Slider::u32(age, 0..=120).text("age"));
            if ui.button("Click each year").clicked {
                *age += 1;
            }

            ui.label(format!("Hello '{}', age {}", name, age));

            ui.advance_cursor(16.0);
            if ui.button("Quit").clicked {
                integration_context.output.quit = true;
            }
        });

        integration_context.output.window_size = Some(ctx.used_size()); // resize the window to be just the size we need it to be
    }

    fn on_exit(&mut self, storage: &mut dyn egui::app::Storage) {
        egui::app::set_value(storage, egui::app::APP_KEY, self);
    }
}

fn main() {
    let title = "My Egui Window";

    // Persist app state to file:
    let storage = egui_glium::storage::FileStorage::from_path(".egui_example_glium.json".into());

    // Alternative: store nowhere
    // let storage = egui::app::DummyStorage::default();

    let app: MyApp = egui::app::get_value(&storage, egui::app::APP_KEY).unwrap_or_default(); // Restore `MyApp` from file, or create new `MyApp`.
    egui_glium::run(title, Box::new(storage), app);
}
