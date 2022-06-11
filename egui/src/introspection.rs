//! Showing UI:s for egui/epaint types.
use crate::*;

pub fn font_family_ui(ui: &mut Ui<'_>, font_family: &mut FontFamily) {
    let families = ui.fonts().families();
    ui.horizontal(|ui| {
        for alternative in families {
            let text = alternative.to_string();
            ui.radio_value(font_family, alternative, text);
        }
    });
}

pub fn font_id_ui(ui: &mut Ui<'_>, font_id: &mut FontId) {
    let families = ui.fonts().families();
    ui.horizontal(|ui| {
        ui.add(Slider::new(&mut font_id.size, 4.0..=40.0).max_decimals(0));
        for alternative in families {
            let text = alternative.to_string();
            ui.radio_value(&mut font_id.family, alternative, text);
        }
    });
}

// Show font texture in demo Ui
pub(crate) fn font_texture_ui(ui: &mut Ui<'_>, [width, height]: [usize; 2]) -> Response {
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
        ui.painter.add(ui.ctx, Shape::mesh(mesh));

        let (tex_w, tex_h) = (width as f32, height as f32);

        response
            .on_hover_cursor(ui.ctx, CursorIcon::ZoomIn)
            .on_hover_ui_at_pointer(ui.ctx, |ui| {
                if let Some(pos) = ui.ctx.pointer_latest_pos() {
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
                    ui.painter.add(ui.ctx, Shape::mesh(mesh));
                }
            });
    })
    .response
}

impl Widget for &epaint::stats::PaintStats {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
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
            label(ui, shapes, "shapes").on_hover_text(ui.ctx, "Boxes, circles, etc");
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
                .on_hover_text(ui.ctx, "Three 32-bit indices per triangles");
            ui.add_space(10.0);

            ui.label("Tessellated (and culled):");
            label(ui, clipped_primitives, "primitives lists")
                .on_hover_text(ui.ctx, "Number of separate clip rectangles");
            label(ui, vertices, "vertices");
            label(ui, indices, "indices")
                .on_hover_text(ui.ctx, "Three 32-bit indices per triangles");
            ui.add_space(10.0);

            // ui.label("Total:");
            // ui.label(self.total().format(""));
        })
        .response
    }
}

fn label<'c>(ui: &mut Ui<'c>, alloc_info: &epaint::stats::AllocInfo, what: &str) -> Response {
    ui.add(Label::new(alloc_info.format(what)).wrap(false))
}

impl Widget for &mut epaint::TessellationOptions {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
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
                .on_hover_text(ui.ctx, "Apply feathering to smooth out the edges of shapes. Turn off for small performance gain.");
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
                    .on_hover_text(ui.ctx, "Most text already is, so don't expect to see a large change.");

                ui.checkbox(debug_ignore_clip_rects, "Ignore clip rectangles");
                ui.checkbox(debug_paint_clip_rects, "Paint clip rectangles");
                ui.checkbox(debug_paint_text_rects, "Paint text bounds");
            });
        })
        .response
    }
}

