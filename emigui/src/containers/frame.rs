//! Frame container

use crate::{paint::*, *};

#[derive(Clone, Debug, Default)]
pub struct Frame {
    // On each side
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
            fill_color: Some(style.background_fill_color),
            outline: Some(Outline::new(1.0, color::WHITE)),
        }
    }

    pub fn menu_bar(_style: &Style) -> Self {
        Self {
            margin: Vec2::splat(1.0),
            corner_radius: 0.0,
            fill_color: None,
            outline: Some(Outline::new(0.5, color::white(128))),
        }
    }

    pub fn menu(style: &Style) -> Self {
        Self {
            margin: Vec2::splat(1.0),
            corner_radius: 2.0,
            fill_color: Some(style.background_fill_color),
            outline: Some(Outline::new(1.0, color::white(128))),
        }
    }

    pub fn popup(style: &Style) -> Self {
        Self {
            margin: style.window_padding,
            corner_radius: 5.0,
            fill_color: Some(style.background_fill_color),
            outline: Some(Outline::new(1.0, color::white(128))),
        }
    }

    pub fn fill_color(mut self, fill_color: Option<Color>) -> Self {
        self.fill_color = fill_color;
        self
    }

    pub fn outline(mut self, outline: Option<Outline>) -> Self {
        self.outline = outline;
        self
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

        let outer_rect = ui.available();
        let inner_rect = outer_rect.shrink2(margin);
        let where_to_put_background = ui.paint_list_len();

        let mut child_ui = ui.child_ui(inner_rect);
        add_contents(&mut child_ui);

        let outer_rect = Rect::from_min_max(outer_rect.min, child_ui.child_bounds().max + margin);

        ui.insert_paint_cmd(
            where_to_put_background,
            PaintCmd::Rect {
                corner_radius,
                fill_color,
                outline,
                rect: outer_rect,
            },
        );

        ui.expand_to_include_child(outer_rect);
        // TODO: move cursor in parent ui
    }
}
