use crate::{math::*, types::*};

#[derive(Clone, Debug)]
pub struct Style {
    /// Show rectangles around each widget
    pub debug_rects: bool,

    /// For stuff like check marks in check boxes.
    pub line_width: f32,

    pub font_name: String,

    /// Height in pixels of most text.
    pub font_size: f32,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            debug_rects: false,
            line_width: 2.0,
            font_name: "Palatino".to_string(),
            font_size: 12.0,
        }
    }
}

fn debug_rect(rect: Rect) -> PaintCmd {
    PaintCmd::Rect {
        corner_radius: 0.0,
        fill_color: None,
        outline: Some(Outline {
            color: srgba(255, 255, 255, 255),
            width: 1.0,
        }),
        pos: rect.pos,
        size: rect.size,
    }
}

/// TODO: a Style struct which defines colors etc
fn translate_cmd(out_commands: &mut Vec<PaintCmd>, style: &Style, cmd: GuiCmd) {
    match cmd {
        GuiCmd::PaintCommands(mut commands) => out_commands.append(&mut commands),
        GuiCmd::Button {
            interact,
            rect,
            text,
        } => {
            let rect_fill_color = if interact.active {
                srgba(136, 136, 136, 255)
            } else if interact.hovered {
                srgba(100, 100, 100, 255)
            } else {
                srgba(68, 68, 68, 255)
            };
            out_commands.push(PaintCmd::Rect {
                corner_radius: 5.0,
                fill_color: Some(rect_fill_color),
                outline: None,
                pos: rect.pos,
                size: rect.size,
            });
            // TODO: clip-rect of text
            out_commands.push(PaintCmd::Text {
                fill_color: srgba(255, 255, 255, 187),
                font_name: style.font_name.clone(),
                font_size: style.font_size,
                pos: Vec2 {
                    x: rect.center().x,
                    y: rect.center().y - 6.0,
                },
                text,
                text_align: TextAlign::Center,
            });

            if style.debug_rects {
                out_commands.push(debug_rect(rect));
            }
        }
        GuiCmd::Checkbox {
            checked,
            interact,
            rect,
            text,
        } => {
            let fill_color = if interact.active {
                srgba(136, 136, 136, 255)
            } else if interact.hovered {
                srgba(100, 100, 100, 255)
            } else {
                srgba(68, 68, 68, 255)
            };

            let stroke_color = if interact.active {
                srgba(255, 255, 255, 255)
            } else if interact.hovered {
                srgba(255, 255, 255, 200)
            } else {
                srgba(255, 255, 255, 170)
            };

            let box_side = 16.0;
            let box_rect = Rect::from_center_size(
                vec2(rect.min().x + box_side * 0.5, rect.center().y),
                vec2(box_side, box_side),
            );
            out_commands.push(PaintCmd::Rect {
                corner_radius: 3.0,
                fill_color: Some(fill_color),
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
                    color: stroke_color,
                    width: style.line_width,
                });
            }

            out_commands.push(PaintCmd::Text {
                fill_color: stroke_color,
                font_name: style.font_name.clone(),
                font_size: style.font_size,
                pos: Vec2 {
                    x: box_rect.max().x + 4.0,
                    y: rect.center().y - 4.0,
                },
                text,
                text_align: TextAlign::Start,
            });

            if style.debug_rects {
                out_commands.push(debug_rect(rect));
            }
        }
        GuiCmd::FoldableHeader {
            interact,
            label,
            open,
            rect,
        } => {
            let fill_color = if interact.active {
                srgba(136, 136, 136, 255)
            } else if interact.hovered {
                srgba(100, 100, 100, 255)
            } else {
                srgba(68, 68, 68, 255)
            };

            let stroke_color = if interact.active {
                srgba(255, 255, 255, 255)
            } else if interact.hovered {
                srgba(255, 255, 255, 200)
            } else {
                srgba(255, 255, 255, 170)
            };

            out_commands.push(PaintCmd::Rect {
                corner_radius: 3.0,
                fill_color: Some(fill_color),
                outline: None,
                pos: rect.pos,
                size: rect.size,
            });

            // TODO: paint a little triangle or arrow or something instead of this

            let box_side = 16.0;
            let box_rect = Rect::from_center_size(
                vec2(rect.min().x + box_side * 0.5, rect.center().y),
                vec2(box_side, box_side),
            );
            // Draw a minus:
            out_commands.push(PaintCmd::Line {
                points: vec![
                    vec2(box_rect.min().x, box_rect.center().y),
                    vec2(box_rect.max().x, box_rect.center().y),
                ],
                color: stroke_color,
                width: style.line_width,
            });
            if open {
                // Draw it as a plus:
                out_commands.push(PaintCmd::Line {
                    points: vec![
                        vec2(box_rect.center().x, box_rect.min().y),
                        vec2(box_rect.center().x, box_rect.max().y),
                    ],
                    color: stroke_color,
                    width: style.line_width,
                });
            }

            out_commands.push(PaintCmd::Text {
                fill_color: stroke_color,
                font_name: style.font_name.clone(),
                font_size: style.font_size,
                pos: Vec2 {
                    x: box_rect.max().x + 4.0,
                    y: rect.center().y - style.font_size / 2.0,
                },
                text: label,
                text_align: TextAlign::Start,
            });
        }
        GuiCmd::RadioButton {
            checked,
            interact,
            rect,
            text,
        } => {
            let fill_color = if interact.active {
                srgba(136, 136, 136, 255)
            } else if interact.hovered {
                srgba(100, 100, 100, 255)
            } else {
                srgba(68, 68, 68, 255)
            };

            let stroke_color = if interact.active {
                srgba(255, 255, 255, 255)
            } else if interact.hovered {
                srgba(255, 255, 255, 200)
            } else {
                srgba(255, 255, 255, 170)
            };

            let circle_radius = 8.0;
            let circle_center = vec2(rect.min().x + circle_radius, rect.center().y);
            out_commands.push(PaintCmd::Circle {
                center: circle_center,
                fill_color: Some(fill_color),
                outline: None,
                radius: circle_radius,
            });

            if checked {
                out_commands.push(PaintCmd::Circle {
                    center: circle_center,
                    fill_color: Some(srgba(0, 0, 0, 255)),
                    outline: None,
                    radius: circle_radius * 0.5,
                });
            }

            out_commands.push(PaintCmd::Text {
                fill_color: stroke_color,
                font_name: style.font_name.clone(),
                font_size: style.font_size,
                pos: Vec2 {
                    x: rect.min().x + 2.0 * circle_radius + 4.0,
                    y: rect.center().y - 4.0,
                },
                text,
                text_align: TextAlign::Start,
            });

            if style.debug_rects {
                out_commands.push(debug_rect(rect));
            }
        }
        GuiCmd::Slider {
            interact,
            label,
            max,
            min,
            rect,
            value,
        } => {
            let thin_rect = Rect::from_min_size(
                vec2(rect.min().x, lerp(rect.min().y, rect.max().y, 2.0 / 3.0)),
                vec2(rect.size.x, 8.0),
            );

            let marker_center_x = remap_clamp(value, min, max, rect.min().x, rect.max().x);

            let marker_rect = Rect::from_center_size(
                vec2(marker_center_x, thin_rect.center().y),
                vec2(16.0, 16.0),
            );

            let marker_fill_color = if interact.active {
                srgba(136, 136, 136, 255)
            } else if interact.hovered {
                srgba(100, 100, 100, 255)
            } else {
                srgba(68, 68, 68, 255)
            };

            out_commands.push(PaintCmd::Rect {
                corner_radius: 2.0,
                fill_color: Some(srgba(34, 34, 34, 255)),
                outline: None,
                pos: thin_rect.pos,
                size: thin_rect.size,
            });

            out_commands.push(PaintCmd::Rect {
                corner_radius: 3.0,
                fill_color: Some(marker_fill_color),
                outline: None,
                pos: marker_rect.pos,
                size: marker_rect.size,
            });

            out_commands.push(PaintCmd::Text {
                fill_color: srgba(255, 255, 255, 187),
                font_name: style.font_name.clone(),
                font_size: style.font_size,
                pos: vec2(
                    rect.min().x,
                    lerp(rect.min().y, rect.max().y, 1.0 / 3.0) - 5.0,
                ),
                text: format!("{}: {:.3}", label, value),
                text_align: TextAlign::Start,
            });

            if style.debug_rects {
                out_commands.push(debug_rect(rect));
            }
        }
        GuiCmd::Text {
            pos,
            text,
            text_align,
            style: text_style,
        } => {
            let fill_color = match text_style {
                TextStyle::Label => srgba(255, 255, 255, 187),
            };
            out_commands.push(PaintCmd::Text {
                fill_color,
                font_name: style.font_name.clone(),
                font_size: style.font_size,
                pos: pos + vec2(0.0, style.font_size / 2.0 - 5.0), // TODO
                text,
                text_align,
            });
        }
    }
}

pub fn into_paint_commands(gui_commands: &[GuiCmd], style: &Style) -> Vec<PaintCmd> {
    let mut paint_commands = vec![];
    for gui_cmd in gui_commands {
        translate_cmd(&mut paint_commands, style, gui_cmd.clone())
    }
    paint_commands
}
