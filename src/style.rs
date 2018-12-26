use crate::types::*;

/// TODO: a Style struct which defines colors etc
fn translate_cmd(cmd: GuiCmd) -> PaintCmd {
    match cmd {
        GuiCmd::Rect {
            rect,
            style,
            interact,
        } => match style {
            RectStyle::Button => {
                let fill_style = if interact.hovered {
                    "#444444ff".to_string()
                } else {
                    "#222222ff".to_string()
                };
                PaintCmd::RoundedRect {
                    corner_radius: 5.0,
                    fill_style,
                    pos: rect.pos,
                    size: rect.size,
                }
            }
        },
        GuiCmd::Text {
            pos,
            text,
            text_align,
            style,
        } => {
            let fill_style = match style {
                TextStyle::Button => "#ffffffbb".to_string(),
                TextStyle::Label => "#ffffffbb".to_string(),
            };
            PaintCmd::Text {
                fill_style,
                font: "14px Palatino".to_string(),
                pos,
                text,
                text_align,
            }
        }
    }
}

pub fn into_paint_commands(gui_commands: Vec<GuiCmd>) -> Vec<PaintCmd> {
    gui_commands.into_iter().map(translate_cmd).collect()
}