impl Widget for &memory::Interaction {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
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

pub fn settings_ui(ui: &mut Ui<'_>) {
    use crate::containers::*;

    CollapsingHeader::new("🎑 Style")
        .default_open(true)
        .show(ui, |ui| {
            style_ui(ui);
        });

    CollapsingHeader::new("✒ Painting")
        .default_open(true)
        .show(ui, |ui| {
            let mut tessellation_options = *ui.ctx.tessellation_options();
            tessellation_options.ui(ui);
            ui.vertical_centered(|ui| reset_button(ui, &mut tessellation_options));
            *ui.ctx.tessellation_options_mut() = tessellation_options;
        });
}

pub fn inspection_ui(ui: &mut Ui<'_>) {
    use crate::containers::*;
    crate::trace!(ui);

    ui.label(format!("Is using pointer: {}", ui.ctx.is_using_pointer()))
        .on_hover_text(
            ui.ctx,
            "Is egui currently using the pointer actively (e.g. dragging a slider)?",
        );
    ui.label(format!("Wants pointer input: {}", ui.ctx.wants_pointer_input()))
        .on_hover_text(ui.ctx, "Is egui currently interested in the location of the pointer (either because it is in use, or because it is hovering over a window).");
    ui.label(format!(
        "Wants keyboard input: {}",
        ui.ctx.wants_keyboard_input()
    ))
    .on_hover_text(ui.ctx, "Is egui currently listening for text input?");
    ui.label(format!(
        "Keyboard focus widget: {}",
        ui.ctx
            .memory()
            .interaction
            .focus
            .focused()
            .as_ref()
            .map(Id::short_debug_format)
            .unwrap_or_default()
    ))
    .on_hover_text(ui.ctx, "Is egui currently listening for text input?");

    let pointer_pos = ui
        .ctx
        .pointer_hover_pos()
        .map_or_else(String::new, |pos| format!("{:?}", pos));
    ui.label(format!("Pointer pos: {}", pointer_pos));

    let top_layer = ui
        .ctx
        .pointer_hover_pos()
        .and_then(|pos| ui.ctx.layer_id_at(pos))
        .map_or_else(String::new, |layer| layer.short_debug_format());
    ui.label(format!("Top layer under mouse: {}", top_layer));

    ui.add_space(16.0);

    ui.label(format!(
        "There are {} text galleys in the layout cache",
        ui.ctx.fonts().num_galleys_in_cache()
    ))
    .on_hover_text(
        ui.ctx,
        "This is approximately the number of text strings on screen",
    );
    ui.add_space(16.0);

    CollapsingHeader::new("📥 Input")
        .default_open(false)
        .show(ui, |ui| {
            let input = ui.input().clone();
            input.ui(ui);
        });

    CollapsingHeader::new("📊 Paint stats")
        .default_open(false)
        .show(ui, |ui| {
            let paint_stats = ui.ctx.paint_stats().clone();
            paint_stats.ui(ui);
        });

    CollapsingHeader::new("🖼 Textures")
        .default_open(false)
        .show(ui, |ui| {
            texture_ui(ui);
        });

    CollapsingHeader::new("🔠 Font texture")
        .default_open(false)
        .show(ui, |ui| {
            let font_image_size = ui.ctx.fonts().font_image_size();
            crate::introspection::font_texture_ui(ui, font_image_size);
        });
}

/// Show stats about the allocated textures.
pub fn texture_ui(ui: &mut Ui<'_>) {
    let tex_mngr = ui.ctx.tex_manager().clone();
    let tex_mngr = tex_mngr.read();

    let mut textures: Vec<_> = tex_mngr.allocated().collect();
    textures.sort_by_key(|(id, _)| *id);

    let mut bytes = 0;
    for (_, tex) in &textures {
        bytes += tex.bytes_used();
    }

    ui.label(format!(
        "{} allocated texture(s), using {:.1} MB",
        textures.len(),
        bytes as f64 * 1e-6
    ));
    let max_preview_size = Vec2::new(48.0, 32.0);

    ui.group(|ui| {
        ScrollArea::vertical()
            .max_height(300.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.style_mut().override_text_style = Some(TextStyle::Monospace);
                Grid::new("textures")
                    .striped(true)
                    .num_columns(4)
                    .spacing(Vec2::new(16.0, 2.0))
                    .min_row_height(max_preview_size.y)
                    .show(ui, |ui| {
                        for (&texture_id, meta) in textures {
                            let [w, h] = meta.size;

                            let mut size = Vec2::new(w as f32, h as f32);
                            size *= (max_preview_size.x / size.x).min(1.0);
                            size *= (max_preview_size.y / size.y).min(1.0);
                            ui.image(texture_id, size).on_hover_ui(ui.ctx, |ui| {
                                // show larger on hover
                                let max_size = 0.5 * ui.ctx.input().screen_rect().size();
                                let mut size = Vec2::new(w as f32, h as f32);
                                size *= max_size.x / size.x.max(max_size.x);
                                size *= max_size.y / size.y.max(max_size.y);
                                ui.image(texture_id, size);
                            });

                            ui.label(format!("{} x {}", w, h));
                            ui.label(format!("{:.3} MB", meta.bytes_used() as f64 * 1e-6));
                            ui.label(format!("{:?}", meta.name));
                            ui.end_row();
                        }
                    });
            });
    });
}

