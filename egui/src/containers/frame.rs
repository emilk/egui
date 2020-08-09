//! Frame container

use crate::{layers::PaintCmdIdx, paint::*, *};

/// Adds a rectangular frame and background to some `Ui`.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    // On each side
    pub margin: Vec2,
    pub corner_radius: f32,
    pub fill: Option<Color>,
    pub outline: Option<LineStyle>,
}

impl Frame {
    pub fn window(style: &Style) -> Self {
        Self {
            margin: style.window_padding,
            corner_radius: style.window.corner_radius,
            fill: Some(style.background_fill),
            outline: style.interact.inactive.rect_outline, // becauce we can resize windows
        }
    }

    pub fn menu_bar(_style: &Style) -> Self {
        Self {
            margin: Vec2::splat(1.0),
            corner_radius: 0.0,
            fill: None,
            outline: Some(LineStyle::new(0.5, color::white(128))),
        }
    }

    pub fn menu(style: &Style) -> Self {
        Self {
            margin: Vec2::splat(1.0),
            corner_radius: 2.0,
            fill: Some(style.background_fill),
            outline: Some(LineStyle::new(1.0, color::white(128))),
        }
    }

    pub fn popup(style: &Style) -> Self {
        Self {
            margin: style.window_padding,
            corner_radius: 5.0,
            fill: Some(style.background_fill),
            outline: Some(LineStyle::new(1.0, color::white(128))),
        }
    }

    pub fn fill(mut self, fill: Option<Color>) -> Self {
        self.fill = fill;
        self
    }

    pub fn outline(mut self, outline: Option<LineStyle>) -> Self {
        self.outline = outline;
        self
    }
}

pub struct Prepared {
    pub frame: Frame,
    outer_rect_bounds: Rect,
    where_to_put_background: PaintCmdIdx,
    pub content_ui: Ui,
}

impl Frame {
    pub fn begin(self, ui: &mut Ui) -> Prepared {
        let outer_rect_bounds = ui.available();
        let inner_rect = outer_rect_bounds.shrink2(self.margin);
        let where_to_put_background = ui.painter().add(PaintCmd::Noop);
        let content_ui = ui.child_ui(inner_rect);
        Prepared {
            frame: self,
            outer_rect_bounds,
            where_to_put_background,
            content_ui,
        }
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        let mut prepared = self.begin(ui);
        let ret = add_contents(&mut prepared.content_ui);
        prepared.end(ui);
        ret
    }
}

impl Prepared {
    pub fn outer_rect(&self) -> Rect {
        Rect::from_min_max(
            self.outer_rect_bounds.min,
            self.content_ui.child_bounds().max + self.frame.margin,
        )
    }

    pub fn end(self, ui: &mut Ui) -> Rect {
        let outer_rect = self.outer_rect();

        let Prepared {
            frame,
            where_to_put_background,
            ..
        } = self;

        ui.painter().set(
            where_to_put_background,
            PaintCmd::Rect {
                corner_radius: frame.corner_radius,
                fill: frame.fill,
                outline: frame.outline,
                rect: outer_rect,
            },
        );

        ui.expand_to_include_child(outer_rect);
        // TODO: move cursor in parent ui

        outer_rect
    }
}
