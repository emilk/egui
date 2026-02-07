#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::{
    self, Color32, Stroke, Style, Theme, global_theme_preference_buttons, style::Selection,
};
use egui_demo_lib::{View as _, WidgetGallery};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([350.0, 590.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui example: custom style",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}

fn setup_custom_style(ctx: &egui::Context) {
    ctx.style_mut_of(Theme::Light, use_light_green_accent);
    ctx.style_mut_of(Theme::Dark, use_dark_purple_accent);
}

fn use_light_green_accent(style: &mut Style) {
    style.visuals.hyperlink_color = Color32::from_rgb(18, 180, 85);
    style.visuals.text_cursor.stroke.color = Color32::from_rgb(28, 92, 48);
    style.visuals.selection = Selection {
        bg_fill: Color32::from_rgb(157, 218, 169),
        stroke: Stroke::new(1.0, Color32::from_rgb(28, 92, 48)),
    };
}

fn use_dark_purple_accent(style: &mut Style) {
    style.visuals.hyperlink_color = Color32::from_rgb(202, 135, 227);
    style.visuals.text_cursor.stroke.color = Color32::from_rgb(234, 208, 244);
    style.visuals.selection = Selection {
        bg_fill: Color32::from_rgb(105, 67, 119),
        stroke: Stroke::new(1.0, Color32::from_rgb(234, 208, 244)),
    };
}

struct MyApp {
    widget_gallery: WidgetGallery,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_style(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx); // Needed for the "Widget Gallery" demo
        Self {
            widget_gallery: WidgetGallery::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("egui using a customized style");
            ui.label("Switch between dark and light mode to see the different styles in action.");
            global_theme_preference_buttons(ui);
            ui.separator();
            self.widget_gallery.ui(ui);
        });
    }
}