pub fn memory_ui(ui: &mut Ui<'_>) {
    if ui
        .button("Reset all")
        .on_hover_text(ui.ctx, "Reset all egui state")
        .clicked()
    {
        *ui.ctx.memory_mut() = Default::default();
    }

    let num_state = ui.ctx.data().len();
    let num_serialized = ui.ctx.data_mut().count_serialized();
    ui.label(format!(
        "{} widget states stored (of which {} are serialized).",
        num_state, num_serialized
    ));

    ui.horizontal(|ui| {
        ui.label(format!(
            "{} areas (panels, windows, popups, …)",
            ui.ctx.memory().areas.count()
        ));
        if ui.button("Reset").clicked() {
            ui.ctx.memory_mut().areas = Default::default();
        }
    });
    ui.indent("areas", |ui| {
        ui.label("Visible areas, ordered back to front.");
        ui.label("Hover to highlight");
        for layer_id in ui.ctx.memory().areas.order().to_owned() {
            let area = ui.ctx.memory().areas.get(layer_id.id).cloned();
            if let Some(area) = area {
                if !ui.ctx.memory().areas.is_visible(&layer_id) {
                    continue;
                }
                let text = format!("{} - {:?}", layer_id.short_debug_format(), area.rect());
                // TODO(emilk): `Sense::hover_highlight()`
                if ui
                    .add(Label::new(RichText::new(text).monospace()).sense(Sense::click()))
                    .hovered()
                {
                    ui.ctx
                        .debug_painter()
                        .debug_rect(ui.ctx, area.rect(), Color32::RED, "");
                }
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label(format!(
            "{} collapsing headers",
            ui.ctx
                .data()
                .count::<containers::collapsing_header::InnerState>()
        ));
        if ui.button("Reset").clicked() {
            ui.ctx
                .data_mut()
                .remove_by_type::<containers::collapsing_header::InnerState>();
        }
    });

    ui.horizontal(|ui| {
        ui.label(format!(
            "{} menu bars",
            ui.ctx.data().count::<menu::BarState>()
        ));
        if ui.button("Reset").clicked() {
            ui.ctx.data_mut().remove_by_type::<menu::BarState>();
        }
    });

    ui.horizontal(|ui| {
        ui.label(format!(
            "{} scroll areas",
            ui.ctx.data().count::<scroll_area::State>()
        ));
        if ui.button("Reset").clicked() {
            ui.ctx.data_mut().remove_by_type::<scroll_area::State>();
        }
    });

    ui.horizontal(|ui| {
        ui.label(format!(
            "{} resize areas",
            ui.ctx.data().count::<resize::State>()
        ));
        if ui.button("Reset").clicked() {
            ui.ctx.data_mut().remove_by_type::<resize::State>();
        }
    });

    ui.shrink_width_to_current(); // don't let the text below grow this window wider
    ui.label("NOTE: the position of this window cannot be reset from within itself.");

    ui.collapsing("Interaction", |ui| {
        let interaction = ui.ctx.memory().interaction.clone();
        interaction.ui(ui);
    });
}

pub fn style_ui(ui: &mut Ui<'_>) {
    let mut style = (**ui.ctx.style()).clone();
    style.ui(ui);
    *ui.ctx.style_mut() = style;
}
