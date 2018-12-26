use crate::{math::*, types::*};

/// TODO: a Style struct which defines colors etc
fn translate_cmd(out_commands: &mut Vec<PaintCmd>, cmd: GuiCmd) {
    match cmd {
        GuiCmd::PaintCommands(mut commands) => out_commands.append(&mut commands),
        GuiCmd::Button {
            interact,
            rect,
            text,
        } => {
            let rect_fill_style = if interact.active {
                "#888888ff".to_string()
            } else if interact.hovered {
                "#666666ff".to_string()
            } else {
                "#444444ff".to_string()
            };
            out_commands.push(PaintCmd::Rect {
                corner_radius: 5.0,
                fill_style: Some(rect_fill_style),
                outline: None,
                pos: rect.pos,
                size: rect.size,
            });
            // TODO: clip-rect of text
            out_commands.push(PaintCmd::Text {
                fill_style: "#ffffffbb".to_string(),
                font: "14px Palatino".to_string(),
                pos: Vec2 {
                    x: rect.center().x,
                    y: rect.center().y + 14.0 / 2.0,
                },
                text,
                text_align: TextAlign::Center,
            });
        }
        GuiCmd::Checkbox {
            checked,
            interact,
            rect,
            text,
        } => {
            let fill_style = if interact.active {
                "#888888ff".to_string()
            } else if interact.hovered {
                "#666666ff".to_string()
            } else {
                "#444444ff".to_string()
            };

            let stroke_style = if interact.active {
                "#ffffffff".to_string()
            } else if interact.hovered {
                "#ffffffcc".to_string()
            } else {
                "#ffffffaa".to_string()
            };

            let box_side = 16.0;
            let box_rect = Rect::from_center_size(
                vec2(rect.min().x + box_side * 0.5, rect.center().y),
                vec2(box_side, box_side),
            );
            out_commands.push(PaintCmd::Rect {
                corner_radius: 3.0,
                fill_style: Some(fill_style),
                outline: None,
                pos: box_rect.pos,
                size: box_rect.size,
            });

            if checked {
                let smaller_rect = Rect::from_center_size(box_rect.center(), vec2(10.0, 10.0));
                out_commands.push(PaintCmd::Line {
                    points: vec![
                        vec2(smaller_rect.min().x, smaller_rect.center().y),
                        vec2(smaller_rect.center().x, smaller_rect.max().y),
                        vec2(smaller_rect.max().x, smaller_rect.min().y),
                    ],
                    style: stroke_style.clone(),
                    width: 4.0,
                });
            }

            out_commands.push(PaintCmd::Text {
                fill_style: stroke_style.clone(),
                font: "14px Palatino".to_string(),
                pos: Vec2 {
                    x: box_rect.max().x + 4.0,
                    y: rect.center().y + 14.0 / 2.0,
                },
                text,
                text_align: TextAlign::Start,
            });
        }
        GuiCmd::RadioButton {
            checked,
            interact,
            rect,
            text,
        } => {
            let fill_style = if interact.active {
                "#888888ff".to_string()
            } else if interact.hovered {
                "#666666ff".to_string()
            } else {
                "#444444ff".to_string()
            };

            let stroke_style = if interact.active {
                "#ffffffff".to_string()
            } else if interact.hovered {
                "#ffffffcc".to_string()
            } else {
                "#ffffffaa".to_string()
            };

            let circle_radius = 8.0;
            let circle_center = vec2(rect.min().x + circle_radius, rect.center().y);
            out_commands.push(PaintCmd::Circle {
                center: circle_center,
                fill_style: Some(fill_style),
                outline: None,
                radius: circle_radius,
            });

            if checked {
                out_commands.push(PaintCmd::Circle {
                    center: circle_center,
                    fill_style: Some("#000000ff".to_string()),
                    outline: None,
                    radius: circle_radius * 0.5,
                });
            }

            out_commands.push(PaintCmd::Text {
                fill_style: stroke_style.clone(),
                font: "14px Palatino".to_string(),
                pos: Vec2 {
                    x: rect.min().x + 2.0 * circle_radius + 4.0,
                    y: rect.center().y + 14.0 / 2.0,
                },
                text,
                text_align: TextAlign::Start,
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
                "#666666ff".to_string()
            } else {
                "#444444ff".to_string()
            };

            out_commands.push(PaintCmd::Rect {
                corner_radius: 2.0,
                fill_style: Some("#222222ff".to_string()),
                outline: None,
                pos: thin_rect.pos,
                size: thin_rect.size,
            });

            out_commands.push(PaintCmd::Rect {
                corner_radius: 3.0,
                fill_style: Some(marker_fill_style),
                outline: None,
                pos: marker_rect.pos,
                size: marker_rect.size,
            });

            out_commands.push(PaintCmd::Text {
                fill_style: "#ffffffbb".to_string(),
                font: "14px Palatino".to_string(),
                pos: rect.min(),
                text: format!("{}: {:.3}", label, value),
                text_align: TextAlign::Start,
            });
        }
        GuiCmd::Text {
            pos,
            text,
            text_align,
            style,
        } => {
            let fill_style = match style {
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
    }
}

pub fn into_paint_commands(gui_commands: &[GuiCmd]) -> Vec<PaintCmd> {
    let mut paint_commands = vec![];
    for gui_cmd in gui_commands {
        translate_cmd(&mut paint_commands, gui_cmd.clone())
    }
    paint_commands
}
