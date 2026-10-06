use egui::{Image, Widget as _, mutex::Mutex};
use std::sync::Arc;

/// Showcase [`egui::Context::request_screenshot`].
#[derive(Default)]
pub struct Screenshot {
    /// Where the screenshot callback puts the captured image.
    received: Arc<Mutex<Option<Arc<egui::ColorImage>>>>,

    texture: Option<egui::TextureHandle>,
    continuous: bool,
}

impl crate::Demo for Screenshot {
    fn name(&self) -> &'static str {
        "📷 Screenshot"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .default_width(250.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for Screenshot {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.set_width(300.0);
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("This demo showcases how to take screenshots via ");
            ui.code("Context::request_screenshot");
            ui.label(".");
        });

        ui.horizontal_top(|ui| {
            let capture = ui.button("📷 Take Screenshot").clicked();
            ui.checkbox(&mut self.continuous, "Capture continuously");
            if capture || self.continuous {
                let received = Arc::clone(&self.received);
                let ctx = ui.ctx().clone();
                ui.ctx().request_screenshot(move |image| {
                    *received.lock() = Some(image);
                    // The callback doesn't repaint on its own, so ask for a frame to show it in.
                    ctx.request_repaint();
                });
            }
        });

        if let Some(image) = self.received.lock().take() {
            self.texture = Some(ui.ctx().load_texture(
                "screenshot_demo",
                image,
                Default::default(),
            ));
        }

        if let Some(texture) = &self.texture {
            Image::new(texture).shrink_to_fit().ui(ui);
        } else {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.set_height(100.0);
                ui.centered_and_justified(|ui| {
                    ui.label("No screenshot taken yet.");
                });
            });
        }
    }
}
