#![allow(unused_variables)] // TODO
use crate::*;

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub size: Vec2,
}

#[derive(Clone, Copy, Debug)]
pub struct Resize {
    // Will still try to stay within parent region bounds
    min_size: Vec2,
    max_size: Vec2,

    default_size: Vec2,

    // If true, won't allow you to make window so big that it creates spacing
    shrink_width_to_fit_content: bool,
    shrink_height_to_fit_content: bool,

    // If true, won't allow you to resize smaller than that everything fits.
    expand_width_to_fit_content: bool,
    expand_height_to_fit_content: bool,
}

impl Default for Resize {
    fn default() -> Self {
        Self {
            min_size: Vec2::splat(32.0),
            max_size: Vec2::infinity(),
            default_size: vec2(f32::INFINITY, 200.0), // TODO
            shrink_width_to_fit_content: false,
            shrink_height_to_fit_content: false,
            expand_width_to_fit_content: true,
            expand_height_to_fit_content: true,
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
}

// TODO: a common trait for Things that follow this pattern
impl Resize {
    pub fn show(mut self, region: &mut Region, add_contents: impl FnOnce(&mut Region)) {
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

        let position = region.cursor();

        // Resize-corner:
        let corner_size = Vec2::splat(16.0); // TODO: style
        let corner_rect = Rect::from_min_size(position + state.size - corner_size, corner_size);
        let corner_interact = region.interact_rect(&corner_rect, id.with("corner"));

        if corner_interact.active {
            if let Some(mouse_pos) = region.input().mouse_pos {
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
            add_contents(&mut contents_region);
            contents_region.bounding_size()
        };
        let desired_size = desired_size.ceil(); // Avoid rounding errors in math

        // ------------------------------

        if self.shrink_width_to_fit_content {
            state.size.x = state.size.x.min(desired_size.x);
        }
        if self.shrink_height_to_fit_content {
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

        region.reserve_space_without_padding(state.size);

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
