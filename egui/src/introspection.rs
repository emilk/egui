//! uis for egui types.
use crate::{
    containers::show_tooltip,
    math::*,
    paint::{self, color::WHITE, PaintCmd, Texture, Triangles, Vertex},
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
        let top_left = Vertex {
            pos: rect.min,
            uv: pos2(0.0, 0.0),
            color: WHITE,
        };
        let bottom_right = Vertex {
            pos: rect.max,
            uv: pos2(1.0, 1.0),
            color: WHITE,
        };
        let mut triangles = Triangles::default();
        triangles.add_rect(top_left, bottom_right);
        ui.painter().add(PaintCmd::Triangles(triangles));

        let tex_w = self.width as f32;
        let tex_h = self.height as f32;

        if ui.hovered(rect) {
            show_tooltip(ui.ctx(), |ui| {
                let pos = ui.input().mouse.pos.unwrap_or_else(|| ui.top_left());
                let zoom_rect = ui.allocate_space(vec2(128.0, 128.0));
                let u = remap_clamp(pos.x, rect.range_x(), 0.0..=tex_w);
                let v = remap_clamp(pos.y, rect.range_y(), 0.0..=tex_h);

                let texel_radius = 32.0;
                let u = u.max(texel_radius).min(tex_w - texel_radius);
                let v = v.max(texel_radius).min(tex_h - texel_radius);

                let top_left = Vertex {
                    pos: zoom_rect.min,
                    uv: pos2((u - texel_radius) / tex_w, (v - texel_radius) / tex_h),
                    color: WHITE,
                };
                let bottom_right = Vertex {
                    pos: zoom_rect.max,
                    uv: pos2((u + texel_radius) / tex_w, (v + texel_radius) / tex_h),
                    color: WHITE,
                };
                let mut triangles = Triangles::default();
                triangles.add_rect(top_left, bottom_right);
                ui.painter().add(PaintCmd::Triangles(triangles));
            });
        }
    }
}

impl paint::FontDefinitions {
    pub fn ui(&mut self, ui: &mut Ui) {
        for (text_style, (_family, size)) in self.fonts.iter_mut() {
            // TODO: radiobutton for family
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
