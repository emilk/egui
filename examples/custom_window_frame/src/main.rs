//! Show a custom window frame instead of the default OS window chrome decorations.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        // Hide the OS-specific "chrome" around the window:
        decorated: false,
        // To have rounded corners we need transparency:
        transparent: true,
        min_window_size: Some(egui::vec2(400.0, 100.0)),
        initial_window_size: Some(egui::vec2(400.0, 240.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Custom window frame", // unused title
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    )
}

#[derive(Default)]
struct MyApp {}

impl eframe::App for MyApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array() // Make sure we don't paint anything behind the rounded corners
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        custom_window_frame(ctx, frame, "egui with custom frame", |ui| {
            ui.label("This is just the contents of the window");
            ui.horizontal(|ui| {
                ui.label("egui theme:");
                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });
    }
}

fn custom_window_frame(
    ctx: &egui::Context,
    frame: &mut eframe::Frame,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    use egui::*;
    let text_color = ctx.style().visuals.text_color();

    // Height of the title bar
    let height = 28.0;

    let button_height = 16.0;

    CentralPanel::default()
        .frame(Frame::none())
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            let painter = ui.painter();

            // Paint the frame:
            painter.rect(
                rect.shrink(1.0),
                10.0,
                ctx.style().visuals.window_fill(),
                Stroke::new(1.0, text_color),
            );

            // Paint the title:
            painter.text(
                rect.center_top() + vec2(0.0, height / 2.0),
                Align2::CENTER_CENTER,
                title,
                FontId::proportional(height * 0.8),
                text_color,
            );

            // Paint the line under the title:
            painter.line_segment(
                [
                    rect.left_top() + vec2(2.0, height),
                    rect.right_top() + vec2(-2.0, height),
                ],
                Stroke::new(1.0, text_color),
            );

            // Interact with the title bar (drag to move window):
            let title_bar_rect = {
                let mut rect = rect;
                rect.max.y = rect.min.y + height;
                rect
            };
            let title_bar_response =
                ui.interact(title_bar_rect, Id::new("title_bar"), Sense::click());

            if title_bar_response.double_clicked() {
                frame.set_maximized(!frame.info().window_info.maximized);
            } else if title_bar_response.is_pointer_button_down_on() {
                frame.drag_window();
            }

            ui.allocate_ui_at_rect(title_bar_rect, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.visuals_mut().button_frame = false;

                    let close_response = ui
                        .add(Button::new(RichText::new("❌").size(button_height)))
                        .on_hover_text("Close the window");
                    if close_response.clicked() {
                        frame.close();
                    }

                    let minimized_response = ui
                        .add(Button::new(RichText::new("🗕").size(button_height)))
                        .on_hover_text("Minimize the window");
                    if minimized_response.clicked() {
                        frame.set_minimized(true);
                    }

                    if frame.info().window_info.maximized {
                        let maximized_response = ui
                            .add(Button::new(RichText::new("🗖").size(button_height)))
                            .on_hover_text("Restore window");
                        if maximized_response.clicked() {
                            frame.set_maximized(false);
                        }
                    } else {
                        let maximized_response = ui
                            .add(Button::new(RichText::new("🗗").size(button_height)))
                            .on_hover_text("Maximize window");
                        if maximized_response.clicked() {
                            frame.set_maximized(true);
                        }
                    }
                });
            });

            // Add the contents:
            let content_rect = {
                let mut rect = rect;
                rect.min.y = title_bar_rect.max.y;
                rect
            }
            .shrink(4.0);
            let mut content_ui = ui.child_ui(content_rect, *ui.layout());
            add_contents(&mut content_ui);
        });
}
