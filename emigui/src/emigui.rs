use std::sync::Arc;

use crate::{
    label, layout,
    layout::Region,
    mesher::Mesher,
    types::{GuiInput, PaintCmd},
    widgets::*,
    FontDefinitions, Fonts, Mesh, RawInput, Texture,
};

#[derive(Clone, Copy, Default)]
struct Stats {
    num_vertices: usize,
    num_triangles: usize,
}

/// Encapsulates input, layout and painting for ease of use.
pub struct Emigui {
    pub last_input: RawInput,
    pub data: Arc<layout::Data>,
    stats: Stats,
}

impl Emigui {
    pub fn new(pixels_per_point: f32) -> Emigui {
        Emigui {
            last_input: Default::default(),
            data: Arc::new(layout::Data::new(pixels_per_point)),
            stats: Default::default(),
        }
    }

    pub fn texture(&self) -> &Texture {
        self.data.fonts.texture()
    }

    pub fn new_frame(&mut self, new_input: RawInput) {
        let gui_input = GuiInput::from_last_and_new(&self.last_input, &new_input);
        self.last_input = new_input;

        // TODO: avoid this clone
        let mut new_data = (*self.data).clone();
        new_data.new_frame(gui_input);
        self.data = Arc::new(new_data);
    }

    pub fn whole_screen_region(&mut self) -> layout::Region {
        let size = self.data.input.screen_size;
        layout::Region {
            data: self.data.clone(),
            style: self.data.style(),
            id: Default::default(),
            dir: layout::Direction::Vertical,
            align: layout::Align::Center,
            cursor: Default::default(),
            bounding_size: Default::default(),
            available_space: size,
        }
    }

    pub fn paint(&mut self) -> Mesh {
        let paint_commands: Vec<PaintCmd> = self.data.graphics.lock().unwrap().drain().collect();
        let mut mesher = Mesher::new(self.last_input.pixels_per_point);
        mesher.paint(&self.data.fonts, &paint_commands);
        let mesh = mesher.mesh;
        self.stats.num_vertices = mesh.vertices.len();
        self.stats.num_triangles = mesh.indices.len() / 3;
        mesh
    }

    pub fn ui(&mut self, region: &mut Region) {
        region.foldable("Style", |region| {
            self.data.style_ui(region);
        });

        region.foldable("Fonts", |region| {
            let old_font_definitions = self.data.fonts.definitions();
            let mut new_font_definitions = old_font_definitions.clone();
            font_definitions_ui(&mut new_font_definitions, region);
            self.data.fonts.texture().ui(region);
            if *old_font_definitions != new_font_definitions {
                let mut new_data = (*self.data).clone();
                let fonts =
                    Fonts::from_definitions(new_font_definitions, self.data.input.pixels_per_point);
                new_data.fonts = Arc::new(fonts);
                self.data = Arc::new(new_data);
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
                region.add(label!("mouse_pos: None"));
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
