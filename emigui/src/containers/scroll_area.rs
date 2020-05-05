use crate::*;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct State {
    /// Positive offset means scrolling down/right
    offset: Vec2,

    show_scroll: bool, // TODO: default value?
}

// TODO: rename VScroll
#[derive(Clone, Debug)]
pub struct ScrollArea {
    max_height: f32,
    always_show_scroll: bool,
    auto_hide_scroll: bool,
}

impl Default for ScrollArea {
    fn default() -> Self {
        Self {
            max_height: 200.0,
            always_show_scroll: false,
            auto_hide_scroll: true,
        }
    }
}

impl ScrollArea {
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }

    pub fn always_show_scroll(mut self, always_show_scroll: bool) -> Self {
        self.always_show_scroll = always_show_scroll;
        self
    }

    pub fn auto_hide_scroll(mut self, auto_hide_scroll: bool) -> Self {
        self.auto_hide_scroll = auto_hide_scroll;
        self
    }
}

impl ScrollArea {
    pub fn show(self, outer_region: &mut Region, add_contents: impl FnOnce(&mut Region)) {
        let ctx = outer_region.ctx().clone();

        let scroll_area_id = outer_region.make_child_id("scroll_area");
        let mut state = ctx
            .memory()
            .scroll_areas
            .get(&scroll_area_id)
            .cloned()
            .unwrap_or_default();

        // content: size of contents (generally large)
        // outer: size of scroll area including scroll bar(s)
        // inner: excluding scroll bar(s). The area we clip the contents to.

        let max_scroll_bar_width = 16.0;

        let current_scroll_bar_width = if state.show_scroll || !self.auto_hide_scroll {
            max_scroll_bar_width // TODO: animate?
        } else {
            0.0
        };

        let outer_size = vec2(
            outer_region.available_width(),
            outer_region.available_height().min(self.max_height),
        );

        let inner_size = outer_size - vec2(current_scroll_bar_width, 0.0);
        let inner_rect = Rect::from_min_size(outer_region.cursor(), inner_size);

        let mut content_region = outer_region.child_region(Rect::from_min_size(
            outer_region.cursor() - state.offset,
            vec2(inner_size.x, f32::INFINITY),
        ));
        let mut content_clip_rect = outer_region.clip_rect().intersect(inner_rect);
        content_clip_rect.max.x = outer_region.clip_rect().max.x - current_scroll_bar_width; // Nice handling of forced resizing beyond the possible
        content_region.set_clip_rect(content_clip_rect);

        add_contents(&mut content_region);
        let content_size = content_region.bounding_size();

        let inner_rect = Rect::from_min_size(
            inner_rect.min,
            vec2(
                inner_rect.width().max(content_size.x), // Expand width to fit content
                inner_rect.height(),
            ),
        );

        let outer_rect = Rect::from_min_size(
            inner_rect.min,
            inner_rect.size() + vec2(current_scroll_bar_width, 0.0),
        );

        let content_interact = outer_region.interact_rect(inner_rect, scroll_area_id.with("area"));
        if content_interact.active {
            // Dragging scroll area to scroll:
            state.offset.y -= ctx.input().mouse_move.y;
        }

        // TODO: check that nothing else is being inteacted with
        if outer_region.contains_mouse(outer_rect) && ctx.memory().active_id.is_none() {
            state.offset.y -= ctx.input().scroll_delta.y;
        }

        let show_scroll_this_frame = content_size.y > inner_size.y || self.always_show_scroll;
        if show_scroll_this_frame || state.show_scroll {
            let left = inner_rect.right() + 2.0;
            let right = outer_rect.right();
            let corner_radius = (right - left) / 2.0;
            let top = inner_rect.top();
            let bottom = inner_rect.bottom();

            let outer_scroll_rect = Rect::from_min_max(
                pos2(left, inner_rect.top()),
                pos2(right, inner_rect.bottom()),
            );

            let from_content =
                |content_y| remap_clamp(content_y, 0.0..=content_size.y, top..=bottom);

            let handle_rect = Rect::from_min_max(
                pos2(left, from_content(state.offset.y)),
                pos2(right, from_content(state.offset.y + inner_rect.height())),
            );

            // intentionally use same id for inside and outside of handle
            let interact_id = scroll_area_id.with("vertical");
            let handle_interact = outer_region.interact_rect(handle_rect, interact_id);

            if let Some(mouse_pos) = ctx.input().mouse_pos {
                if handle_interact.active {
                    if inner_rect.top() <= mouse_pos.y && mouse_pos.y <= inner_rect.bottom() {
                        state.offset.y +=
                            ctx.input().mouse_move.y * content_size.y / inner_rect.height();
                    }
                } else {
                    // Check for mouse down outside handle:
                    let scroll_bg_interact =
                        outer_region.interact_rect(outer_scroll_rect, interact_id);

                    if scroll_bg_interact.active {
                        // Center scroll at mouse pos:
                        let mpos_top = mouse_pos.y - handle_rect.height() / 2.0;
                        state.offset.y = remap(mpos_top, top..=bottom, 0.0..=content_size.y);
                    }
                }
            }

            state.offset.y = state.offset.y.max(0.0);
            state.offset.y = state.offset.y.min(content_size.y - inner_rect.height());

            // Avoid frame-delay by calculating a new handle rect:
            let handle_rect = Rect::from_min_max(
                pos2(left, from_content(state.offset.y)),
                pos2(right, from_content(state.offset.y + inner_rect.height())),
            );

            let style = outer_region.style();
            let handle_fill_color = style.interact_fill_color(&handle_interact);
            let handle_outline = style.interact_outline(&handle_interact);

            outer_region.add_paint_cmd(PaintCmd::Rect {
                rect: outer_scroll_rect,
                corner_radius,
                fill_color: Some(color::gray(0, 196)), // TODO style
                outline: None,
            });

            outer_region.add_paint_cmd(PaintCmd::Rect {
                rect: handle_rect.expand(-2.0),
                corner_radius,
                fill_color: handle_fill_color,
                outline: handle_outline,
            });
        }

        // let size = content_size.min(inner_rect.size());
        // let size = vec2(
        //     content_size.x, // ignore inner_rect, i.e. try to expand horizontally if necessary
        //     content_size.y.min(inner_rect.size().y), // respect vertical height.
        // );
        let size = outer_rect.size();
        outer_region.reserve_space(size, None);

        state.offset.y = state.offset.y.min(content_size.y - inner_rect.height());
        state.offset.y = state.offset.y.max(0.0);
        state.show_scroll = show_scroll_this_frame;

        outer_region
            .memory()
            .scroll_areas
            .insert(scroll_area_id, state);
    }
}
