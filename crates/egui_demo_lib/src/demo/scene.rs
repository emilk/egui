use egui::emath::TSTransform;
use egui::scene::{fit_to_rect_in_scene, Scene};
use egui::{Pos2, Rect, Sense, UiBuilder, Vec2};

use super::widget_gallery;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SceneDemo {
    parent_from_child: TSTransform,
    widget_gallery: widget_gallery::WidgetGallery,
}

impl Default for SceneDemo {
    fn default() -> Self {
        Self {
            parent_from_child: TSTransform::from_scaling(0.5),
            widget_gallery: Default::default(),
        }
    }
}

impl crate::Demo for SceneDemo {
    fn name(&self) -> &'static str {
        "🔍 Scene"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        use crate::View as _;
        let window = egui::Window::new("Scene")
            .default_width(300.0)
            .default_height(300.0)
            .scroll(false)
            .open(open);
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl crate::View for SceneDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "Pan, zoom in, and zoom out with scrolling (see the plot demo for more instructions). \
                   Double click on the background to reset.",
        );
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
        ui.separator();

        ui.monospace(format!("{:#?}", &mut self.parent_from_child));

        ui.separator();

        egui::Frame::group(ui.style())
            .inner_margin(0.0)
            .show(ui, |ui| {
                let scene = Scene::new().max_inner_size([500.0, 1000.0]);

                let mut reset_view = false;
                let response = scene.show(ui, &mut self.parent_from_child, |ui| {
                    self.widget_gallery.ui(ui);

                    // ui.put(
                    //     Rect::from_min_size(Pos2::ZERO, Vec2::splat(64.0)),
                    //     egui::Button::new("Button"),
                    // );
                    // ui.put(
                    //     Rect::from_min_size(Pos2::ZERO + 72.0 * Vec2::DOWN, Vec2::new(500.0, 64.0)),
                    //     egui::Label::new(format!("Inner ui min_rect: {:?}", ui.min_rect())),
                    // );

                    reset_view = ui.button("Reset view").clicked();
                });

                if reset_view || response.double_clicked() {
                    self.parent_from_child = TSTransform::from_scaling(0.5);
                }
            });
    }
}
