#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<MyApp>::default())
        }),
    )
}

struct MyApp {
    name: String,
    age: u32,
    show_window: bool,
    show_popup: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
            show_window: false,
            show_popup: false,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut self.age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                self.age += 1;
            }
            ui.label(format!("Hello '{}' , age {}", self.name, self.age));

            ui.image(egui::include_image!(
                "../../../crates/egui/assets/ferris.png"
            ));

            ui.separator();

            // --- Demonstrate the new *_with builder helpers ----------------

            // 1. Area::show_with — always visible floating label
            egui::Area::new(egui::Id::new("builder_area")).show_with(ctx, |ui| {
                ui.label("Area built with show_with (drag me)");
            });

            // 2. Button toggles a popup built with Popup::show_with
            if ui.button("Toggle popup (builder)").clicked() {
                self.show_popup = !self.show_popup;
            }
            if self.show_popup {
                let anchor = ui.button("anchor");
                egui::Popup::from_response(&anchor)
                    .show_with(|ui| { ui.label("Hello from Popup::show_with"); });
            }

            // 3. A Window built with Window::show_with
            if ui.button("Toggle window (builder)").clicked() {
                self.show_window = !self.show_window;
            }
            if self.show_window {
                egui::Window::new("Builder window").show_with(ctx, |ui| {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, "This window was built with show_with ✅");

                    // 4. StripBuilder::new_with inside the window
                    egui_extras::StripBuilder::new_with(ui, |sb| {
                        sb.size(egui_extras::Size::remainder()).horizontal(|mut strip| {
                            strip.cell(|ui| { ui.label("Left cell"); });
                            strip.cell(|ui| { ui.label("Right cell"); });
                        })
                    });
                });
            }
        });
    }
}
