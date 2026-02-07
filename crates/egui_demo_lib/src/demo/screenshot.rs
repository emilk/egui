use egui::{Image, UserData, ViewportCommand, Widget as _};
use std::sync::Arc;

/// Showcase [`ViewportCommand::Screenshot`].
#[derive(PartialEq, Eq, Default)]
pub struct Screenshot {
    image: Option<(Arc<egui::ColorImage>, egui::TextureHandle)>,
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
            ui.code("ViewportCommand::Screenshot");
            ui.label(".");
        });

        ui.horizontal_top(|ui| {
            let capture = ui.button("📷 Take Screenshot").clicked();
            ui.checkbox(&mut self.continuous, "Capture continuously");
            if capture || self.continuous {
                ui.send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
            }
        });

        let image = ui.input(|i| {
            i.events
                .iter()
                .filter_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(Arc::clone(image))
                    } else {
                        None
                    }
                })
                .next_back()
        });

        if let Some(image) = image {
            self.image = Some((
                Arc::clone(&image),
                ui.ctx()
                    .load_texture("screenshot_demo", image, Default::default()),
            ));
        }

        if let Some((_, texture)) = &self.image {
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
