use crate::types::*;

// TODO: implement Gui on this so we can add children to a widget
// pub struct Widget {}

type Id = u64;

#[derive(Clone, Copy, Debug, Default)]
pub struct GuiState {
    /// The widget being interacted with (e.g. dragged, in case of a slider).
    pub active_id: Option<Id>,
}

pub struct Gui {
    pub commands: Vec<GuiCmd>,
    pub cursor: Vec2,
    pub input: GuiInput,
    pub state: GuiState,
}

impl Gui {
    pub fn input(&self) -> &GuiInput {
        &self.input
    }

    pub fn gui_commands(&self) -> &[GuiCmd] {
        &self.commands
    }

    // ------------------------------------------------------------------------

    pub fn button<S: Into<String>>(&mut self, text: S) -> InteractInfo {
        let text: String = text.into();
        let id = self.get_id(&text);
        let rect = Rect {
            pos: self.cursor,
            size: Vec2 { x: 200.0, y: 32.0 }, // TODO: get from some settings
        };

        let hovered = rect.contains(self.input.mouse_pos);
        let clicked = hovered && self.input.mouse_clicked;
        if clicked {
            self.state.active_id = Some(id);
        }
        let active = self.state.active_id == Some(id);

        let interact = InteractInfo {
            hovered,
            clicked,
            active,
        };

        self.commands.push(GuiCmd::Rect {
            interact,
            rect,
            style: RectStyle::Button,
        });

        // TODO: clip-rect of text
        self.text(
            Vec2 {
                x: rect.pos.x + 8.0,
                y: rect.center().y + 14.0 / 2.0,
            },
            TextStyle::Button,
            text,
        );
        self.cursor.y += rect.size.y + 16.0;
        interact
    }

    pub fn label<S: Into<String>>(&mut self, text: S) {
        let text: String = text.into();
        for line in text.split('\n') {
            self.text(self.cursor, TextStyle::Label, line);
            self.cursor.y += 16.0;
        }
        self.cursor.y += 16.0; // Padding
    }

    // ------------------------------------------------------------------------

    fn get_id(&self, id_str: &str) -> Id {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        hasher.write(id_str.as_bytes());
        hasher.finish()
    }

    fn text<S: Into<String>>(&mut self, pos: Vec2, style: TextStyle, text: S) {
        self.commands.push(GuiCmd::Text {
            pos,
            style,
            text: text.into(),
            text_align: TextAlign::Start,
        });
    }
}
