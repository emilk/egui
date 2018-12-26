use crate::{layout, math::*, style, types::*};

/// Encapsulates input, layout and painting for ease of use.
#[derive(Clone, Debug, Default)]
pub struct Emgui {
    pub last_input: RawInput,
    pub layout: layout::Layout,
    pub style: style::Style,
}

impl Emgui {
    pub fn new_frame(&mut self, new_input: RawInput) {
        let gui_input = GuiInput::from_last_and_new(&self.last_input, &new_input);
        self.last_input = new_input;

        // TODO: this should be nicer
        self.layout.commands.clear();
        self.layout.cursor = vec2(32.0, 32.0);
        self.layout.input = gui_input;
        if !gui_input.mouse_down {
            self.layout.state.active_id = None;
        }
    }

    pub fn paint(&mut self) -> Vec<PaintCmd> {
        style::into_paint_commands(self.layout.gui_commands(), &self.style)
    }
}
