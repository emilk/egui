use crate::*;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct State {
    /// This is the size that the user has picked by dragging the resize handles.
    /// This may be smaller and/or larger than the actual size.
    /// For instance, the user may have tried to shrink too much (not fitting the contents).
    /// Or the user requested a large area, but the content don't need that much space.
    pub(crate) desired_size: Vec2,

    /// Actual size of content last frame
    last_content_size: Vec2,

    /// Externally requested size (e.g. by Window) for the next frame
    pub(crate) requested_size: Option<Vec2>,
}

impl State {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }
}

/// A region that can be resized by dragging the bottom right corner.
#[derive(Clone, Copy, Debug)]
#[must_use = "You should call .show()"]
pub struct Resize {
    id: Option<Id>,
    id_source: Option<Id>,

    /// If false, we are no enabled
    resizable: bool,

    pub(crate) min_size: Vec2,
    pub(crate) max_size: Vec2,

    default_size: Vec2,

    with_stroke: bool,
}

impl Default for Resize {
    fn default() -> Self {
        Self {
            id: None,
            id_source: None,
            resizable: true,
            min_size: Vec2::splat(16.0),
            max_size: Vec2::splat(f32::INFINITY),
            default_size: vec2(320.0, 128.0), // TODO(emilk): preferred size of [`Resize`] area.
            with_stroke: true,
        }
    }
}

impl Resize {
    /// Assign an explicit and globally unique id.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// A source for the unique [`Id`], e.g. `.id_source("second_resize_area")` or `.id_source(loop_index)`.
    pub fn id_source(mut self, id_source: impl std::hash::Hash) -> Self {
        self.id_source = Some(Id::new(id_source));
        self
    }

    /// Preferred / suggested width. Actual width will depend on contents.
    ///
    /// Examples:
    /// * if the contents is text, this will decide where we break long lines.
    /// * if the contents is a canvas, this decides the width of it,
    /// * if the contents is some buttons, this is ignored and we will auto-size.
    pub fn default_width(mut self, width: f32) -> Self {
        self.default_size.x = width;
        self
    }

    /// Preferred / suggested height. Actual height will depend on contents.
    ///
    /// Examples:
    /// * if the contents is a [`ScrollArea`] then this decides the maximum size.
    /// * if the contents is a canvas, this decides the height of it,
    /// * if the contents is text and buttons, then the `default_height` is ignored
    ///   and the height is picked automatically..
    pub fn default_height(mut self, height: f32) -> Self {
        self.default_size.y = height;
        self
    }

    pub fn default_size(mut self, default_size: impl Into<Vec2>) -> Self {
        self.default_size = default_size.into();
        self
    }

    /// Won't shrink to smaller than this
    pub fn min_size(mut self, min_size: impl Into<Vec2>) -> Self {
        self.min_size = min_size.into();
        self
    }

    /// Won't shrink to smaller than this
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_size.x = min_width;
        self
    }

    /// Won't shrink to smaller than this
    pub fn min_height(mut self, min_height: f32) -> Self {
        self.min_size.y = min_height;
        self
    }

    /// Won't expand to larger than this
    pub fn max_size(mut self, max_size: impl Into<Vec2>) -> Self {
        self.max_size = max_size.into();
        self
    }

    /// Can you resize it with the mouse?
    /// Note that a window can still auto-resize
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn is_resizable(&self) -> bool {
        self.resizable
    }

    /// Not manually resizable, just takes the size of its contents.
    /// Text will not wrap, but will instead make your window width expand.
    pub fn auto_sized(self) -> Self {
        self.min_size(Vec2::ZERO)
            .default_size(Vec2::splat(f32::INFINITY))
            .resizable(false)
    }

    pub fn fixed_size(mut self, size: impl Into<Vec2>) -> Self {
        let size = size.into();
        self.default_size = size;
        self.min_size = size;
        self.max_size = size;
        self.resizable = false;
        self
    }

    pub fn with_stroke(mut self, with_stroke: bool) -> Self {
        self.with_stroke = with_stroke;
        self
    }
}

struct Prepared {
    id: Id,
    state: State,
    corner_response: Option<Response>,
    content_ui: Ui,
}

