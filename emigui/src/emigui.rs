use std::sync::Arc;

use crate::{
    label, layout,
    layout::{show_popup, LayoutOptions, Region},
    math::{clamp, remap_clamp, vec2},
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
    gui.add(Slider::new(&mut options.clickable_diameter, 0.0, 60.0).text("clickable_diameter"));
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

fn show_font_texture(texture: &Texture, gui: &mut Region) {
    gui.add(label!(
        "Font texture size: {} x {} (hover to zoom)",
        texture.width,
        texture.height
    ));
    let mut size = vec2(texture.width as f32, texture.height as f32);
    if size.x > gui.width() {
        size *= gui.width() / size.x;
    }
    let interact = gui.reserve_space(size, None);
    let rect = interact.rect;
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

    if interact.hovered {
        show_popup(gui.data(), gui.input().mouse_pos, |gui| {
            let zoom_rect = gui.reserve_space(vec2(128.0, 128.0), None).rect;
            let u = remap_clamp(
                gui.input().mouse_pos.x,
                rect.min().x,
                rect.max().x,
                0.0,
                texture.width as f32 - 1.0,
            )
            .round();
            let v = remap_clamp(
                gui.input().mouse_pos.y,
                rect.min().y,
                rect.max().y,
                0.0,
                texture.height as f32 - 1.0,
            )
            .round();

            let texel_radius = 32.0;
            let u = clamp(u, texel_radius, texture.width as f32 - 1.0 - texel_radius);
            let v = clamp(v, texel_radius, texture.height as f32 - 1.0 - texel_radius);

            let top_left = Vertex {
                pos: zoom_rect.min(),
                uv: ((u - texel_radius) as u16, (v - texel_radius) as u16),
                color: Color::WHITE,
            };
            let bottom_right = Vertex {
                pos: zoom_rect.max(),
                uv: ((u + texel_radius) as u16, (v + texel_radius) as u16),
                color: Color::WHITE,
            };
            let mut frame = Frame::default();
            frame.add_rect(top_left, bottom_right);
            gui.add_graphic(GuiCmd::PaintCommands(vec![PaintCmd::Frame(frame)]));
        });
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
    pub fn new(pixels_per_point: f32) -> Emigui {
        Emigui {
            last_input: Default::default(),
            data: Arc::new(layout::Data::new(pixels_per_point)),
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
            let old_font_sizes = self.data.fonts.sizes();
            let mut new_font_sizes = old_font_sizes.clone();
            show_font_sizes(&mut new_font_sizes, gui);
            show_font_texture(self.texture(), gui);
            if *old_font_sizes != new_font_sizes {
                let mut new_data = (*self.data).clone();
                let fonts = Fonts::from_sizes(new_font_sizes, self.data.input.pixels_per_point);
                new_data.fonts = Arc::new(fonts);
                self.data = Arc::new(new_data);
            }
        });

        region.foldable("Stats", |gui| {
            gui.add(label!(
                "Screen size: {} x {} points, pixels_per_point: {}",
                gui.input().screen_size.x,
                gui.input().screen_size.y,
                gui.input().pixels_per_point,
            ));
            gui.add(label!(
                "mouse_pos: {} x {}",
                gui.input().mouse_pos.x,
                gui.input().mouse_pos.y,
            ));
            gui.add(label!(
                "gui cursor: {} x {}",
                gui.cursor().x,
                gui.cursor().y,
            ));
            gui.add(label!("num_vertices: {}", self.stats.num_vertices));
            gui.add(label!("num_triangles: {}", self.stats.num_triangles));
        });
    }
}
