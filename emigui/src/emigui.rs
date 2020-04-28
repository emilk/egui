use std::sync::Arc;

use crate::{containers::*, mesher::*, widgets::*, *};

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

    pub fn begin_frame(&mut self, new_input: RawInput) {
        if !self.last_input.mouse_down || self.last_input.mouse_pos.is_none() {
            self.ctx.memory.lock().active_id = None;
        }

        let gui_input = GuiInput::from_last_and_new(&self.last_input, &new_input);
        self.last_input = new_input.clone(); // TODO: also stored in Context. Remove this one

        // TODO: avoid this clone
        let mut new_ctx = (*self.ctx).clone();

        new_ctx.last_raw_input = new_input;
        new_ctx.begin_frame(gui_input);
        self.ctx = Arc::new(new_ctx);
    }

    pub fn end_frame(&mut self) -> (Output, PaintBatches) {
        let output = self.ctx.end_frame();
        let paint_batches = self.paint();
        (output, paint_batches)
    }

    fn paint(&mut self) -> PaintBatches {
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

    /// A region for the entire screen, behind any windows.
    pub fn background_region(&mut self) -> Region {
        let rect = Rect::from_min_size(Default::default(), self.ctx.input.screen_size);
        Region::new(self.ctx.clone(), Layer::Background, Id::background(), rect)
    }
}

impl Emigui {
    pub fn ui(&mut self, region: &mut Region) {
        region.collapsing("Style", |region| {
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

        region.collapsing("Fonts", |region| {
            let old_font_definitions = self.ctx.fonts.definitions();
            let mut new_font_definitions = old_font_definitions.clone();
            font_definitions_ui(&mut new_font_definitions, region);
            self.ctx.fonts.texture().ui(region);
            if *old_font_definitions != new_font_definitions {
                let mut new_ctx = (*self.ctx).clone();
                let fonts =
                    Fonts::from_definitions(new_font_definitions, self.ctx.input.pixels_per_point);
                new_ctx.fonts = Arc::new(fonts);
                self.ctx = Arc::new(new_ctx);
            }
        });

        region.collapsing("Input", |region| {
            CollapsingHeader::new("Raw Input")
                .default_open()
                .show(region, |region| {
                    region.ctx().last_raw_input().clone().ui(region)
                });
            CollapsingHeader::new("Input")
                .default_open()
                .show(region, |region| region.input().clone().ui(region));
        });

        region.collapsing("Stats", |region| {
            region.add(label!(
                "Screen size: {} x {} points, pixels_per_point: {}",
                region.input().screen_size.x,
                region.input().screen_size.y,
                region.input().pixels_per_point,
            ));
            if let Some(mouse_pos) = region.input().mouse_pos {
                region.add(label!("mouse_pos: {:.2} x {:.2}", mouse_pos.x, mouse_pos.y,));
            } else {
                region.add_label("mouse_pos: None");
            }
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
            Slider::f32(size, 4.0..=40.0)
                .precision(0)
                .text(format!("{:?}", text_style)),
        );
    }
    if region.add(Button::new("Reset fonts")).clicked {
        *font_definitions = crate::fonts::default_font_definitions();
    }
}

impl RawInput {
    pub fn ui(&self, region: &mut Region) {
        // TODO: simpler way to show values, e.g. `region.value("Mouse Pos:", self.mouse_pos);
        region.add(label!("mouse_down: {}", self.mouse_down));
        region.add(label!("mouse_pos: {:.1?}", self.mouse_pos));
        region.add(label!("scroll_delta: {:?}", self.scroll_delta));
        region.add(label!("screen_size: {:?}", self.screen_size));
        region.add(label!("pixels_per_point: {}", self.pixels_per_point));
        region.add(label!("time: {:.3} s", self.time));
        region.add(label!("text: {:?}", self.text));
        // region.add(label!("dropped_files: {}", self.dropped_files));
        // region.add(label!("hovered_files: {}", self.hovered_files));
    }
}

impl GuiInput {
    pub fn ui(&self, region: &mut Region) {
        region.add(label!("mouse_down: {}", self.mouse_down));
        region.add(label!("mouse_pressed: {}", self.mouse_pressed));
        region.add(label!("mouse_released: {}", self.mouse_released));
        region.add(label!("mouse_pos: {:?}", self.mouse_pos));
        region.add(label!("mouse_move: {:?}", self.mouse_move));
        region.add(label!("scroll_delta: {:?}", self.scroll_delta));
        region.add(label!("screen_size: {:?}", self.screen_size));
        region.add(label!("pixels_per_point: {}", self.pixels_per_point));
        region.add(label!("time: {}", self.time));
        region.add(label!("text: {:?}", self.text));
        // region.add(label!("dropped_files: {}", self.dropped_files));
        // region.add(label!("hovered_files: {}", self.hovered_files));
    }
}