impl Resize {
    fn begin(&mut self, ui: &mut Ui) -> Prepared {
        let position = ui.available_rect_before_wrap().min;
        let id = self.id.unwrap_or_else(|| {
            let id_source = self.id_source.unwrap_or_else(|| Id::new("resize"));
            ui.make_persistent_id(id_source)
        });

        let mut state = State::load(ui.ctx(), id).unwrap_or_else(|| {
            ui.ctx().request_repaint(); // counter frame delay

            let default_size = self
                .default_size
                .at_least(self.min_size)
                .at_most(self.max_size)
                .at_most(
                    ui.ctx().screen_rect().size() - ui.spacing().window_margin.sum(), // hack for windows
                );

            State {
                desired_size: default_size,
                last_content_size: vec2(0.0, 0.0),
                requested_size: None,
            }
        });

        state.desired_size = state
            .desired_size
            .at_least(self.min_size)
            .at_most(self.max_size);

        let mut user_requested_size = state.requested_size.take();

        let corner_response = if self.resizable {
            // Resize-corner:
            let corner_size = Vec2::splat(ui.visuals().resize_corner_size);
            let corner_rect =
                Rect::from_min_size(position + state.desired_size - corner_size, corner_size);
            let corner_response = ui.interact(corner_rect, id.with("corner"), Sense::drag());

            if let Some(pointer_pos) = corner_response.interact_pointer_pos() {
                user_requested_size =
                    Some(pointer_pos - position + 0.5 * corner_response.rect.size());
            }

            Some(corner_response)
        } else {
            None
        };

        if let Some(user_requested_size) = user_requested_size {
            state.desired_size = user_requested_size;
        } else {
            // We are not being actively resized, so auto-expand to include size of last frame.
            // This prevents auto-shrinking if the contents contain width-filling widgets (separators etc)
            // but it makes a lot of interactions with [`Window`]s nicer.
            state.desired_size = state.desired_size.max(state.last_content_size);
        }

        state.desired_size = state
            .desired_size
            .at_least(self.min_size)
            .at_most(self.max_size);

        // ------------------------------

        let inner_rect = Rect::from_min_size(position, state.desired_size);

        let mut content_clip_rect = inner_rect.expand(ui.visuals().clip_rect_margin);

        // If we pull the resize handle to shrink, we want to TRY to shrink it.
        // After laying out the contents, we might be much bigger.
        // In those cases we don't want the clip_rect to be smaller, because
        // then we will clip the contents of the region even thought the result gets larger. This is simply ugly!
        // So we use the memory of last_content_size to make the clip rect large enough.
        content_clip_rect.max = content_clip_rect.max.max(
            inner_rect.min + state.last_content_size + Vec2::splat(ui.visuals().clip_rect_margin),
        );

        content_clip_rect = content_clip_rect.intersect(ui.clip_rect()); // Respect parent region

        let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
        content_ui.set_clip_rect(content_clip_rect);

        Prepared {
            id,
            state,
            corner_response,
            content_ui,
        }
    }

    pub fn show<R>(mut self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
        let mut prepared = self.begin(ui);
        let ret = add_contents(&mut prepared.content_ui);
        self.end(ui, prepared);
        ret
    }

    fn end(self, ui: &mut Ui, prepared: Prepared) {
        let Prepared {
            id,
            mut state,
            corner_response,
            content_ui,
        } = prepared;

        state.last_content_size = content_ui.min_size();

        // ------------------------------

        let size = if self.with_stroke || self.resizable {
            // We show how large we are,
            // so we must follow the contents:

            state.desired_size = state.desired_size.max(state.last_content_size);

            // We are as large as we look
            state.desired_size
        } else {
            // Probably a window.
            state.last_content_size
        };
        ui.advance_cursor_after_rect(Rect::from_min_size(content_ui.min_rect().min, size));

        // ------------------------------

        if self.with_stroke && corner_response.is_some() {
            let rect = Rect::from_min_size(content_ui.min_rect().left_top(), state.desired_size);
            let rect = rect.expand(2.0); // breathing room for content
            ui.painter().add(Shape::rect_stroke(
                rect,
                3.0,
                ui.visuals().widgets.noninteractive.bg_stroke,
            ));
        }

        if let Some(corner_response) = corner_response {
            paint_resize_corner(ui, &corner_response);

            if corner_response.hovered() || corner_response.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeNwSe);
            }
        }

        state.store(ui.ctx(), id);

        if ui.ctx().style().debug.show_resize {
            ui.ctx().debug_painter().debug_rect(
                Rect::from_min_size(content_ui.min_rect().left_top(), state.desired_size),
                Color32::GREEN,
                "desired_size",
            );
            ui.ctx().debug_painter().debug_rect(
                Rect::from_min_size(content_ui.min_rect().left_top(), state.last_content_size),
                Color32::LIGHT_BLUE,
                "last_content_size",
            );
        }
    }
}

use epaint::Stroke;

pub fn paint_resize_corner(ui: &mut Ui, response: &Response) {
    let stroke = ui.style().interact(response).fg_stroke;
    paint_resize_corner_with_style(ui, &response.rect, stroke, Align2::RIGHT_BOTTOM);
}

pub fn paint_resize_corner_with_style(ui: &mut Ui, rect: &Rect, stroke: Stroke, corner: Align2) {
    let painter = ui.painter();
    let cp = painter.round_pos_to_pixels(corner.pos_in_rect(rect));
    let mut w = 2.0;

    while w <= rect.width() && w <= rect.height() {
        painter.line_segment(
            [
                pos2(cp.x - w * corner.x().to_sign(), cp.y),
                pos2(cp.x, cp.y - w * corner.y().to_sign()),
            ],
            stroke,
        );
        w += 4.0;
    }
}
