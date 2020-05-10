//! Frame container

use crate::*;

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub margin: Vec2,
    pub corner_radius: f32,
    pub fill_color: Option<Color>,
    pub outline: Option<Outline>,
}

impl Frame {
    pub fn window(style: &Style) -> Self {
        Self {
            margin: style.window_padding,
            corner_radius: style.window.corner_radius,
            fill_color: Some(style.background_fill_color()),
            outline: Some(Outline::new(1.0, color::WHITE)),
        }
    }

    pub fn menu(style: &Style) -> Self {
        Self {
            margin: Vec2::splat(1.0),
            corner_radius: 2.0,
            fill_color: Some(style.background_fill_color()),
            outline: Some(Outline::new(1.0, color::white(128))),
        }
    }
}

impl Frame {
    pub fn show(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) {
        let Frame {
            margin,
            corner_radius,
            fill_color,
            outline,
        } = self;

        let outer_pos = ui.cursor();
        let inner_rect =
            Rect::from_min_size(outer_pos + margin, ui.available_space() - 2.0 * margin);
        let where_to_put_background = ui.paint_list_len();

        let mut child_ui = ui.child_ui(inner_rect);
        add_contents(&mut child_ui);

        let inner_size = child_ui.bounding_size();
        let inner_size = inner_size.ceil(); // TODO: round to pixel

        let outer_rect = Rect::from_min_size(outer_pos, margin + inner_size + margin);

        ui.insert_paint_cmd(
            where_to_put_background,
            PaintCmd::Rect {
                corner_radius,
                fill_color,
                outline,
                rect: outer_rect,
            },
        );

        ui.expand_to_include_child(child_ui.child_bounds().expand2(margin));
        // TODO: move up cursor?
    }
}
