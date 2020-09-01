//! Frame container

use crate::{layers::PaintCmdIdx, paint::*, *};

/// Adds a rectangular frame and background to some `Ui`.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    // On each side
    pub margin: Vec2,
    pub corner_radius: f32,
    pub fill: Srgba,
    pub stroke: Stroke,
}

impl Frame {
    pub fn window(style: &Style) -> Self {
        Self {
            margin: style.spacing.window_padding,
            corner_radius: style.visuals.window_corner_radius,
            fill: style.visuals.background_fill,
            stroke: style.visuals.interacted.inactive.bg_stroke, // because we can resize windows
        }
    }

    pub fn menu_bar(_style: &Style) -> Self {
        Self {
            margin: Vec2::splat(1.0),
            corner_radius: 0.0,
            fill: Default::default(),
            stroke: Stroke::new(0.5, Srgba::gray(128)),
        }
    }

    pub fn menu(style: &Style) -> Self {
        Self {
            margin: Vec2::splat(1.0),
            corner_radius: 2.0,
            fill: style.visuals.background_fill,
            stroke: Stroke::new(1.0, Srgba::gray(128)),
        }
    }

    pub fn popup(style: &Style) -> Self {
        Self {
            margin: style.spacing.window_padding,
            corner_radius: 5.0,
            fill: style.visuals.background_fill,
            stroke: Stroke::new(1.0, Srgba::gray(128)),
        }
    }

    pub fn fill(mut self, fill: Srgba) -> Self {
        self.fill = fill;
        self
    }

    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
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
                stroke: frame.stroke,
                rect: outer_rect,
            },
        );

        ui.expand_to_include_child(outer_rect);
        // TODO: move cursor in parent ui

        outer_rect
    }
}
