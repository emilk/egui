#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use std::path::PathBuf;

use eframe::{
    Storage, StorageProviderBuild,
    egui::{self, ahash::HashMap},
};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([350.0, 590.0]),
        storage_build: StorageProviderBuild::Custom(custom_storage),
        ..Default::default()
    };
    eframe::run_native(
        "egui example: custom style",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct MyApp {
    pub custom_data: String,
    pub custom_data2: String,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            custom_data: "Hello".to_string(),
            custom_data2: "World".to_string(),
        }
    }
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx); // Needed for the "Widget Gallery" demo
        cc.storage
            .and_then(|storage| eframe::get_value(storage, "app"))
            .unwrap_or_default()
    }
}

impl eframe::App for MyApp {
    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::set_value(storage, "app", &self);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("egui using a custom storage for app");
            ui.label("Change data and restart the app to see it.");
            ui.separator();
            ui.text_edit_singleline(&mut self.custom_data);
            ui.text_edit_singleline(&mut self.custom_data2);
        });
    }
}

fn custom_storage(_app_name: &str) -> Option<Box<dyn Storage>> {
    CustomStorageData::new(
        std::env::current_dir()
            .unwrap_or_default()
            .join("custom_storage.json"),
    )
    .map(|data| Box::new(data) as Box<dyn Storage>)
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomStorageData {
    hashmap: HashMap<String, String>,
    path: PathBuf,
}

impl CustomStorageData {
    pub fn new(path: PathBuf) -> Option<Self> {
        let hashmap: HashMap<String, String> = std::fs::read(&path)
            .ok()
            .and_then(|contents| serde_json::from_slice(contents.as_slice()).ok())
            .unwrap_or_default();

        Some(Self { hashmap, path })
    }
}

impl Storage for CustomStorageData {
    fn get_string(&self, key: &str) -> Option<String> {
        self.hashmap.get(key).cloned()
    }

    fn set_string(&mut self, key: &str, value: String) {
        self.hashmap.insert(key.to_string(), value);
    }

    fn flush(&mut self) {
        let Ok(content) = serde_json::to_string_pretty(&self.hashmap) else {
            return;
        };
        _ = std::fs::write(&self.path, content);
    }
}
