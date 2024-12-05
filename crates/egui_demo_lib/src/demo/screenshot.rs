use egui::{
    Frame, Image, ImageSource, Label, RichText, Sense, UiBuilder, UserData, ViewportCommand, Widget,
};
use std::sync::Arc;

/// Showcase [`egui::Ui::response`].
#[derive(PartialEq, Eq, Default)]
pub struct Screenshot {
    image: Option<(Arc<egui::ColorImage>, egui::TextureHandle)>,
}

impl crate::Demo for Screenshot {
    fn name(&self) -> &'static str {
        "📸 Screenshot"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .default_width(250.0)
            .show(ctx, |ui| {
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
            if ui.button("📸 Take Screenshot").clicked() {
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
            }
        });

        let image = ui.ctx().input(|i| {
            i.events.iter().find_map(|e| {
                let egui::Event::Screenshot { image, .. } = e else {
                    return None;
                };
                Some(image.clone())
            })
        });

        if let Some(image) = image {
            self.image = Some((
                image.clone(),
                ui.ctx()
                    .load_texture("screenshot_demo", image, Default::default()),
            ));
        }

        if let Some((_, texture)) = &self.image {
            Image::new(texture).shrink_to_fit().ui(ui);
        } else {
        }
    }
}
