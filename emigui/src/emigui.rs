use std::sync::Arc;

use crate::{
    layout,
    layout::{LayoutOptions, Region},
    math::vec2,
    mesher::Vertex,
    style,
    types::{Color, GuiCmd, GuiInput, PaintCmd},
    widgets::*,
    FontSizes, Fonts, Frame, RawInput, Texture,
};

#[derive(Clone, Copy, Default)]
struct Stats {
    num_vertices: usize,
    num_triangles: usize,
}

fn show_options(options: &mut LayoutOptions, gui: &mut Region) {
    if gui.add(Button::new("Reset LayoutOptions")).clicked {
        *options = Default::default();
    }
    gui.add(Slider::new(&mut options.item_spacing.x, 0.0, 10.0).text("item_spacing.x"));
    gui.add(Slider::new(&mut options.item_spacing.y, 0.0, 10.0).text("item_spacing.y"));
    gui.add(Slider::new(&mut options.window_padding.x, 0.0, 10.0).text("window_padding.x"));
    gui.add(Slider::new(&mut options.window_padding.y, 0.0, 10.0).text("window_padding.y"));
    gui.add(Slider::new(&mut options.indent, 0.0, 100.0).text("indent"));
    gui.add(Slider::new(&mut options.button_padding.x, 0.0, 20.0).text("button_padding.x"));
    gui.add(Slider::new(&mut options.button_padding.y, 0.0, 20.0).text("button_padding.y"));
    gui.add(Slider::new(&mut options.start_icon_width, 0.0, 60.0).text("start_icon_width"));
}

fn show_style(style: &mut style::Style, gui: &mut Region) {
    if gui.add(Button::new("Reset Style")).clicked {
        *style = Default::default();
    }
    gui.add(Checkbox::new(&mut style.debug_rects, "debug_rects"));
    gui.add(Slider::new(&mut style.line_width, 0.0, 10.0).text("line_width"));
}

fn show_font_sizes(font_sizes: &mut FontSizes, gui: &mut Region) {
    for (text_style, mut size) in font_sizes {
        gui.add(Slider::new(&mut size, 4.0, 40.0).text(format!("{:?}", text_style)));
    }
}

/// Encapsulates input, layout and painting for ease of use.
pub struct Emigui {
    pub last_input: RawInput,
    pub data: Arc<layout::Data>,
    pub style: style::Style,
    stats: Stats,
}

impl Emigui {
    pub fn new() -> Emigui {
        Emigui {
            last_input: Default::default(),
            data: Arc::new(layout::Data::new()),
            style: Default::default(),
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
            options: self.data.options(),
            id: Default::default(),
            dir: layout::Direction::Vertical,
            align: layout::Align::Center,
            cursor: Default::default(),
            bounding_size: Default::default(),
            available_space: size,
        }
    }

    pub fn paint(&mut self) -> Frame {
        let gui_commands = self.data.graphics.lock().unwrap().drain();
        let paint_commands = style::into_paint_commands(gui_commands, &self.style);
        let frame = Frame::paint(&self.data.fonts, &paint_commands);
        self.stats.num_vertices = frame.vertices.len();
        self.stats.num_triangles = frame.indices.len() / 3;
        frame
    }

    pub fn example(&mut self, region: &mut Region) {
        region.foldable("LayoutOptions", |gui| {
            let mut options = self.data.options();
            show_options(&mut options, gui);
            self.data.set_options(options);
        });

        region.foldable("Style", |gui| {
            show_style(&mut self.style, gui);
        });

        region.foldable("Fonts", |gui| {
            let texture = self.texture();
            gui.add(label(format!(
                "Font texture size: {} x {}",
                texture.width, texture.height
            )));
            let size = vec2(texture.width as f32, texture.height as f32);
            let rect = gui.reserve_space(size, None).rect;
            let top_left = Vertex {
                pos: rect.min(),
                uv: (0, 0),
                color: Color::WHITE,
            };
            let bottom_right = Vertex {
                pos: rect.max(),
                uv: (texture.width as u16 - 1, texture.height as u16 - 1),
                color: Color::WHITE,
            };
            let mut frame = Frame::default();
            frame.add_rect(top_left, bottom_right);
            gui.add_graphic(GuiCmd::PaintCommands(vec![PaintCmd::Frame(frame)]));

            let old_font_sizes = self.data.fonts.sizes();
            let mut new_font_sizes = old_font_sizes.clone();
            show_font_sizes(&mut new_font_sizes, gui);
            if *old_font_sizes != new_font_sizes {
                let mut new_data = (*self.data).clone();
                let fonts = Fonts::from_sizes(new_font_sizes);
                new_data.fonts = Arc::new(fonts);
                self.data = Arc::new(new_data);
            }
        });

        region.foldable("Stats", |gui| {
            gui.add(label(format!("num_vertices: {}", self.stats.num_vertices)));
            gui.add(label(format!(
                "num_triangles: {}",
                self.stats.num_triangles
            )));
        });
    }
}
