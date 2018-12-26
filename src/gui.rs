use crate::types::*;

// TODO: implement Gui on this so we can add children to a widget
// pub struct Widget {}

pub struct Gui {
    commands: Vec<GuiCmd>,
    input: GuiInput,

    cursor: Vec2,
}

impl Gui {
    pub fn new(input: GuiInput) -> Self {
        Gui {
            commands: vec![],
            input,
            cursor: Vec2 { x: 32.0, y: 32.0 },
        }
    }

    pub fn input(&self) -> &GuiInput {
        &self.input
    }

    pub fn into_commands(self) -> Vec<GuiCmd> {
        self.commands
    }

    pub fn paint_commands(&self) -> &[GuiCmd] {
        &self.commands
    }

    fn rect(&mut self, rect: Rect, style: RectStyle) -> InteractInfo {
        let hovered = rect.contains(self.input.mouse_pos);
        let clicked = hovered && self.input.mouse_clicked;
        let interact = InteractInfo { hovered, clicked };
        self.commands.push(GuiCmd::Rect {
            interact,
            rect,
            style,
        });
        interact
    }

    fn text<S: Into<String>>(&mut self, pos: Vec2, style: TextStyle, text: S) {
        self.commands.push(GuiCmd::Text {
            pos,
            style,
            text: text.into(),
            text_align: TextAlign::Start,
        });
    }

    // ------------------------------------------------------------------------

    pub fn button<S: Into<String>>(&mut self, text: S) -> InteractInfo {
        let rect = Rect {
            pos: self.cursor,
            size: Vec2 { x: 200.0, y: 32.0 }, // TODO: get from some settings
        };
        let interact = self.rect(rect, RectStyle::Button);

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
        for line in text.into().split("\n") {
            self.text(self.cursor, TextStyle::Label, line);
            self.cursor.y += 16.0;
        }
        self.cursor.y += 16.0; // Padding
    }

    // ------------------------------------------------------------------------
}
