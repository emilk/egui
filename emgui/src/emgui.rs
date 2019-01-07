use std::sync::Arc;

use crate::{font::Font, layout, style, types::GuiInput, Frame, Painter, RawInput};

/// Encapsulates input, layout and painting for ease of use.
pub struct Emgui {
    pub last_input: RawInput,
    pub data: Arc<layout::Data>,
    pub style: style::Style,
    pub painter: Painter,
}

impl Emgui {
    pub fn new(font: Arc<Font>) -> Emgui {
        Emgui {
            last_input: Default::default(),
            data: Arc::new(layout::Data::new(font.clone())),
            style: Default::default(),
            painter: Painter::new(font),
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
        self.painter.paint(&paint_commands)
    }
}
