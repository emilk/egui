use std::sync::Arc;

use crate::{layout, mesher::*, widgets::*, *};

#[derive(Clone, Copy, Default)]
struct Stats {
    num_batches: usize,
    num_vertices: usize,
    num_triangles: usize,
}

/// Encapsulates input, layout and painting for ease of use.
/// TODO: merge into Context
pub struct Emigui {
    pub last_input: RawInput,
    pub ctx: Arc<Context>,
    stats: Stats,
    mesher_options: MesherOptions,
}

impl Emigui {
    pub fn new(pixels_per_point: f32) -> Emigui {
        Emigui {
            last_input: Default::default(),
            ctx: Arc::new(Context::new(pixels_per_point)),
            stats: Default::default(),
            mesher_options: MesherOptions::default(),
        }
    }

    pub fn texture(&self) -> &Texture {
        self.ctx.fonts.texture()
    }

    pub fn new_frame(&mut self, new_input: RawInput) {
        let gui_input = GuiInput::from_last_and_new(&self.last_input, &new_input);
        self.last_input = new_input;

        // TODO: avoid this clone
        let mut new_data = (*self.ctx).clone();
        new_data.new_frame(gui_input);
        self.ctx = Arc::new(new_data);
    }

    /// A region for the entire screen, behind any windows.
    pub fn background_region(&mut self) -> Region {
        let child_rect = Rect::from_min_size(Default::default(), self.ctx.input.screen_size);
        Region {
            ctx: self.ctx.clone(),
            id: Id::background(),
            layer: Layer::Background,
            clip_rect: child_rect,
            desired_rect: child_rect,
            cursor: Default::default(),
            bounding_size: Default::default(),
            style: self.ctx.style(),
            dir: layout::Direction::Vertical,
            align: layout::Align::Center,
        }
    }

    pub fn paint(&mut self) -> PaintBatches {
        self.mesher_options.aa_size = 1.0 / self.last_input.pixels_per_point;
        let paint_commands = self.ctx.drain_paint_lists();
        let batches = mesh_paint_commands(&self.mesher_options, &self.ctx.fonts, paint_commands);
        self.stats = Default::default();
        self.stats.num_batches = batches.len();
        for (_, mesh) in &batches {
            self.stats.num_vertices += mesh.vertices.len();
            self.stats.num_triangles += mesh.indices.len() / 3;
        }
        batches
    }

    pub fn ui(&mut self, region: &mut Region) {
        region.foldable("Style", |region| {
            region.add(Checkbox::new(
                &mut self.mesher_options.anti_alias,
                "Antialias",
            ));
            region.add(Checkbox::new(
                &mut self.mesher_options.debug_paint_clip_rects,
                "Paint Clip Rects (debug)",
            ));
            self.ctx.style_ui(region);
        });

        region.foldable("Fonts", |region| {
            let old_font_definitions = self.ctx.fonts.definitions();
            let mut new_font_definitions = old_font_definitions.clone();
            font_definitions_ui(&mut new_font_definitions, region);
            self.ctx.fonts.texture().ui(region);
            if *old_font_definitions != new_font_definitions {
                let mut new_data = (*self.ctx).clone();
                let fonts =
                    Fonts::from_definitions(new_font_definitions, self.ctx.input.pixels_per_point);
                new_data.fonts = Arc::new(fonts);
                self.ctx = Arc::new(new_data);
            }
        });

        region.foldable("Stats", |region| {
            region.add(label!(
                "Screen size: {} x {} points, pixels_per_point: {}",
                region.input().screen_size.x,
                region.input().screen_size.y,
                region.input().pixels_per_point,
            ));
            if let Some(mouse_pos) = region.input().mouse_pos {
                region.add(label!("mouse_pos: {} x {}", mouse_pos.x, mouse_pos.y,));
            } else {
                region.add_label("mouse_pos: None");
            }
            region.add(label!(
                "region cursor: {} x {}",
                region.cursor().x,
                region.cursor().y,
            ));
            region.add(label!("num_batches: {}", self.stats.num_batches));
            region.add(label!("num_vertices: {}", self.stats.num_vertices));
            region.add(label!("num_triangles: {}", self.stats.num_triangles));
        });
    }
}

fn font_definitions_ui(font_definitions: &mut FontDefinitions, region: &mut Region) {
    for (text_style, (_family, size)) in font_definitions.iter_mut() {
        // TODO: radiobutton for family
        region.add(
            Slider::f32(size, 4.0, 40.0)
                .precision(0)
                .text(format!("{:?}", text_style)),
        );
    }
    if region.add(Button::new("Reset fonts")).clicked {
        *font_definitions = crate::fonts::default_font_definitions();
    }
}
