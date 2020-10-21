//! Example of how to use Egui

#![deny(warnings)]
#![warn(clippy::all)]

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
struct MyApp {
    my_string: String,
    value: f32,
}

impl egui::app::App for MyApp {
    /// This function will be called whenever the Ui needs to be shown,
    /// which may be many times per second.
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _info: &egui::app::BackendInfo,
        _tex_allocator: Option<&mut dyn egui::app::TextureAllocator>,
    ) -> egui::app::AppOutput {
        let MyApp { my_string, value } = self;

        // Example used in `README.md`.
        egui::Window::new("Debug").show(ui.ctx(), |ui| {
            ui.label(format!("Hello, world {}", 123));
            if ui.button("Save").clicked {
                my_save_function();
            }
            ui.text_edit(my_string);
            ui.add(egui::Slider::f32(value, 0.0..=1.0).text("float"));
        });

        Default::default()
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

fn my_save_function() {
    // dummy
}
