use std::sync::Arc;

use crate::{
    font::Font,
    layout,
    layout::{LayoutOptions, Region},
    style,
    types::GuiInput,
    widgets::*,
    Frame, Painter, RawInput,
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

/// Encapsulates input, layout and painting for ease of use.
pub struct Emigui {
    pub last_input: RawInput,
    pub data: Arc<layout::Data>,
    pub style: style::Style,
    pub painter: Painter,
    stats: Stats,
}

impl Emigui {
    pub fn new(font: Arc<Font>) -> Emigui {
        Emigui {
            last_input: Default::default(),
            data: Arc::new(layout::Data::new(font.clone())),
            style: Default::default(),
            painter: Painter::new(font),
            stats: Default::default(),
        }
    }

    pub fn texture(&self) -> (u16, u16, &[u8]) {
        self.painter.texture()
    }

    pub fn new_frame(&mut self, new_input: RawInput) {
        let gui_input = GuiInput::from_last_and_new(&self.last_input, &new_input);
        self.last_input = new_input;

        let mut new_data = (*self.data).clone();
        new_data.new_frame(gui_input);
        self.data = Arc::new(new_data);
    }

    pub fn whole_screen_region(&mut self) -> layout::Region {
        let size = self.data.input.screen_size;
        layout::Region {
            data: self.data.clone(),
            id: Default::default(),
            dir: layout::Direction::Vertical,
            cursor: Default::default(),
            bounding_size: Default::default(),
            available_space: size,
        }
    }

    pub fn options(&self) -> &layout::LayoutOptions {
        &self.data.options
    }

    pub fn set_options(&mut self, options: layout::LayoutOptions) {
        let mut new_data = (*self.data).clone();
        new_data.options = options;
        self.data = Arc::new(new_data);
    }

    pub fn paint(&mut self) -> Frame {
        let gui_commands = self.data.graphics.lock().unwrap().drain();
        let paint_commands = style::into_paint_commands(gui_commands, &self.style);
        let frame = self.painter.paint(&paint_commands);
        self.stats.num_vertices = frame.vertices.len();
        self.stats.num_triangles = frame.indices.len() / 3;
        frame
    }

    pub fn example(&mut self, region: &mut Region) {
        let mut options = self.options().clone();
        region.foldable("LayoutOptions", |gui| {
            show_options(&mut options, gui);
        });

        let mut style = self.style.clone();
        region.foldable("Style", |gui| {
            show_style(&mut style, gui);
        });

        region.foldable("Stats", |gui| {
            gui.add(label(format!("num_vertices: {}", self.stats.num_vertices)));
            gui.add(label(format!(
                "num_triangles: {}",
                self.stats.num_triangles
            )));
        });

        // self.set_options(options); // TODO
        self.style = style;
    }
}
