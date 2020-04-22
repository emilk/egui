use crate::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    /// Positive offset means scrolling down/right
    pub offset: Vec2,
}

pub struct ScrollArea {
    max_height: f32,
}

impl Default for ScrollArea {
    fn default() -> Self {
        Self { max_height: 200.0 }
    }
}

impl ScrollArea {
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }
}

impl ScrollArea {
    pub fn show<F>(self, outer_region: &mut Region, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        let ctx = outer_region.ctx().clone();

        let scroll_area_id = outer_region.id.with("scroll_area");
        let mut state = ctx
            .memory
            .lock()
            .scroll_areas
            .get(&scroll_area_id)
            .cloned()
            .unwrap_or_default();

        // content: size of contents (generally large)
        // outer: size of scroll area including scroll bars
        // inner: excluding scroll bars

        let scroll_bar_width = 16.0;

        let outer_size = vec2(outer_region.available_width(), self.max_height);
        let outer_rect = Rect::from_min_size(outer_region.cursor, outer_size);

        let inner_size = outer_size - vec2(scroll_bar_width, 0.0);
        let inner_rect = Rect::from_min_size(outer_region.cursor, inner_size);

        let mut content_region =
            outer_region.child_region(Rect::from_min_size(outer_region.cursor(), inner_size));
        content_region.cursor -= state.offset;
        content_region.desired_rect = content_region.desired_rect.translate(-state.offset);
        add_contents(&mut content_region);
        let content_size = content_region.bounding_size;

        let content_interact = ctx.interact(
            outer_region.layer,
            inner_rect,
            Some(scroll_area_id.with("area")),
        );
        if content_interact.active {
            // Dragging scroll area to scroll:
            state.offset.y -= ctx.input.mouse_move.y;
        }

        let show_scroll = content_size.y > inner_size.y;
        if show_scroll {
            let corner_radius = scroll_bar_width / 2.0;

            let left = inner_rect.right();
            let right = outer_rect.right();

            let outer_scroll_rect = Rect::from_min_max(
                pos2(left, inner_rect.top()),
                pos2(right, inner_rect.bottom()),
            );

            let scroll_handle_min_y = remap_clamp(
                state.offset.y,
                0.0,
                content_size.y,
                inner_rect.top(),
                inner_rect.bottom(),
            );

            let scroll_handle_max_y = remap_clamp(
                state.offset.y + inner_rect.height(),
                0.0,
                content_size.y,
                inner_rect.top(),
                inner_rect.bottom(),
            );

            let scroll_handle_rect = Rect::from_min_max(
                pos2(left, scroll_handle_min_y),
                pos2(right, scroll_handle_max_y),
            );

            let scroll_handle_id = scroll_area_id.with("v_scroll_handle");
            let handle_interact = ctx.interact(
                outer_region.layer,
                scroll_handle_rect,
                Some(scroll_handle_id),
            );

            if handle_interact.active {
                // state.offset.y = remap_clamp(
                //     ctx.input.mouse_pos.y,
                //     inner_rect.top(),
                //     inner_rect.bottom(),
                //     0.0,
                //     content_size.y,
                // );
                if let Some(mouse_pos) = ctx.input.mouse_pos {
                    if inner_rect.top() <= mouse_pos.y && mouse_pos.y <= inner_rect.bottom() {
                        state.offset.y +=
                            ctx.input.mouse_move.y * content_size.y / inner_rect.height();
                    }
                }
            }

            let style = outer_region.style();
            // let background_fill_color = style.background_fill_color();
            let handle_fill_color = style.interact_fill_color(&handle_interact);
            let handle_outline = style.interact_outline(&handle_interact);

            outer_region.add_paint_cmd(PaintCmd::Rect {
                rect: outer_scroll_rect,
                corner_radius,
                fill_color: Some(color::BLACK),
                outline: None,
            });

            outer_region.add_paint_cmd(PaintCmd::Rect {
                rect: scroll_handle_rect,
                corner_radius,
                fill_color: handle_fill_color,
                outline: handle_outline,
            });
        }

        let size = content_size.min(content_region.clip_rect.size());
        outer_region.reserve_space_without_padding(size);

        state.offset.y = state.offset.y.max(0.0);
        state.offset.y = state.offset.y.min(content_size.y - inner_rect.height());

        outer_region
            .ctx()
            .memory
            .lock()
            .scroll_areas
            .insert(scroll_area_id, state);
    }
}
