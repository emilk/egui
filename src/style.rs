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
            // font_name: "Palatino".to_string(),
            font_name: "Courier".to_string(),
            // font_name: "Courier New".to_string(),
            font_size: 12.0,
        }
    }
}

impl Style {
    /// e.g. the background of the slider
    fn background_fill_color(&self) -> Color {
        srgba(34, 34, 34, 200)
    }

    fn text_color(&self) -> Color {
        srgba(255, 255, 255, 187)
    }

    /// Fill color of the interactive part of a component (button, slider grab, checkbox, ...)
    fn interact_fill_color(&self, interact: &InteractInfo) -> Color {
        if interact.active {
            srgba(136, 136, 136, 255)
        } else if interact.hovered {
            srgba(100, 100, 100, 255)
        } else {
            srgba(68, 68, 68, 220)
        }
    }

    /// Stroke and text color of the interactive part of a component (button, slider grab, checkbox, ...)
    fn interact_stroke_color(&self, interact: &InteractInfo) -> Color {
        if interact.active {
            srgba(255, 255, 255, 255)
        } else if interact.hovered {
            srgba(255, 255, 255, 200)
        } else {
            srgba(255, 255, 255, 170)
        }
    }

    /// Returns small icon rectangle and big icon rectangle
    fn icon_rectangles(&self, rect: &Rect) -> (Rect, Rect) {
        let box_side = 16.0;
        let big_icon_rect = Rect::from_center_size(
            vec2(rect.min().x + 4.0 + box_side * 0.5, rect.center().y),
            vec2(box_side, box_side),
        );

        let small_icon_rect = Rect::from_center_size(big_icon_rect.center(), vec2(10.0, 10.0));

        (small_icon_rect, big_icon_rect)
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
        GuiCmd::Button { interact, rect } => {
            out_commands.push(PaintCmd::Rect {
                corner_radius: 5.0,
                fill_color: Some(style.interact_fill_color(&interact)),
                outline: None,
                pos: rect.pos,
                size: rect.size,
            });
            if style.debug_rects {
                out_commands.push(debug_rect(rect));
            }
        }
        GuiCmd::Checkbox {
            checked,
            interact,
            rect,
        } => {
            let (small_icon_rect, big_icon_rect) = style.icon_rectangles(&rect);
            out_commands.push(PaintCmd::Rect {
                corner_radius: 3.0,
                fill_color: Some(style.interact_fill_color(&interact)),
                outline: None,
                pos: big_icon_rect.pos,
                size: big_icon_rect.size,
            });

            let stroke_color = style.interact_stroke_color(&interact);

            if checked {
                out_commands.push(PaintCmd::Line {
                    points: vec![
                        vec2(small_icon_rect.min().x, small_icon_rect.center().y),
                        vec2(small_icon_rect.center().x, small_icon_rect.max().y),
                        vec2(small_icon_rect.max().x, small_icon_rect.min().y),
                    ],
                    color: stroke_color,
                    width: style.line_width,
                });
            }

            if style.debug_rects {
                out_commands.push(debug_rect(rect));
            }
        }
        GuiCmd::FoldableHeader {
            interact,
            open,
            rect,
        } => {
            let fill_color = style.interact_fill_color(&interact);
            let stroke_color = style.interact_stroke_color(&interact);

            out_commands.push(PaintCmd::Rect {
                corner_radius: 3.0,
                fill_color: Some(fill_color),
                outline: None,
                pos: rect.pos,
                size: rect.size,
            });

            // TODO: paint a little triangle or arrow or something instead of this

            let (small_icon_rect, _) = style.icon_rectangles(&rect);
            // Draw a minus:
            out_commands.push(PaintCmd::Line {
                points: vec![
                    vec2(small_icon_rect.min().x, small_icon_rect.center().y),
                    vec2(small_icon_rect.max().x, small_icon_rect.center().y),
                ],
                color: stroke_color,
                width: style.line_width,
            });
            if !open {
                // Draw it as a plus:
                out_commands.push(PaintCmd::Line {
                    points: vec![
                        vec2(small_icon_rect.center().x, small_icon_rect.min().y),
                        vec2(small_icon_rect.center().x, small_icon_rect.max().y),
                    ],
                    color: stroke_color,
                    width: style.line_width,
                });
            }
        }
        GuiCmd::RadioButton {
            checked,
            interact,
            rect,
        } => {
            let fill_color = style.interact_fill_color(&interact);
            let stroke_color = style.interact_stroke_color(&interact);

            let (small_icon_rect, big_icon_rect) = style.icon_rectangles(&rect);

            out_commands.push(PaintCmd::Circle {
                center: big_icon_rect.center(),
                fill_color: Some(fill_color),
                outline: None,
                radius: big_icon_rect.size.x / 2.0,
            });

            if checked {
                out_commands.push(PaintCmd::Circle {
                    center: small_icon_rect.center(),
                    fill_color: Some(stroke_color),
                    outline: None,
                    radius: small_icon_rect.size.x / 2.0,
                });
            }

            if style.debug_rects {
                out_commands.push(debug_rect(rect));
            }
        }
        GuiCmd::Slider {
            interact,
            max,
            min,
            rect,
            value,
        } => {
            let thin_rect = Rect::from_center_size(rect.center(), vec2(rect.size.x, 6.0));
            let marker_center_x = remap_clamp(value, min, max, rect.min().x, rect.max().x);

            let marker_rect = Rect::from_center_size(
                vec2(marker_center_x, thin_rect.center().y),
                vec2(16.0, 16.0),
            );

            out_commands.push(PaintCmd::Rect {
                corner_radius: 2.0,
                fill_color: Some(style.background_fill_color()),
                outline: None,
                pos: thin_rect.pos,
                size: thin_rect.size,
            });

            out_commands.push(PaintCmd::Rect {
                corner_radius: 3.0,
                fill_color: Some(style.interact_fill_color(&interact)),
                outline: None,
                pos: marker_rect.pos,
                size: marker_rect.size,
            });

            if style.debug_rects {
                out_commands.push(debug_rect(rect));
            }
        }
        GuiCmd::Text {
            pos,
            text,
            style: text_style,
        } => {
            let fill_color = match text_style {
                TextStyle::Label => style.text_color(),
            };
            out_commands.push(PaintCmd::Text {
                fill_color,
                font_name: style.font_name.clone(),
                font_size: style.font_size,
                pos,
                text,
            });
        }
        GuiCmd::Window { rect } => {
            out_commands.push(PaintCmd::Rect {
                corner_radius: 5.0,
                fill_color: Some(style.background_fill_color()),
                outline: Some(Outline {
                    color: srgba(255, 255, 255, 255), // TODO
                    width: 1.0,
                }),
                pos: rect.pos,
                size: rect.size,
            });
        }
    }
}

pub fn into_paint_commands<'a, GuiCmdIterator>(
    gui_commands: GuiCmdIterator,
    style: &Style,
) -> Vec<PaintCmd>
where
    GuiCmdIterator: Iterator<Item = &'a GuiCmd>,
{
    let mut paint_commands = vec![];
    for gui_cmd in gui_commands {
        translate_cmd(&mut paint_commands, style, gui_cmd.clone())
    }
    paint_commands
}
