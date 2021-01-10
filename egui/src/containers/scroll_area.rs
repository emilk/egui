use crate::*;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub(crate) struct State {
    /// Positive offset means scrolling down/right
    offset: Vec2,

    show_scroll: bool,

    /// Momentum, used for kinetic scrolling
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub vel: Vec2,
    /// Mouse offset relative to the top of the handle when started moving the handle.
    scroll_start_offset_from_top: Option<f32>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            offset: Vec2::zero(),
            show_scroll: false,
            vel: Vec2::zero(),
            scroll_start_offset_from_top: None,
        }
    }
}

// TODO: rename VScroll
/// Add vertical scrolling to a contained [`Ui`].
#[derive(Clone, Debug)]
pub struct ScrollArea {
    max_height: f32,
    always_show_scroll: bool,
    id_source: Option<Id>,
    offset: Option<Vec2>,
}

impl ScrollArea {
    /// Will make the area be as high as it is allowed to be (i.e. fill the [`Ui`] it is in)
    pub fn auto_sized() -> Self {
        Self::from_max_height(f32::INFINITY)
    }

    /// Use `f32::INFINITY` if you want the scroll area to expand to fit the surrounding Ui
    pub fn from_max_height(max_height: f32) -> Self {
        Self {
            max_height,
            always_show_scroll: false,
            id_source: None,
            offset: None,
        }
    }

    /// If `false` (default), the scroll bar will be hidden when not needed/
    /// If `true`, the scroll bar will always be displayed even if not needed.
    pub fn always_show_scroll(mut self, always_show_scroll: bool) -> Self {
        self.always_show_scroll = always_show_scroll;
        self
    }

    /// A source for the unique `Id`, e.g. `.id_source("second_scroll_area")` or `.id_source(loop_index)`.
    pub fn id_source(mut self, id_source: impl std::hash::Hash) -> Self {
        self.id_source = Some(Id::new(id_source));
        self
    }

    /// Set the vertical scroll offset position.
    ///
    /// See also: [`Ui::scroll_to_cursor`](crate::ui::Ui::scroll_to_cursor) and
    /// [`Response::scroll_to_me`](crate::types::Response::scroll_to_me)
    pub fn scroll_offset(mut self, offset: f32) -> Self {
        self.offset = Some(Vec2::new(0.0, offset));
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
            id_source,
            offset,
        } = self;

        let ctx = ui.ctx().clone();

        let id_source = id_source.unwrap_or_else(|| Id::new("scroll_area"));
        let id = ui.make_persistent_id(id_source);
        let mut state = ctx
            .memory()
            .scroll_areas
            .get(&id)
            .cloned()
            .unwrap_or_default();

        if let Some(offset) = offset {
            state.offset = offset;
        }

        // content: size of contents (generally large; that's why we want scroll bars)
        // outer: size of scroll area including scroll bar(s)
        // inner: excluding scroll bar(s). The area we clip the contents to.

        let max_scroll_bar_width = max_scroll_bar_width_with_margin(ui);

        let current_scroll_bar_width = if always_show_scroll {
            max_scroll_bar_width
        } else {
            max_scroll_bar_width * ui.ctx().animate_bool(id, state.show_scroll)
        };

        let available_outer = ui.available_rect_before_wrap();

        let outer_size = vec2(
            available_outer.width(),
            available_outer.height().at_most(max_height),
        );

        let inner_size = outer_size - vec2(current_scroll_bar_width, 0.0);
        let inner_rect = Rect::from_min_size(available_outer.min, inner_size);

        let mut content_ui = ui.child_ui(
            Rect::from_min_size(
                inner_rect.min - state.offset,
                vec2(inner_size.x, f32::INFINITY),
            ),
            *ui.layout(),
        );
        let mut content_clip_rect = inner_rect.expand(ui.style().visuals.clip_rect_margin);
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

        let content_size = content_ui.min_size();

        // We take the scroll target so only this ScrollArea will use it.
        let scroll_target = content_ui.ctx().frame_state().scroll_target.take();
        if let Some((scroll_y, align)) = scroll_target {
            let center_factor = align.to_factor();

            let top = content_ui.min_rect().top();
            let visible_range = top..=top + content_ui.clip_rect().height();
            let offset_y = scroll_y - lerp(visible_range, center_factor);

            let mut spacing = ui.style().spacing.item_spacing.y;

            // Depending on the alignment we need to add or subtract the spacing
            spacing *= remap(center_factor, 0.0..=1.0, -1.0..=1.0);

            state.offset.y = offset_y + spacing;
        }

        let width = if inner_rect.width().is_finite() {
            inner_rect.width().max(content_size.x) // Expand width to fit content
        } else {
            // ScrollArea is in an infinitely wide parent
            content_size.x
        };

        let inner_rect = Rect::from_min_size(inner_rect.min, vec2(width, inner_rect.height()));

