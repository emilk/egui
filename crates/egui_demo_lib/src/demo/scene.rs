use egui::{Pos2, Rect, Scene, Vec2};

use super::widget_gallery;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SceneDemo {
    widget_gallery: widget_gallery::WidgetGallery,
    scene_rect: Rect,
}

impl Default for SceneDemo {
    fn default() -> Self {
        Self {
            widget_gallery: widget_gallery::WidgetGallery::default().with_date_button(false), // disable date button so that we don't fail the snapshot test
            scene_rect: Rect::ZERO, // `egui::Scene` will initialize this to something valid
        }
    }
}

impl crate::Demo for SceneDemo {
    fn name(&self) -> &'static str {
        "🔍 Scene"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        use crate::View as _;
        egui::Window::new("Scene")
            .default_width(300.0)
            .default_height(300.0)
            .scroll(false)
            .open(open)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| self.ui(ui));
    }
}

impl crate::View for SceneDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "You can pan by scrolling, and zoom using cmd-scroll. \
            Double click on the background to reset view.",
        );
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
        ui.separator();

        ui.label(format!("Scene rect: {:#?}", &mut self.scene_rect));

        ui.separator();

        egui::Frame::group(ui.style())
            .inner_margin(0.0)
            .show(ui, |ui| {
                let scene = Scene::new()
                    .max_inner_size([350.0, 1000.0])
                    .zoom_range(0.1..=2.0);

                let mut reset_view = false;
                let mut inner_rect = Rect::NAN;
                let response = scene
                    .show(ui, &mut self.scene_rect, |ui| {
                        reset_view = ui.button("Reset view").clicked();

                        ui.add_space(16.0);

                        self.widget_gallery.ui(ui);

                        ui.put(
                            Rect::from_min_size(Pos2::new(0.0, -64.0), Vec2::new(200.0, 16.0)),
                            egui::Label::new("You can put a widget anywhere").selectable(false),
                        );

                        inner_rect = ui.min_rect();
                    })
                    .response;

                if reset_view || response.double_clicked() {
                    self.scene_rect = inner_rect;
                }
            });
    }
}
