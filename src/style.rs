use crate::{math::*, types::*};

/// TODO: a Style struct which defines colors etc
fn translate_cmd(out_commands: &mut Vec<PaintCmd>, cmd: GuiCmd) {
    match cmd {
        GuiCmd::Rect {
            rect,
            style,
            interact,
        } => match style {
            RectStyle::Button => {
                let fill_style = if interact.active {
                    "#888888ff".to_string()
                } else if interact.hovered {
                    "#444444ff".to_string()
                } else {
                    "#222222ff".to_string()
                };
                out_commands.push(PaintCmd::RoundedRect {
                    corner_radius: 5.0,
                    fill_style,
                    pos: rect.pos,
                    size: rect.size,
                });
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
            out_commands.push(PaintCmd::Text {
                fill_style,
                font: "14px Palatino".to_string(),
                pos,
                text,
                text_align,
            });
        }
        GuiCmd::Slider {
            interact,
            label,
            max,
            min,
            rect,
            value,
        } => {
            let thin_rect = Rect::from_center_size(rect.center(), vec2(rect.size.x, 8.0));

            let marker_center_x = remap_clamp(value, min, max, rect.min().x, rect.max().x);

            let marker_rect =
                Rect::from_center_size(vec2(marker_center_x, rect.center().y), vec2(16.0, 16.0));

            let marker_fill_style = if interact.active {
                "#888888ff".to_string()
            } else if interact.hovered {
                "#444444ff".to_string()
            } else {
                "#222222ff".to_string()
            };

            out_commands.push(PaintCmd::RoundedRect {
                corner_radius: 2.0,
                fill_style: "#111111ff".to_string(),
                pos: thin_rect.pos,
                size: thin_rect.size,
            });

            out_commands.push(PaintCmd::RoundedRect {
                corner_radius: 3.0,
                fill_style: marker_fill_style,
                pos: marker_rect.pos,
                size: marker_rect.size,
            });

            out_commands.push(PaintCmd::Text {
                fill_style: "#ffffffbb".to_string(),
                font: "14px Palatino".to_string(),
                pos: rect.center(),
                text: format!("{}: {:.3}", label, value),
                text_align: TextAlign::Center,
            });
        }
    }
}

pub fn into_paint_commands(gui_commands: &[GuiCmd]) -> Vec<PaintCmd> {
    let mut paint_commands = vec![];
    for gui_cmd in gui_commands {
        translate_cmd(&mut paint_commands, gui_cmd.clone())
    }
    paint_commands
}
