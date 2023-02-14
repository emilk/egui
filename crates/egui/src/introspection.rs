//! Showing UI:s for egui/epaint types.
use crate::*;

pub fn font_family_ui(ui: &mut Ui, font_family: &mut FontFamily) {
    let families = ui.fonts(|f| f.families());
    ui.horizontal(|ui| {
        for alternative in families {
            let text = alternative.to_string();
            ui.radio_value(font_family, alternative, text);
        }
    });
}

pub fn font_id_ui(ui: &mut Ui, font_id: &mut FontId) {
    let families = ui.fonts(|f| f.families());
    ui.horizontal(|ui| {
        ui.add(Slider::new(&mut font_id.size, 4.0..=40.0).max_decimals(1));
        for alternative in families {
            let text = alternative.to_string();
            ui.radio_value(&mut font_id.family, alternative, text);
        }
    });
}

// Show font texture in demo Ui
pub(crate) fn font_texture_ui(ui: &mut Ui, [width, height]: [usize; 2]) -> Response {
    ui.vertical(|ui| {
        let color = if ui.visuals().dark_mode {
            Color32::WHITE
        } else {
            Color32::BLACK
        };

        ui.label(format!(
            "Texture size: {} x {} (hover to zoom)",
            width, height
        ));
        if width <= 1 || height <= 1 {
            return;
        }
        let mut size = vec2(width as f32, height as f32);
        if size.x > ui.available_width() {
            size *= ui.available_width() / size.x;
        }
        let (rect, response) = ui.allocate_at_least(size, Sense::hover());
        let mut mesh = Mesh::default();
        mesh.add_rect_with_uv(rect, [pos2(0.0, 0.0), pos2(1.0, 1.0)].into(), color);
        ui.painter().add(Shape::mesh(mesh));

        let (tex_w, tex_h) = (width as f32, height as f32);

        response
            .on_hover_cursor(CursorIcon::ZoomIn)
            .on_hover_ui_at_pointer(|ui| {
                if let Some(pos) = ui.ctx().pointer_latest_pos() {
                    let (_id, zoom_rect) = ui.allocate_space(vec2(128.0, 128.0));
                    let u = remap_clamp(pos.x, rect.x_range(), 0.0..=tex_w);
                    let v = remap_clamp(pos.y, rect.y_range(), 0.0..=tex_h);

                    let texel_radius = 32.0;
                    let u = u.at_least(texel_radius).at_most(tex_w - texel_radius);
                    let v = v.at_least(texel_radius).at_most(tex_h - texel_radius);

                    let uv_rect = Rect::from_min_max(
                        pos2((u - texel_radius) / tex_w, (v - texel_radius) / tex_h),
                        pos2((u + texel_radius) / tex_w, (v + texel_radius) / tex_h),
                    );
                    let mut mesh = Mesh::default();
                    mesh.add_rect_with_uv(zoom_rect, uv_rect, color);
                    ui.painter().add(Shape::mesh(mesh));
                }
            });
    })
    .response
}

impl Widget for &epaint::stats::PaintStats {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            ui.label(
                "egui generates intermediate level shapes like circles and text. \
                These are later tessellated into triangles.",
            );
            ui.add_space(10.0);

            ui.style_mut().override_text_style = Some(TextStyle::Monospace);

            let epaint::stats::PaintStats {
                shapes,
                shape_text,
                shape_path,
                shape_mesh,
                shape_vec,
                num_callbacks,
                text_shape_vertices,
                text_shape_indices,
                clipped_primitives,
                vertices,
                indices,
            } = self;

            ui.label("Intermediate:");
            label(ui, shapes, "shapes").on_hover_text("Boxes, circles, etc");
            ui.horizontal(|ui| {
                label(ui, shape_text, "text");
                ui.small("(mostly cached)");
            });
            label(ui, shape_path, "paths");
            label(ui, shape_mesh, "nested meshes");
            label(ui, shape_vec, "nested shapes");
            ui.label(format!("{:6} callbacks", num_callbacks));
            ui.add_space(10.0);

            ui.label("Text shapes:");
            label(ui, text_shape_vertices, "vertices");
            label(ui, text_shape_indices, "indices")
                .on_hover_text("Three 32-bit indices per triangles");
            ui.add_space(10.0);

            ui.label("Tessellated (and culled):");
            label(ui, clipped_primitives, "primitives lists")
                .on_hover_text("Number of separate clip rectangles");
            label(ui, vertices, "vertices");
            label(ui, indices, "indices").on_hover_text("Three 32-bit indices per triangles");
            ui.add_space(10.0);

            // ui.label("Total:");
            // ui.label(self.total().format(""));
        })
        .response
    }
}

fn label(ui: &mut Ui, alloc_info: &epaint::stats::AllocInfo, what: &str) -> Response {
    ui.add(Label::new(alloc_info.format(what)).wrap(false))
}

impl Widget for &mut epaint::TessellationOptions {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            let epaint::TessellationOptions {
                feathering,
                feathering_size_in_pixels,
                coarse_tessellation_culling,
                prerasterized_discs,
                round_text_to_pixels,
                debug_paint_clip_rects,
                debug_paint_text_rects,
                debug_ignore_clip_rects,
                bezier_tolerance,
                epsilon: _,
            } = self;

            ui.checkbox(feathering, "Feathering (antialias)")
                .on_hover_text("Apply feathering to smooth out the edges of shapes. Turn off for small performance gain.");
            let feathering_slider = crate::Slider::new(feathering_size_in_pixels, 0.0..=10.0)
                .smallest_positive(0.1)
                .logarithmic(true)
                .text("Feathering size in pixels");
            ui.add_enabled(*feathering, feathering_slider);

            ui.checkbox(prerasterized_discs, "Speed up filled circles with pre-rasterization");

            ui.add(
                crate::widgets::Slider::new(bezier_tolerance, 0.0001..=10.0)
                    .logarithmic(true)
                    .show_value(true)
                    .text("Spline Tolerance"),
            );
            ui.collapsing("debug", |ui| {
                ui.checkbox(
                    coarse_tessellation_culling,
                    "Do coarse culling in the tessellator",
                );
                ui.checkbox(round_text_to_pixels, "Align text positions to pixel grid")
                    .on_hover_text("Most text already is, so don't expect to see a large change.");

                ui.checkbox(debug_ignore_clip_rects, "Ignore clip rectangles");
                ui.checkbox(debug_paint_clip_rects, "Paint clip rectangles");
                ui.checkbox(debug_paint_text_rects, "Paint text bounds");
            });
        })
        .response
    }
}

impl Widget for &memory::Interaction {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.vertical(|ui| {
            ui.label(format!("click_id: {:?}", self.click_id));
            ui.label(format!("drag_id: {:?}", self.drag_id));
            ui.label(format!("drag_is_window: {:?}", self.drag_is_window));
            ui.label(format!("click_interest: {:?}", self.click_interest));
            ui.label(format!("drag_interest: {:?}", self.drag_interest));
        })
        .response
    }
}
