use std::sync::Arc;

use crate::{layout, mesher::Mesher, widgets::*, *};

#[derive(Clone, Copy, Default)]
struct Stats {
    num_vertices: usize,
    num_triangles: usize,
}

/// Encapsulates input, layout and painting for ease of use.
pub struct Emigui {
    pub last_input: RawInput,
    pub ctx: Arc<Context>,
    stats: Stats,
    anti_alias: bool,
}

impl Emigui {
    pub fn new(pixels_per_point: f32) -> Emigui {
        Emigui {
            last_input: Default::default(),
            ctx: Arc::new(Context::new(pixels_per_point)),
            stats: Default::default(),
            anti_alias: true,
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
        Region {
            ctx: self.ctx.clone(),
            layer: Layer::Background,
            style: self.ctx.style(),
            id: Id::background(),
            dir: layout::Direction::Vertical,
            align: layout::Align::Center,
            rect: Rect::from_min_size(Default::default(), self.ctx.input.screen_size),
            cursor: Default::default(),
            bounding_size: Default::default(),
        }
    }

    pub fn paint(&mut self) -> Mesh {
        let paint_commands = self.ctx.drain_paint_lists();
        let mut mesher = Mesher::new(self.last_input.pixels_per_point);
        mesher.options.anti_alias = self.anti_alias;

        mesher.paint(&self.ctx.fonts, &paint_commands);
        let mesh = mesher.mesh;
        self.stats.num_vertices = mesh.vertices.len();
        self.stats.num_triangles = mesh.indices.len() / 3;
        mesh
    }

    pub fn ui(&mut self, region: &mut Region) {
        region.foldable("Style", |region| {
            region.add(Checkbox::new(&mut self.anti_alias, "Antialias"));
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
