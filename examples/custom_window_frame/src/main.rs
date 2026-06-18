//! Show a custom window frame instead of the default OS window chrome decorations.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui::{self, ViewportCommand};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false) // Hide the OS-specific "chrome" around the window
            .with_inner_size([400.0, 100.0])
            .with_min_inner_size([400.0, 100.0])
            .with_transparent(true), // To have rounded corners we need transparency

        ..Default::default()
    };
    eframe::run_native(
        "Custom window frame", // unused title
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

#[derive(Default)]
struct MyApp {}

impl eframe::App for MyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        custom_window_frame(ui, "egui with custom frame", |ui| {
            ui.label("This is just the contents of the window.");
            ui.horizontal(|ui| {
                ui.label("egui theme:");
                egui::widgets::global_theme_preference_buttons(ui);
            });
        });
    }
}

fn custom_window_frame(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    use egui::UiBuilder;

    let panel_frame = egui::Frame::new()
        .fill(ui.global_style().visuals.window_fill())
        .corner_radius(10)
        .stroke(ui.global_style().visuals.widgets.noninteractive.fg_stroke)
        .outer_margin(1); // so the stroke is within the bounds

    panel_frame.show(ui, |ui| {
        let app_rect = ui.max_rect();

        ui.expand_to_include_rect(app_rect); // Expand frame to include it all

        let title_bar_height = 32.0;
        let title_bar_rect = {
            let mut rect = app_rect;
            rect.max.y = rect.min.y + title_bar_height;
            rect
        };
        title_bar_ui(ui, title_bar_rect, title);

        // Add the contents:
        let content_rect = {
            let mut rect = app_rect;
            rect.min.y = title_bar_rect.max.y;
            rect
        }
        .shrink(4.0);
        let mut content_ui = ui.new_child(UiBuilder::new().max_rect(content_rect));
        add_contents(&mut content_ui);
    });
}

fn title_bar_ui(ui: &mut egui::Ui, title_bar_rect: eframe::epaint::Rect, title: &str) {
    use egui::{Align2, FontId, Id, PointerButton, Sense, UiBuilder, vec2};

    let painter = ui.painter();

    let title_bar_response = ui.interact(
        title_bar_rect,
        Id::new("title_bar"),
        Sense::click_and_drag(),
    );

    // Paint the title:
    painter.text(
        title_bar_rect.center(),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(20.0),
        ui.style().visuals.text_color(),
    );

    // Paint the line under the title:
    painter.line_segment(
        [
            title_bar_rect.left_bottom() + vec2(1.0, 0.0),
            title_bar_rect.right_bottom() + vec2(-1.0, 0.0),
        ],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );

    // Interact with the title bar (drag to move window):
    if title_bar_response.double_clicked() {
        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    }

    if title_bar_response.drag_started_by(PointerButton::Primary) {
        ui.send_viewport_cmd(ViewportCommand::StartDrag);
    }

    ui.scope_builder(
        UiBuilder::new()
            .max_rect(title_bar_rect)
            .layout(egui::Layout::right_to_left(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.visuals_mut().button_frame = false;
            ui.add_space(8.0);
            close_maximize_minimize(ui);
        },
    );
}

/// Show some close/maximize/minimize buttons for the native window.
fn close_maximize_minimize(ui: &mut egui::Ui) {
    use egui::{Button, RichText};

    let button_height = 12.0;

    let close_response = ui
        .add(Button::new(RichText::new("❌").size(button_height)))
        .on_hover_text("Close the window");
    if close_response.clicked() {
        ui.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    if is_maximized {
        let maximized_response = ui
            .add(Button::new(RichText::new("🗗").size(button_height)))
            .on_hover_text("Restore window");
        if maximized_response.clicked() {
            ui.send_viewport_cmd(ViewportCommand::Maximized(false));
        }
    } else {
        let maximized_response = ui
            .add(Button::new(RichText::new("🗗").size(button_height)))
            .on_hover_text("Maximize window");
        if maximized_response.clicked() {
            ui.send_viewport_cmd(ViewportCommand::Maximized(true));
        }
    }

    let minimized_response = ui
        .add(Button::new(RichText::new("🗕").size(button_height)))
        .on_hover_text("Minimize the window");
    if minimized_response.clicked() {
        ui.send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}
