#![allow(unused_variables)] // TODO
use crate::*;

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub size: Vec2,
}

// TODO: auto-shink/grow should be part of another container!
#[derive(Clone, Copy, Debug)]
pub struct Resize {
    /// If false, we are no enabled
    resizable: bool,

    // Will still try to stay within parent region bounds
    min_size: Vec2,
    max_size: Vec2,

    default_size: Vec2,

    // If true, won't allow you to make window so big that it creates spacing
    auto_shrink_width: bool,
    auto_shrink_height: bool,

    // If true, won't allow you to resize smaller than that everything fits.
    expand_width_to_fit_content: bool,
    expand_height_to_fit_content: bool,

    handle_offset: Vec2,
}

impl Default for Resize {
    fn default() -> Self {
        Self {
            resizable: true,
            min_size: Vec2::splat(32.0),
            max_size: Vec2::infinity(),
            default_size: vec2(f32::INFINITY, 200.0), // TODO
            auto_shrink_width: false,
            auto_shrink_height: false,
            expand_width_to_fit_content: true,
            expand_height_to_fit_content: true,
            handle_offset: Default::default(),
        }
    }
}

impl Resize {
    pub fn default_height(mut self, height: f32) -> Self {
        self.default_size.y = height;
        self
    }

    pub fn default_size(mut self, default_size: Vec2) -> Self {
        self.default_size = default_size;
        self
    }

    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    pub fn max_size(mut self, max_size: Vec2) -> Self {
        self.max_size = max_size;
        self
    }

    /// Can you resize it with the mouse?
    /// Note that a window can still auto-resize
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn fixed_size(mut self, size: Vec2) -> Self {
        self.auto_shrink_width = false;
        self.auto_shrink_height = false;
        self.expand_width_to_fit_content = false;
        self.expand_height_to_fit_content = false;
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

    /// true: prevent from resizing to smaller than contents.
    /// false: allow shrinking to smaller than contents.
    pub fn auto_expand(mut self, auto_expand: bool) -> Self {
        self.expand_width_to_fit_content = auto_expand;
        self.expand_height_to_fit_content = auto_expand;
        self
    }

    /// true: prevent from resizing to smaller than contents.
    /// false: allow shrinking to smaller than contents.
    pub fn auto_expand_width(mut self, auto_expand: bool) -> Self {
        self.expand_width_to_fit_content = auto_expand;
        self
    }

    /// true: prevent from resizing to smaller than contents.
    /// false: allow shrinking to smaller than contents.
    pub fn auto_expand_height(mut self, auto_expand: bool) -> Self {
        self.expand_height_to_fit_content = auto_expand;
        self
    }

    /// Offset the position of the resize handle by this much
    pub fn handle_offset(mut self, handle_offset: Vec2) -> Self {
        self.handle_offset = handle_offset;
        self
    }

    pub fn auto_shrink_width(mut self, auto_shrink_width: bool) -> Self {
        self.auto_shrink_width = auto_shrink_width;
        self
    }

    pub fn auto_shrink_height(mut self, auto_shrink_height: bool) -> Self {
        self.auto_shrink_height = auto_shrink_height;
        self
    }
}

// TODO: a common trait for Things that follow this pattern
impl Resize {
    pub fn show(mut self, region: &mut Region, add_contents: impl FnOnce(&mut Region)) {
        if !self.resizable {
            return add_contents(region);
        }

        let id = region.make_child_id("scroll");
        self.min_size = self.min_size.min(region.available_space());
        self.max_size = self.max_size.min(region.available_space());
        self.max_size = self.max_size.max(self.min_size);

        let (is_new, mut state) = match region.memory().resize.get(&id) {
            Some(state) => (false, state.clone()),
            None => {
                let default_size = self.default_size.clamp(self.min_size..=self.max_size);
                (true, State { size: default_size })
            }
        };

        state.size = state.size.clamp(self.min_size..=self.max_size);
        let last_frame_size = state.size;

        let position = region.cursor();

        // Resize-corner:
        let corner_size = Vec2::splat(16.0); // TODO: style
        let corner_rect = Rect::from_min_size(
            position + state.size + self.handle_offset - corner_size,
            corner_size,
        );
        let corner_interact = region.interact_rect(&corner_rect, id.with("corner"));

        if corner_interact.active {
            if let Some(mouse_pos) = region.input().mouse_pos {
                // This is the desired size. We may not be able to achieve it.

                state.size = mouse_pos - position + 0.5 * corner_interact.rect.size();
                // We don't clamp to max size, because we want to be able to push against outer bounds.
                // For instance, if we are inside a bigger Resize region, we want to expand that.
                // state.size = state.size.clamp(self.min_size..=self.max_size);
                state.size = state.size.max(self.min_size);
            }
        }

        // ------------------------------

        let inner_rect = Rect::from_min_size(region.cursor(), state.size);
        let desired_size = {
            let mut contents_region = region.child_region(inner_rect);
            contents_region.clip_rect = region
                .clip_rect()
                .intersect(&inner_rect.expand(region.style().clip_rect_margin));

            // region.debug_text_at(
            //     inner_rect.min + last_frame_size,
            //     &format!("last_frame_size: {:?}", last_frame_size),
            // );

            // If we pull the resize handle to shrink, we want to TRY to shink it.
            // After laying out the contents, we might be much bigger.
            // In those cases we don't want the clip_rect to be smaller, because
            // then we will clip the contents of the region even thought the result gets larger. This is simply ugly!
            contents_region.clip_rect.max = contents_region
                .clip_rect
                .max
                .max(contents_region.clip_rect.min + last_frame_size)
                .min(region.clip_rect.max); // Respect parent region

            add_contents(&mut contents_region);
            contents_region.bounding_size()
        };
        let desired_size = desired_size.ceil(); // Avoid rounding errors in math

        // ------------------------------

        if self.auto_shrink_width {
            state.size.x = state.size.x.min(desired_size.x);
        }
        if self.auto_shrink_height {
            state.size.y = state.size.y.min(desired_size.y);
        }
        if self.expand_width_to_fit_content || is_new {
            state.size.x = state.size.x.max(desired_size.x);
        }
        if self.expand_height_to_fit_content || is_new {
            state.size.y = state.size.y.max(desired_size.y);
        }

        state.size = state.size.max(self.min_size);
        // state.size = state.size.clamp(self.min_size..=self.max_size);
        state.size = state.size.round(); // TODO: round to pixels

        region.reserve_space(state.size, None);

        // ------------------------------

        paint_resize_corner(region, &corner_rect, &corner_interact);

        if corner_interact.hovered || corner_interact.active {
            region.ctx().output.lock().cursor_icon = CursorIcon::ResizeNwSe;
        }

        region.memory().resize.insert(id, state);
    }
}

fn paint_resize_corner(region: &mut Region, rect: &Rect, interact: &InteractInfo) {
    let color = region.style().interact_stroke_color(&interact);
    let width = region.style().interact_stroke_width(&interact);

    let corner = rect.right_bottom().round(); // TODO: round to pixels
    let mut w = 2.0;

    while w < 12.0 {
        region.add_paint_cmd(PaintCmd::line_segment(
            (pos2(corner.x - w, corner.y), pos2(corner.x, corner.y - w)),
            color,
            width,
        ));
        w += 4.0;
    }
}
