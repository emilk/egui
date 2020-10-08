//! uis for egui types.
use crate::{
    containers::show_tooltip,
    math::*,
    paint::{self, color::WHITE, PaintCmd, Texture, Triangles},
    *,
};

impl Texture {
    pub fn ui(&self, ui: &mut Ui) {
        ui.add(label!(
            "Texture size: {} x {} (hover to zoom)",
            self.width,
            self.height
        ));
        if self.width <= 1 || self.height <= 1 {
            return;
        }
        let mut size = vec2(self.width as f32, self.height as f32);
        if size.x > ui.available().width() {
            size *= ui.available().width() / size.x;
        }
        let rect = ui.allocate_space(size);
        let mut triangles = Triangles::default();
        triangles.add_rect_with_uv(rect, [pos2(0.0, 0.0), pos2(1.0, 1.0)].into(), WHITE);
        ui.painter().add(PaintCmd::Triangles(triangles));

        let tex_w = self.width as f32;
        let tex_h = self.height as f32;

        if ui.hovered(rect) {
            show_tooltip(ui.ctx(), |ui| {
                let pos = ui
                    .input()
                    .mouse
                    .pos
                    .unwrap_or_else(|| ui.min_rect().left_top());
                let zoom_rect = ui.allocate_space(vec2(128.0, 128.0));
                let u = remap_clamp(pos.x, rect.x_range(), 0.0..=tex_w);
                let v = remap_clamp(pos.y, rect.y_range(), 0.0..=tex_h);

                let texel_radius = 32.0;
                let u = u.at_least(texel_radius).at_most(tex_w - texel_radius);
                let v = v.at_least(texel_radius).at_most(tex_h - texel_radius);

                let uv_rect = Rect::from_min_max(
                    pos2((u - texel_radius) / tex_w, (v - texel_radius) / tex_h),
                    pos2((u + texel_radius) / tex_w, (v + texel_radius) / tex_h),
                );
                let mut triangles = Triangles::default();
                triangles.add_rect_with_uv(zoom_rect, uv_rect, WHITE);
                ui.painter().add(PaintCmd::Triangles(triangles));
            });
        }
    }
}

impl paint::FontDefinitions {
    pub fn ui(&mut self, ui: &mut Ui) {
        for (text_style, (_family, size)) in self.fonts.iter_mut() {
            // TODO: radio button for family
            ui.add(
                Slider::f32(size, 4.0..=40.0)
                    .precision(0)
                    .text(format!("{:?}", text_style)),
            );
        }
        if ui.add(Button::new("Reset fonts")).clicked {
            *self = paint::FontDefinitions::with_pixels_per_point(self.pixels_per_point);
        }
    }
}
