use crate::{math::*, types::*};

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Style {
    /// Horizontal and vertical padding within a window frame.
    pub window_padding: Vec2,

    /// Button size is text size plus this on each side
    pub button_padding: Vec2,

    /// Horizontal and vertical spacing between widgets
    pub item_spacing: Vec2,

    /// Indent foldable regions etc by this much.
    pub indent: f32,

    /// Anything clickable is (at least) this wide.
    pub clickable_diameter: f32,

    /// Checkboxes, radio button and foldables have an icon at the start.
    /// The text starts after this many pixels.
    pub start_icon_width: f32,

    // -----------------------------------------------
    // Purely visual:
    /// For stuff like check marks in check boxes.
    pub line_width: f32,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            window_padding: vec2(6.0, 6.0),
            button_padding: vec2(5.0, 3.0),
            item_spacing: vec2(8.0, 4.0),
            indent: 21.0,
            clickable_diameter: 34.0,
            start_icon_width: 20.0,
            line_width: 2.0,
        }
    }
}

impl Style {
    /// e.g. the background of the slider
    fn background_fill_color(&self) -> Color {
        gray(34, 200)
    }

    fn text_color(&self) -> Color {
        gray(255, 200)
    }

    /// Fill color of the interactive part of a component (button, slider grab, checkbox, ...)
    fn interact_fill_color(&self, interact: &InteractInfo) -> Color {
        if interact.active {
            srgba(100, 100, 200, 255)
        } else if interact.hovered {
            srgba(100, 100, 150, 255)
        } else {
            srgba(60, 60, 70, 255)
        }
    }

    /// Stroke and text color of the interactive part of a component (button, slider grab, checkbox, ...)
    fn interact_stroke_color(&self, interact: &InteractInfo) -> Color {
        if interact.active {
            gray(255, 255)
        } else if interact.hovered {
            gray(255, 200)
        } else {
            gray(255, 170)
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

// ----------------------------------------------------------------------------

fn translate_cmd(out_commands: &mut Vec<PaintCmd>, style: &Style, cmd: GuiCmd) {
    match cmd {
        GuiCmd::PaintCommands(mut commands) => out_commands.append(&mut commands),
        GuiCmd::Button { interact } => {
            out_commands.push(PaintCmd::Rect {
                corner_radius: 10.0,
                fill_color: Some(style.interact_fill_color(&interact)),
                outline: None,
                rect: interact.rect,
            });
        }
        GuiCmd::Checkbox { checked, interact } => {
            let (small_icon_rect, big_icon_rect) = style.icon_rectangles(&interact.rect);
            out_commands.push(PaintCmd::Rect {
                corner_radius: 3.0,
                fill_color: Some(style.interact_fill_color(&interact)),
                outline: None,
                rect: big_icon_rect,
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
        }
        GuiCmd::FoldableHeader { interact, open } => {
            let fill_color = style.interact_fill_color(&interact);
            let stroke_color = style.interact_stroke_color(&interact);

            out_commands.push(PaintCmd::Rect {
                corner_radius: 3.0,
                fill_color: Some(fill_color),
                outline: None,
                rect: interact.rect,
            });

            let (small_icon_rect, _) = style.icon_rectangles(&interact.rect);
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
        GuiCmd::RadioButton { checked, interact } => {
            let fill_color = style.interact_fill_color(&interact);
            let stroke_color = style.interact_stroke_color(&interact);

            let (small_icon_rect, big_icon_rect) = style.icon_rectangles(&interact.rect);

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
        }
        GuiCmd::Slider {
            interact,
            max,
            min,
            value,
        } => {
            let rect = interact.rect;
            let thickness = rect.size().y;
            let thin_size = vec2(rect.size.x, thickness / 5.0);
            let thin_rect = Rect::from_center_size(rect.center(), thin_size);
            let marker_center_x = remap_clamp(value, min, max, rect.min().x, rect.max().x);

            out_commands.push(PaintCmd::Rect {
                corner_radius: 4.0,
                fill_color: Some(style.background_fill_color()),
                outline: Some(Outline {
                    color: gray(200, 255), // TODO
                    width: 1.0,
                }),
                rect: thin_rect,
            });

            out_commands.push(PaintCmd::Circle {
                center: vec2(marker_center_x, thin_rect.center().y),
                fill_color: Some(style.interact_fill_color(&interact)),
                outline: Some(Outline {
                    color: style.interact_stroke_color(&interact),
                    width: 1.5,
                }),
                radius: thickness / 3.0,
            });
        }
        GuiCmd::Text {
            color,
            pos,
            text,
            text_style,
            x_offsets,
        } => {
            let color = color.unwrap_or_else(|| style.text_color());
            out_commands.push(PaintCmd::Text {
                color,
                text_style,
                pos,
                text,
                x_offsets,
            });
        }
        GuiCmd::Window { rect } => {
            out_commands.push(PaintCmd::Rect {
                corner_radius: 5.0,
                fill_color: Some(style.background_fill_color()),
                outline: Some(Outline {
                    color: gray(255, 255), // TODO
                    width: 1.0,
                }),
                rect,
            });
        }
    }
}

pub fn into_paint_commands<GuiCmdIterator>(
    gui_commands: GuiCmdIterator,
    style: &Style,
) -> Vec<PaintCmd>
where
    GuiCmdIterator: Iterator<Item = GuiCmd>,
{
    let mut paint_commands = vec![];
    for gui_cmd in gui_commands {
        translate_cmd(&mut paint_commands, style, gui_cmd)
    }
    paint_commands
}
