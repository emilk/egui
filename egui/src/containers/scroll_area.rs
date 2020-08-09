use crate::*;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub(crate) struct State {
    /// Positive offset means scrolling down/right
    offset: Vec2,

    show_scroll: bool,

    // Times are relative, and we don't want to continue animations anyway, hence `serde(skip)`
    /// Used to animate the showing of the scroll bar
    #[cfg_attr(feature = "serde", serde(skip))]
    toggle_time: f64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            offset: Vec2::zero(),
            show_scroll: false,
            toggle_time: f64::NEG_INFINITY,
        }
    }
}

// TODO: rename VScroll
/// Add vertical scrolling to a contained `Ui`.
#[derive(Clone, Debug)]
pub struct ScrollArea {
    max_height: f32,
    always_show_scroll: bool,
}

impl Default for ScrollArea {
    fn default() -> Self {
        Self {
            max_height: 200.0,
            always_show_scroll: false,
        }
    }
}

impl ScrollArea {
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }

    /// If `false` (defualt), the scroll bar will be hidden when not needed/
    /// If `true`, the scroll bar will always be displayed even if not needed.
    pub fn always_show_scroll(mut self, always_show_scroll: bool) -> Self {
        self.always_show_scroll = always_show_scroll;
        self
    }
}

struct Prepared {
    id: Id,
    state: State,
    current_scroll_bar_width: f32,
    always_show_scroll: bool,
    inner_rect: Rect,
    content_ui: Ui,
}

