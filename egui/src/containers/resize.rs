#![allow(unused_variables)] // TODO

use crate::*;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "with_serde", derive(serde::Deserialize, serde::Serialize))]
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

#[derive(Clone, Copy, Debug)]
pub struct Resize {
    id: Option<Id>,

    /// If false, we are no enabled
    resizable: bool,

    // Will still try to stay within parent ui bounds
    min_size: Vec2,
    max_size: Vec2,

    default_size: Vec2,

    outline: bool,
    handle_offset: Vec2,
}

impl Default for Resize {
    fn default() -> Self {
        Self {
            id: None,
            resizable: true,
            min_size: Vec2::splat(16.0),
            max_size: Vec2::infinity(),
            default_size: vec2(200.0, 400.0), // TODO: default height for a resizable area (e.g. a window)
            outline: true,
            handle_offset: Default::default(),
        }
    }
}

impl Resize {
    /// Assign an explicit and globablly unique id.
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    pub fn default_width(mut self, width: f32) -> Self {
        self.default_size.x = width;
        self
    }

    pub fn default_height(mut self, height: f32) -> Self {
        self.default_size.y = height;
        self
    }

    pub fn default_size(mut self, default_size: impl Into<Vec2>) -> Self {
        self.default_size = default_size.into();
        self
    }

    pub fn min_size(mut self, min_size: impl Into<Vec2>) -> Self {
        self.min_size = min_size.into();
        self
    }

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
    pub fn auto_sized(self) -> Self {
        self.default_size(Vec2::splat(f32::INFINITY))
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

    pub fn as_wide_as_possible(mut self) -> Self {
        self.min_size.x = f32::INFINITY;
        self
    }

    /// Offset the position of the resize handle by this much
    pub fn handle_offset(mut self, handle_offset: impl Into<Vec2>) -> Self {
        self.handle_offset = handle_offset.into();
        self
    }

    pub fn outline(mut self, outline: bool) -> Self {
        self.outline = outline;
        self
    }
}

struct Prepared {
    id: Id,
    state: State,
    corner_interact: Option<InteractInfo>,
    content_ui: Ui,
}

impl Resize {
    fn begin(&mut self, ui: &mut Ui) -> Prepared {
        let id = self.id.unwrap_or_else(|| ui.make_child_id("resize"));
        self.min_size = self.min_size.min(ui.available().size());
        self.max_size = self.max_size.min(ui.available().size());
        self.max_size = self.max_size.max(self.min_size);

        let mut state = ui.memory().resize.get(&id).cloned().unwrap_or_else(|| {
            let default_size = self.default_size.clamp(self.min_size..=self.max_size);

            State {
                desired_size: default_size,
                last_content_size: vec2(0.0, 0.0),
                requested_size: None,
            }
        });

        state.desired_size = state.desired_size.clamp(self.min_size..=self.max_size);

        let position = ui.available().min;

        let corner_interact = if self.resizable {
            // Resize-corner:
            let corner_size = Vec2::splat(16.0); // TODO: style
            let corner_rect = Rect::from_min_size(
                position + state.desired_size + self.handle_offset - corner_size,
                corner_size,
            );
            let corner_interact = ui.interact(corner_rect, id.with("corner"), Sense::drag());

            if corner_interact.active {
                if let Some(mouse_pos) = ui.input().mouse.pos {
                    // This is the desired size. We may not be able to achieve it.

                    state.desired_size = mouse_pos - position + 0.5 * corner_interact.rect.size()
                        - self.handle_offset;
                    // We don't clamp to max size, because we want to be able to push against outer bounds.
                    // For instance, if we are inside a bigger Resize region, we want to expand that.
                    // state.desired_size = state.desired_size.clamp(self.min_size..=self.max_size);
                    state.desired_size = state.desired_size.max(self.min_size);
                }
            }
            Some(corner_interact)
        } else {
            None
        };

        if let Some(requested_size) = state.requested_size.take() {
            state.desired_size = requested_size;
            // We don't clamp to max size, because we want to be able to push against outer bounds.
            // For instance, if we are inside a bigger Resize region, we want to expand that.
            // state.desired_size = state.desired_size.clamp(self.min_size..=self.max_size);
            state.desired_size = state.desired_size.max(self.min_size);
        }

        // ------------------------------

        let inner_rect = Rect::from_min_size(position, state.desired_size);

        let mut content_clip_rect = inner_rect.expand(ui.style().clip_rect_margin);

        // If we pull the resize handle to shrink, we want to TRY to shink it.
        // After laying out the contents, we might be much bigger.
        // In those cases we don't want the clip_rect to be smaller, because
        // then we will clip the contents of the region even thought the result gets larger. This is simply ugly!
        // So we use the memory of last_content_size to make the clip rect large enough.
        content_clip_rect.max = content_clip_rect.max.max(
            inner_rect.min + state.last_content_size + Vec2::splat(ui.style().clip_rect_margin),
        );

        content_clip_rect = content_clip_rect.intersect(ui.clip_rect()); // Respect parent region

        let mut content_ui = ui.child_ui(inner_rect);
        content_ui.set_clip_rect(content_clip_rect);

        Prepared {
            id,
            state,
            corner_interact,
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
            corner_interact,
            content_ui,
        } = prepared;

        state.last_content_size = content_ui.bounding_size();
        state.last_content_size = state.last_content_size.ceil(); // Avoid rounding errors in math

        // ------------------------------

        if self.outline || self.resizable {
            // We show how large we are,
            // so we must follow the contents:

            state.desired_size = state.desired_size.max(state.last_content_size);
            state.desired_size = ui.round_vec_to_pixels(state.desired_size);

            // We are as large as we look
            ui.allocate_space(state.desired_size);
        } else {
            // Probably a window.
            ui.allocate_space(state.last_content_size);
        }

        // ------------------------------

        if self.outline && corner_interact.is_some() {
            let rect = Rect::from_min_size(content_ui.top_left(), state.desired_size);
            let rect = rect.expand(2.0); // breathing room for content
            ui.add_paint_cmd(paint::PaintCmd::Rect {
                rect,
                corner_radius: 3.0,
                fill: None,
                outline: Some(ui.style().thin_outline),
            });
        }

        if let Some(corner_interact) = corner_interact {
            paint_resize_corner(ui, &corner_interact);

            if corner_interact.hovered || corner_interact.active {
                ui.ctx().output().cursor_icon = CursorIcon::ResizeNwSe;
            }
        }

        ui.memory().resize.insert(id, state);
    }
}

fn paint_resize_corner(ui: &mut Ui, interact: &InteractInfo) {
    let color = ui.style().interact(interact).stroke_color;
    let width = ui.style().interact(interact).stroke_width;

    let corner = ui.round_pos_to_pixels(interact.rect.right_bottom());
    let mut w = 2.0;

    while w < 12.0 {
        ui.add_paint_cmd(paint::PaintCmd::line_segment(
            [pos2(corner.x - w, corner.y), pos2(corner.x, corner.y - w)],
            color,
            width,
        ));
        w += 4.0;
    }
}
