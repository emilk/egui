use crate::{font::Font, layout, style, types::*};

/// Encapsulates input, layout and painting for ease of use.
#[derive(Clone)]
pub struct Emgui {
    pub last_input: RawInput,
    pub layout: layout::Layout,
    pub style: style::Style,
}

impl Emgui {
    pub fn new(font: Font) -> Emgui {
        Emgui {
            last_input: Default::default(),
            layout: layout::Layout::new(font),
            style: Default::default(),
        }
    }

    pub fn new_frame(&mut self, new_input: RawInput) {
        let gui_input = GuiInput::from_last_and_new(&self.last_input, &new_input);
        self.last_input = new_input;
        self.layout.new_frame(gui_input);
    }

    pub fn paint(&mut self) -> Vec<PaintCmd> {
        style::into_paint_commands(self.layout.gui_commands(), &self.style)
    }
}
