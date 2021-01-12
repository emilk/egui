//! Frame container

use crate::{layers::ShapeIdx, paint::*, *};

/// Adds a rectangular frame and background to some [`Ui`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Frame {
    // On each side
    pub margin: Vec2,
    pub corner_radius: f32,
    pub shadow: Shadow,
    pub fill: Color32,
    pub stroke: Stroke,
}

impl Frame {
    pub fn none() -> Self {
        Self::default()
    }

    pub(crate) fn panel(style: &Style) -> Self {
        Self {
            margin: Vec2::new(8.0, 2.0),
            corner_radius: 0.0,
            fill: style.visuals.widgets.noninteractive.bg_fill,
            stroke: style.visuals.widgets.noninteractive.bg_stroke,
            ..Default::default()
        }
    }

    pub fn central_panel(style: &Style) -> Self {
        Self {
            margin: Vec2::new(8.0, 8.0),
            corner_radius: 0.0,
            fill: style.visuals.widgets.noninteractive.bg_fill,
            stroke: Default::default(),
            ..Default::default()
        }
    }

    pub fn window(style: &Style) -> Self {
        Self {
            margin: style.spacing.window_padding,
            corner_radius: style.visuals.window_corner_radius,
            shadow: style.visuals.window_shadow,
            fill: style.visuals.widgets.noninteractive.bg_fill,
            stroke: style.visuals.widgets.inactive.bg_stroke, // because we can resize windows
        }
    }

    pub fn menu(style: &Style) -> Self {
        Self {
            margin: Vec2::splat(1.0),
            corner_radius: 2.0,
            shadow: Shadow::small(),
            fill: style.visuals.widgets.noninteractive.bg_fill,
            stroke: style.visuals.widgets.noninteractive.bg_stroke,
        }
    }

    pub fn popup(style: &Style) -> Self {
        Self {
            margin: style.spacing.window_padding,
            corner_radius: 5.0,
            shadow: Shadow::small(),
            fill: style.visuals.widgets.noninteractive.bg_fill,
            stroke: style.visuals.widgets.noninteractive.bg_stroke,
        }
    }

    /// dark canvas to draw on
    pub fn dark_canvas(style: &Style) -> Self {
        Self {
            margin: Vec2::new(10.0, 10.0),
            corner_radius: 5.0,
            fill: Color32::from_black_alpha(250),
            stroke: style.visuals.widgets.noninteractive.bg_stroke,
            ..Default::default()
        }
    }
}

impl Frame {
    pub fn fill(mut self, fill: Color32) -> Self {
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
    where_to_put_background: ShapeIdx,
    pub content_ui: Ui,
}

impl Frame {
    pub fn begin(self, ui: &mut Ui) -> Prepared {
        let where_to_put_background = ui.painter().add(Shape::Noop);
        let outer_rect_bounds = ui.available_rect_before_wrap();
        let inner_rect = outer_rect_bounds.shrink2(self.margin);
        let content_ui = ui.child_ui(inner_rect, *ui.layout());

        // content_ui.set_clip_rect(outer_rect_bounds.shrink(self.stroke.width * 0.5)); // Can't do this since we don't know final size yet

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
            self.content_ui.min_rect().max + self.frame.margin,
        )
    }

    pub fn end(self, ui: &mut Ui) -> Rect {
        let outer_rect = self.outer_rect();

        let Prepared {
            frame,
            where_to_put_background,
            ..
        } = self;

        let frame_shape = Shape::Rect {
            rect: outer_rect,
            corner_radius: frame.corner_radius,
            fill: frame.fill,
            stroke: frame.stroke,
        };

        if frame.shadow == Default::default() {
            ui.painter().set(where_to_put_background, frame_shape);
        } else {
            let shadow = frame.shadow.tessellate(outer_rect, frame.corner_radius);
            let shadow = Shape::Triangles(shadow);
            ui.painter().set(
                where_to_put_background,
                Shape::Vec(vec![shadow, frame_shape]),
            )
        };

        ui.advance_cursor_after_rect(outer_rect);

        outer_rect
    }
}