impl ScrollArea {
    fn begin(self, ui: &mut Ui) -> Prepared {
        let Self {
            max_height,
            always_show_scroll,
        } = self;

        let ctx = ui.ctx().clone();

        let id = ui.make_child_id("scroll_area");
        let state = ctx
            .memory()
            .scroll_areas
            .get(&id)
            .cloned()
            .unwrap_or_default();

        // content: size of contents (generally large; that's why we want scroll bars)
        // outer: size of scroll area including scroll bar(s)
        // inner: excluding scroll bar(s). The area we clip the contents to.

        let max_scroll_bar_width = max_scroll_bar_width_with_margin(ui);

        let current_scroll_bar_width = if always_show_scroll {
            max_scroll_bar_width
        } else {
            let time_since_toggle = (ui.input().time - state.toggle_time) as f32;
            let animation_time = ui.style().animation_time;
            if time_since_toggle <= animation_time {
                ui.ctx().request_repaint();
            }
            if state.show_scroll {
                remap_clamp(
                    time_since_toggle,
                    0.0..=animation_time,
                    0.0..=max_scroll_bar_width,
                )
            } else {
                remap_clamp(
                    time_since_toggle,
                    0.0..=animation_time,
                    max_scroll_bar_width..=0.0,
                )
            }
        };

        let outer_size = vec2(
            ui.available().width(),
            ui.available().height().min(max_height),
        );

        let inner_size = outer_size - vec2(current_scroll_bar_width, 0.0);
        let inner_rect = Rect::from_min_size(ui.available().min, inner_size);

        let mut content_ui = ui.child_ui(Rect::from_min_size(
            inner_rect.min - state.offset,
            vec2(inner_size.x, f32::INFINITY),
        ));
        let mut content_clip_rect = inner_rect.expand(ui.style().clip_rect_margin);
        content_clip_rect = content_clip_rect.intersect(ui.clip_rect());
        content_clip_rect.max.x = ui.clip_rect().max.x - current_scroll_bar_width; // Nice handling of forced resizing beyond the possible
        content_ui.set_clip_rect(content_clip_rect);

        Prepared {
            id,
            state,
            always_show_scroll,
            inner_rect,
            current_scroll_bar_width,
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
    fn end(self, ui: &mut Ui) {
        let Prepared {
            id,
            mut state,
            inner_rect,
            always_show_scroll,
            mut current_scroll_bar_width,
            content_ui,
        } = self;

        let content_size = content_ui.bounding_size();

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

        let content_is_too_small = content_size.y > inner_rect.height();

        if content_is_too_small {
            // Drag contents to scroll (for touch screens mostly):
            let content_interact = ui.interact(inner_rect, id.with("area"), Sense::drag());
            if content_interact.active {
                state.offset.y -= ui.input().mouse.delta.y;
            }
        }

        // TODO: check that nothing else is being interacted with
        if ui.contains_mouse(outer_rect) {
            state.offset.y -= ui.input().scroll_delta.y;
        }

        let show_scroll_this_frame = content_is_too_small || always_show_scroll;

        let max_scroll_bar_width = max_scroll_bar_width_with_margin(ui);

        if show_scroll_this_frame && current_scroll_bar_width <= 0.0 {
            // Avoid frame delay; start shwoing scroll bar right away:
            current_scroll_bar_width = remap_clamp(
                ui.input().predicted_dt,
                0.0..=ui.style().animation_time,
                0.0..=max_scroll_bar_width,
            );
            ui.ctx().request_repaint();
        }

        if current_scroll_bar_width > 0.0 {
            let animation_t = current_scroll_bar_width / max_scroll_bar_width;
            // margin between contents and scroll bar
            let margin = animation_t * ui.style().item_spacing.x;
            let left = inner_rect.right() + margin;
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
            let interact_id = id.with("vertical");
            let mut interact = ui.interact(handle_rect, interact_id, Sense::click_and_drag());

            if let Some(mouse_pos) = ui.input().mouse.pos {
                if interact.active {
                    if inner_rect.top() <= mouse_pos.y && mouse_pos.y <= inner_rect.bottom() {
                        state.offset.y +=
                            ui.input().mouse.delta.y * content_size.y / inner_rect.height();
                    }
                } else {
                    // Check for mouse down outside handle:
                    let scroll_bg_interact =
                        ui.interact(outer_scroll_rect, interact_id, Sense::click_and_drag());

                    if scroll_bg_interact.active {
                        // Center scroll at mouse pos:
                        let mpos_top = mouse_pos.y - handle_rect.height() / 2.0;
                        state.offset.y = remap(mpos_top, top..=bottom, 0.0..=content_size.y);
                    }

                    interact = interact.union(scroll_bg_interact);
                }
            }

            state.offset.y = state.offset.y.max(0.0);
            state.offset.y = state.offset.y.min(content_size.y - inner_rect.height());

            // Avoid frame-delay by calculating a new handle rect:
            let mut handle_rect = Rect::from_min_max(
                pos2(left, from_content(state.offset.y)),
                pos2(right, from_content(state.offset.y + inner_rect.height())),
            );
            let min_handle_height = (2.0 * corner_radius).max(8.0);
            if handle_rect.size().y < min_handle_height {
                handle_rect = Rect::from_center_size(
                    handle_rect.center(),
                    vec2(handle_rect.size().x, min_handle_height),
                );
            }

            let style = ui.style();
            let handle_fill = style.interact(&interact).fill;
            let handle_outline = style.interact(&interact).rect_outline;

            ui.painter().add(paint::PaintCmd::Rect {
                rect: outer_scroll_rect,
                corner_radius,
                fill: Some(ui.style().dark_bg_color),
                outline: None,
            });

            ui.painter().add(paint::PaintCmd::Rect {
                rect: handle_rect.expand(-2.0),
                corner_radius,
                fill: Some(handle_fill),
                outline: handle_outline,
            });
        }

        let size = vec2(
            outer_rect.size().x,
            outer_rect.size().y.min(content_size.y), // shrink if content is so small that we don't need scroll bars
        );
        ui.allocate_space(size);

        if show_scroll_this_frame != state.show_scroll {
            state.toggle_time = ui.input().time;
            ui.ctx().request_repaint();
        }

        state.offset.y = state.offset.y.min(content_size.y - inner_rect.height());
        state.offset.y = state.offset.y.max(0.0);
        state.show_scroll = show_scroll_this_frame;

        ui.memory().scroll_areas.insert(id, state);
    }
}

fn max_scroll_bar_width_with_margin(ui: &Ui) -> f32 {
    ui.style().item_spacing.x + 16.0
}