        let outer_rect = Rect::from_min_size(
            inner_rect.min,
            inner_rect.size() + vec2(current_scroll_bar_width, 0.0),
        );

        let content_is_too_small = content_size.y > inner_rect.height();

        if content_is_too_small {
            // Drag contents to scroll (for touch screens mostly):
            let content_response = ui.interact(inner_rect, id.with("area"), Sense::drag());

            let input = ui.input();
            if content_response.active {
                state.offset.y -= input.mouse.delta.y;
                state.vel = input.mouse.velocity;
            } else {
                let stop_speed = 20.0; // Pixels per second.
                let friction_coeff = 1000.0; // Pixels per second squared.
                let dt = input.unstable_dt;

                let friction = friction_coeff * dt;
                if friction > state.vel.length() || state.vel.length() < stop_speed {
                    state.vel = Vec2::zero();
                } else {
                    state.vel -= friction * state.vel.normalized();
                    // Offset has an inverted coordinate system compared to
                    // the velocity, so we subtract it instead of adding it
                    state.offset.y -= state.vel.y * dt;
                    ui.ctx().request_repaint();
                }
            }
        }

        let max_offset = content_size.y - inner_rect.height();
        if ui.rect_contains_mouse(outer_rect) {
            let mut frame_state = ui.ctx().frame_state();
            let scroll_delta = frame_state.scroll_delta;

            let scrolling_up = state.offset.y > 0.0 && scroll_delta.y > 0.0;
            let scrolling_down = state.offset.y < max_offset && scroll_delta.y < 0.0;

            if scrolling_up || scrolling_down {
                state.offset.y -= scroll_delta.y;
                // Clear scroll delta so no parent scroll will use it.
                frame_state.scroll_delta = Vec2::zero();
            }
        }

        let show_scroll_this_frame = content_is_too_small || always_show_scroll;

        let max_scroll_bar_width = max_scroll_bar_width_with_margin(ui);

        if show_scroll_this_frame && current_scroll_bar_width <= 0.0 {
            // Avoid frame delay; start showing scroll bar right away:
            current_scroll_bar_width = max_scroll_bar_width * ui.ctx().animate_bool(id, true);
        }

        if current_scroll_bar_width > 0.0 {
            let animation_t = current_scroll_bar_width / max_scroll_bar_width;
            // margin between contents and scroll bar
            let margin = animation_t * ui.style().spacing.item_spacing.x;
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

            let interact_id = id.with("vertical");
            let response = ui.interact(outer_scroll_rect, interact_id, Sense::click_and_drag());

            if response.active {
                if let Some(mouse_pos) = ui.input().mouse.pos {
                    let scroll_start_offset_from_top =
                        state.scroll_start_offset_from_top.get_or_insert_with(|| {
                            if handle_rect.contains(mouse_pos) {
                                mouse_pos.y - handle_rect.top()
                            } else {
                                let handle_top_pos_at_bottom = bottom - handle_rect.height();
                                // Calculate the new handle top position, centering the handle on the mouse.
                                let new_handle_top_pos = clamp(
                                    mouse_pos.y - handle_rect.height() / 2.0,
                                    top..=handle_top_pos_at_bottom,
                                );
                                mouse_pos.y - new_handle_top_pos
                            }
                        });

                    let new_handle_top = mouse_pos.y - *scroll_start_offset_from_top;
                    state.offset.y = remap(new_handle_top, top..=bottom, 0.0..=content_size.y);
                }
            } else {
                state.scroll_start_offset_from_top = None;
            }

            state.offset.y = state.offset.y.max(0.0);
            state.offset.y = state.offset.y.min(max_offset);

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

            let visuals = ui.style().interact(&response);

            ui.painter().add(paint::Shape::Rect {
                rect: outer_scroll_rect,
                corner_radius,
                fill: ui.style().visuals.dark_bg_color,
                stroke: Default::default(),
                // fill: visuals.bg_fill,
                // stroke: visuals.bg_stroke,
            });

            ui.painter().add(paint::Shape::Rect {
                rect: handle_rect.expand(-2.0),
                corner_radius,
                fill: visuals.fg_fill,
                stroke: visuals.fg_stroke,
            });
        }

        let size = vec2(
            outer_rect.size().x,
            outer_rect.size().y.min(content_size.y), // shrink if content is so small that we don't need scroll bars
        );
        ui.advance_cursor_after_rect(Rect::from_min_size(outer_rect.min, size));

        if show_scroll_this_frame != state.show_scroll {
            ui.ctx().request_repaint();
        }

        state.offset.y = state.offset.y.min(content_size.y - inner_rect.height());
        state.offset.y = state.offset.y.max(0.0);
        state.show_scroll = show_scroll_this_frame;

        ui.memory().scroll_areas.insert(id, state);
    }
}

fn max_scroll_bar_width_with_margin(ui: &Ui) -> f32 {
    ui.style().spacing.item_spacing.x + 16.0
}
