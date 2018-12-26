use types::*;

#[derive(Default)]
pub struct InteractInfo {
    pub hovering: bool,
    pub clicked: bool,
}

// TODO: implement Gui on this so we can add children to a widget
// pub struct Widget {}

pub struct Gui {
    commands: Vec<PaintCmd>,
    input: GuiInput,

    cursor: Vec2,
}

impl Gui {
    pub fn new(input: GuiInput) -> Self {
        Gui {
            commands: vec![PaintCmd::Clear {
                fill_style: "#44444400".to_string(),
            }],
            input,
            cursor: Vec2 { x: 32.0, y: 32.0 },
        }
    }

    pub fn input(&self) -> &GuiInput {
        &self.input
    }

    pub fn into_commands(self) -> Vec<PaintCmd> {
        self.commands
    }

    pub fn paint_commands(&self) -> &[PaintCmd] {
        &self.commands
    }

    fn rect(&mut self, rect: Rect, fill_style: String) -> InteractInfo {
        let hovering = rect.contains(self.input.mouse_pos);
        let clicked = hovering && self.input.mouse_clicked;
        let ii = InteractInfo { hovering, clicked };
        self.commands.push(PaintCmd::RoundedRect {
            fill_style,
            pos: rect.pos,
            corner_radius: 5.0,
            size: rect.size,
        });
        ii
    }

    fn text<S: Into<String>>(&mut self, pos: Vec2, text: S) {
        self.commands.push(PaintCmd::Text {
            fill_style: "#ffffffbb".to_string(),
            font: "14px Palatino".to_string(),
            pos,
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
        let hovering = rect.contains(self.input.mouse_pos);
        let fill_style = if hovering {
            "#444444ff".to_string()
        } else {
            "#222222ff".to_string()
        };
        let ii = self.rect(rect, fill_style);
        self.text(
            Vec2 {
                x: rect.pos.x + 4.0,
                y: rect.center().y + 14.0 / 2.0,
            },
            text,
        );
        self.cursor.y += rect.size.y + 16.0;
        ii
    }

    pub fn label<S: Into<String>>(&mut self, text: S) {
        for line in text.into().split("\n") {
            let pos = self.cursor;
            self.text(pos, line);
            self.cursor.y += 16.0;
        }
        self.cursor.y += 16.0; // Padding
    }

    // ------------------------------------------------------------------------
}
