//! Coordinate system names:
//! * content: size of contents (generally large; that's why we want scroll bars)
//! * outer: size of scroll area including scroll bar(s)
//! * inner: excluding scroll bar(s). The area we clip the contents to.

#![allow(clippy::needless_range_loop)]

use crate::*;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct State {
    /// Positive offset means scrolling down/right
    pub offset: Vec2,

    /// Were the scroll bars visible last frame?
    show_scroll: [bool; 2],

    /// The content were to large to fit large frame.
    content_is_too_large: [bool; 2],

    /// Momentum, used for kinetic scrolling
    #[cfg_attr(feature = "serde", serde(skip))]
    vel: Vec2,

    /// Mouse offset relative to the top of the handle when started moving the handle.
    scroll_start_offset_from_top_left: [Option<f32>; 2],

    /// Is the scroll sticky. This is true while scroll handle is in the end position
    /// and remains that way until the user moves the scroll_handle. Once unstuck (false)
    /// it remains false until the scroll touches the end position, which reenables stickiness.
    scroll_stuck_to_end: [bool; 2],
}

impl Default for State {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            show_scroll: [false; 2],
            content_is_too_large: [false; 2],
            vel: Vec2::ZERO,
            scroll_start_offset_from_top_left: [None; 2],
            scroll_stuck_to_end: [true; 2],
        }
    }
}

impl State {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(id))
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_persisted(id, self));
    }
}

pub struct ScrollAreaOutput<R> {
    /// What the user closure returned.
    pub inner: R,

    /// [`Id`] of the [`ScrollArea`].
    pub id: Id,

    /// The current state of the scroll area.
    pub state: State,

    /// The size of the content. If this is larger than [`Self::inner_rect`],
    /// then there was need for scrolling.
    pub content_size: Vec2,

    /// Where on the screen the content is (excludes scroll bars).
    pub inner_rect: Rect,
}

/// Indicate whether the horizontal and vertical scroll bars must be always visible, hidden or visible when needed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScrollBarVisibility {
    AlwaysVisible,
    VisibleWhenNeeded,
    AlwaysHidden,
}

/// Add vertical and/or horizontal scrolling to a contained [`Ui`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::ScrollArea::vertical().show(ui, |ui| {
///     // Add a lot of widgets here.
/// });
/// # });
/// ```
///
/// You can scroll to an element using [`Response::scroll_to_me`], [`Ui::scroll_to_cursor`] and [`Ui::scroll_to_rect`].
#[derive(Clone, Debug)]
#[must_use = "You should call .show()"]
pub struct ScrollArea {
    /// Do we have horizontal/vertical scrolling?
    has_bar: [bool; 2],
    auto_shrink: [bool; 2],
    max_size: Vec2,
    min_scrolled_size: Vec2,
    scroll_bar_visibility: ScrollBarVisibility,
    id_source: Option<Id>,
    offset_x: Option<f32>,
    offset_y: Option<f32>,

    /// If false, we ignore scroll events.
    scrolling_enabled: bool,
    drag_to_scroll: bool,

    /// If true for vertical or horizontal the scroll wheel will stick to the
    /// end position until user manually changes position. It will become true
    /// again once scroll handle makes contact with end.
    stick_to_end: [bool; 2],
}

impl ScrollArea {
    /// Create a horizontal scroll area.
    pub fn horizontal() -> Self {
        Self::new([true, false])
    }

    /// Create a vertical scroll area.
    pub fn vertical() -> Self {
        Self::new([false, true])
    }

    /// Create a bi-directional (horizontal and vertical) scroll area.
    pub fn both() -> Self {
        Self::new([true, true])
    }

    /// Create a scroll area where both direction of scrolling is disabled.
    /// It's unclear why you would want to do this.
    pub fn neither() -> Self {
        Self::new([false, false])
    }

    /// Create a scroll area where you decide which axis has scrolling enabled.
    /// For instance, `ScrollArea::new([true, false])` enables horizontal scrolling.
    pub fn new(has_bar: [bool; 2]) -> Self {
        Self {
            has_bar,
            auto_shrink: [true; 2],
            max_size: Vec2::INFINITY,
            min_scrolled_size: Vec2::splat(64.0),
            scroll_bar_visibility: ScrollBarVisibility::AlwaysHidden,
            id_source: None,
            offset_x: None,
            offset_y: None,
            scrolling_enabled: true,
            drag_to_scroll: true,
            stick_to_end: [false; 2],
        }
    }

    /// The maximum width of the outer frame of the scroll area.
    ///
    /// Use `f32::INFINITY` if you want the scroll area to expand to fit the surrounding [`Ui`] (default).
    ///
    /// See also [`Self::auto_shrink`].
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_size.x = max_width;
        self
    }

    /// The maximum height of the outer frame of the scroll area.
    ///
    /// Use `f32::INFINITY` if you want the scroll area to expand to fit the surrounding [`Ui`] (default).
    ///
    /// See also [`Self::auto_shrink`].
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_size.y = max_height;
        self
    }

    /// The minimum width of a horizontal scroll area which requires scroll bars.
    ///
    /// The [`ScrollArea`] will only become smaller than this if the content is smaller than this
    /// (and so we don't require scroll bars).
    ///
    /// Default: `64.0`.
    pub fn min_scrolled_width(mut self, min_scrolled_width: f32) -> Self {
        self.min_scrolled_size.x = min_scrolled_width;
        self
    }

    /// The minimum height of a vertical scroll area which requires scroll bars.
    ///
    /// The [`ScrollArea`] will only become smaller than this if the content is smaller than this
    /// (and so we don't require scroll bars).
    ///
    /// Default: `64.0`.
    pub fn min_scrolled_height(mut self, min_scrolled_height: f32) -> Self {
        self.min_scrolled_size.y = min_scrolled_height;
        self
    }

    /// Set the visibility of both horizontal and vertical scroll bars.
    ///
    /// With `ScrollBarVisibility::VisibleWhenNeeded` (default), the scroll bar will be visible only when needed.
    pub fn scroll_bar_visibility(mut self, scroll_bar_visibility: ScrollBarVisibility) -> Self {
        self.scroll_bar_visibility = scroll_bar_visibility;
        self
    }

    /// A source for the unique [`Id`], e.g. `.id_source("second_scroll_area")` or `.id_source(loop_index)`.
    pub fn id_source(mut self, id_source: impl std::hash::Hash) -> Self {
        self.id_source = Some(Id::new(id_source));
        self
    }

    /// Set the horizontal and vertical scroll offset position.
    ///
    /// Positive offset means scrolling down/right.
    ///
    /// See also: [`Self::vertical_scroll_offset`], [`Self::horizontal_scroll_offset`],
    /// [`Ui::scroll_to_cursor`](crate::ui::Ui::scroll_to_cursor) and
    /// [`Response::scroll_to_me`](crate::Response::scroll_to_me)
    pub fn scroll_offset(mut self, offset: Vec2) -> Self {
        self.offset_x = Some(offset.x);
        self.offset_y = Some(offset.y);
        self
    }

    /// Set the vertical scroll offset position.
    ///
    /// Positive offset means scrolling down.
    ///
    /// See also: [`Self::scroll_offset`], [`Ui::scroll_to_cursor`](crate::ui::Ui::scroll_to_cursor) and
    /// [`Response::scroll_to_me`](crate::Response::scroll_to_me)
    pub fn vertical_scroll_offset(mut self, offset: f32) -> Self {
        self.offset_y = Some(offset);
        self
    }

    /// Set the horizontal scroll offset position.
    ///
    /// Positive offset means scrolling right.
    ///
    /// See also: [`Self::scroll_offset`], [`Ui::scroll_to_cursor`](crate::ui::Ui::scroll_to_cursor) and
    /// [`Response::scroll_to_me`](crate::Response::scroll_to_me)
    pub fn horizontal_scroll_offset(mut self, offset: f32) -> Self {
        self.offset_x = Some(offset);
        self
    }

    /// Turn on/off scrolling on the horizontal axis.
    pub fn hscroll(mut self, hscroll: bool) -> Self {
        self.has_bar[0] = hscroll;
        self
    }

    /// Turn on/off scrolling on the vertical axis.
    pub fn vscroll(mut self, vscroll: bool) -> Self {
        self.has_bar[1] = vscroll;
        self
    }

    /// Turn on/off scrolling on the horizontal/vertical axes.
    pub fn scroll2(mut self, has_bar: [bool; 2]) -> Self {
        self.has_bar = has_bar;
        self
    }

    /// Control the scrolling behavior.
    ///
    /// * If `true` (default), the scroll area will respond to user scrolling.
    /// * If `false`, the scroll area will not respond to user scrolling.
    ///
    /// This can be used, for example, to optionally freeze scrolling while the user
    /// is typing text in a [`TextEdit`] widget contained within the scroll area.
    ///
    /// This controls both scrolling directions.
    pub fn enable_scrolling(mut self, enable: bool) -> Self {
        self.scrolling_enabled = enable;
        self
    }

    /// Can the user drag the scroll area to scroll?
    ///
    /// This is useful for touch screens.
    ///
    /// If `true`, the [`ScrollArea`] will sense drags.
    ///
    /// Default: `true`.
    pub fn drag_to_scroll(mut self, drag_to_scroll: bool) -> Self {
        self.drag_to_scroll = drag_to_scroll;
        self
    }

    /// For each axis, should the containing area shrink if the content is small?
    ///
    /// * If `true`, egui will add blank space outside the scroll area.
    /// * If `false`, egui will add blank space inside the scroll area.
    ///
    /// Default: `[true; 2]`.
    pub fn auto_shrink(mut self, auto_shrink: [bool; 2]) -> Self {
        self.auto_shrink = auto_shrink;
        self
    }

    pub(crate) fn has_any_bar(&self) -> bool {
        self.has_bar[0] || self.has_bar[1]
    }

    /// The scroll handle will stick to the rightmost position even while the content size
    /// changes dynamically. This can be useful to simulate text scrollers coming in from right
    /// hand side. The scroll handle remains stuck until user manually changes position. Once "unstuck"
    /// it will remain focused on whatever content viewport the user left it on. If the scroll
    /// handle is dragged all the way to the right it will again become stuck and remain there
    /// until manually pulled from the end position.
    pub fn stick_to_right(mut self, stick: bool) -> Self {
        self.stick_to_end[0] = stick;
        self
    }

    /// The scroll handle will stick to the bottom position even while the content size
    /// changes dynamically. This can be useful to simulate terminal UIs or log/info scrollers.
    /// The scroll handle remains stuck until user manually changes position. Once "unstuck"
    /// it will remain focused on whatever content viewport the user left it on. If the scroll
    /// handle is dragged to the bottom it will again become stuck and remain there until manually
    /// pulled from the end position.
    pub fn stick_to_bottom(mut self, stick: bool) -> Self {
        self.stick_to_end[1] = stick;
        self
    }
}

struct Prepared {
    id: Id,
    state: State,
    has_bar: [bool; 2],
    auto_shrink: [bool; 2],
    /// How much horizontal and vertical space are used up by the
    /// width of the vertical bar, and the height of the horizontal bar?
    current_bar_use: Vec2,
    scroll_bar_visibility: ScrollBarVisibility,
    /// Where on the screen the content is (excludes scroll bars).
    inner_rect: Rect,
    content_ui: Ui,
    /// Relative coordinates: the offset and size of the view of the inner UI.
    /// `viewport.min == ZERO` means we scrolled to the top.
    viewport: Rect,
    scrolling_enabled: bool,
    stick_to_end: [bool; 2],
}

impl ScrollArea {
    fn begin(self, ui: &mut Ui) -> Prepared {
        let Self {
            has_bar,
            auto_shrink,
            max_size,
            min_scrolled_size,
            scroll_bar_visibility,
            id_source,
            offset_x,
            offset_y,
            scrolling_enabled,
            drag_to_scroll,
            stick_to_end,
        } = self;

        let ctx = ui.ctx().clone();

        let id_source = id_source.unwrap_or_else(|| Id::new("scroll_area"));
        let id = ui.make_persistent_id(id_source);
        ui.ctx().check_for_id_clash(
            id,
            Rect::from_min_size(ui.available_rect_before_wrap().min, Vec2::ZERO),
            "ScrollArea",
        );
        let mut state = State::load(&ctx, id).unwrap_or_default();

        state.offset.x = offset_x.unwrap_or(state.offset.x);
        state.offset.y = offset_y.unwrap_or(state.offset.y);

        let max_scroll_bar_width = max_scroll_bar_width_with_margin(ui);

        let current_hscroll_bar_height = if !has_bar[0] {
            0.0
        } else if scroll_bar_visibility == ScrollBarVisibility::AlwaysVisible {
            max_scroll_bar_width
        } else {
            max_scroll_bar_width * ui.ctx().animate_bool(id.with("h"), state.show_scroll[0])
        };

        let current_vscroll_bar_width = if !has_bar[1] {
            0.0
        } else if scroll_bar_visibility == ScrollBarVisibility::AlwaysVisible {
            max_scroll_bar_width
        } else {
            max_scroll_bar_width * ui.ctx().animate_bool(id.with("v"), state.show_scroll[1])
        };

        let current_bar_use = vec2(current_vscroll_bar_width, current_hscroll_bar_height);

        let available_outer = ui.available_rect_before_wrap();

        let outer_size = available_outer.size().at_most(max_size);

        let inner_size = {
            let mut inner_size = outer_size - current_bar_use;

            // Don't go so far that we shrink to zero.
            // In particular, if we put a [`ScrollArea`] inside of a [`ScrollArea`], the inner
            // one shouldn't collapse into nothingness.
            // See https://github.com/emilk/egui/issues/1097
            for d in 0..2 {
                if has_bar[d] {
                    inner_size[d] = inner_size[d].max(min_scrolled_size[d]);
                }
            }
            inner_size
        };

        let inner_rect = Rect::from_min_size(available_outer.min, inner_size);

        let mut content_max_size = inner_size;

        if true {
            // Tell the inner Ui to *try* to fit the content without needing to scroll,
            // i.e. better to wrap text and shrink images than showing a horizontal scrollbar!
        } else {
            // Tell the inner Ui to use as much space as possible, we can scroll to see it!
            for d in 0..2 {
                if has_bar[d] {
                    content_max_size[d] = f32::INFINITY;
                }
            }
        }

        let content_max_rect = Rect::from_min_size(inner_rect.min - state.offset, content_max_size);
        let mut content_ui = ui.child_ui(content_max_rect, *ui.layout());

        {
            // Clip the content, but only when we really need to:
            let clip_rect_margin = ui.visuals().clip_rect_margin;
            let scroll_bar_inner_margin = ui.spacing().scroll_bar_inner_margin;
            let mut content_clip_rect = ui.clip_rect();
            for d in 0..2 {
                if has_bar[d] {
                    if state.content_is_too_large[d] {
                        content_clip_rect.min[d] = inner_rect.min[d] - clip_rect_margin;
                        content_clip_rect.max[d] = inner_rect.max[d] + clip_rect_margin;
                    }

                    if state.show_scroll[d] {
                        // Make sure content doesn't cover scroll bars
                        let tiny_gap = 1.0;
                        content_clip_rect.max[1 - d] =
                            inner_rect.max[1 - d] + scroll_bar_inner_margin - tiny_gap;
                    }
                } else {
                    // Nice handling of forced resizing beyond the possible:
                    content_clip_rect.max[d] = ui.clip_rect().max[d] - current_bar_use[d];
                }
            }
            // Make sure we din't accidentally expand the clip rect
            content_clip_rect = content_clip_rect.intersect(ui.clip_rect());
            content_ui.set_clip_rect(content_clip_rect);
        }

        let viewport = Rect::from_min_size(Pos2::ZERO + state.offset, inner_size);

        if (scrolling_enabled && drag_to_scroll)
            && (state.content_is_too_large[0] || state.content_is_too_large[1])
        {
            // Drag contents to scroll (for touch screens mostly).
            // We must do this BEFORE adding content to the `ScrollArea`,
            // or we will steal input from the widgets we contain.
            let content_response = ui.interact(inner_rect, id.with("area"), Sense::drag());

            if content_response.dragged() {
                for d in 0..2 {
                    if has_bar[d] {
                        ui.input(|input| {
                            state.offset[d] -= input.pointer.delta()[d];
                            state.vel[d] = input.pointer.velocity()[d];
                        });
                        state.scroll_stuck_to_end[d] = false;
                    } else {
                        state.vel[d] = 0.0;
                    }
                }
            } else {
                let stop_speed = 20.0; // Pixels per second.
                let friction_coeff = 1000.0; // Pixels per second squared.
                let dt = ui.input(|i| i.unstable_dt);

                let friction = friction_coeff * dt;
                if friction > state.vel.length() || state.vel.length() < stop_speed {
                    state.vel = Vec2::ZERO;
                } else {
                    state.vel -= friction * state.vel.normalized();
                    // Offset has an inverted coordinate system compared to
                    // the velocity, so we subtract it instead of adding it
                    state.offset -= state.vel * dt;
                    ui.ctx().request_repaint();
                }
            }
        }

        Prepared {
            id,
            state,
            has_bar,
            auto_shrink,
            current_bar_use,
            scroll_bar_visibility,
            inner_rect,
            content_ui,
            viewport,
            scrolling_enabled,
            stick_to_end,
        }
    }

    /// Show the [`ScrollArea`], and add the contents to the viewport.
    ///
    /// If the inner area can be very long, consider using [`Self::show_rows`] instead.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> ScrollAreaOutput<R> {
        self.show_viewport_dyn(ui, Box::new(|ui, _viewport| add_contents(ui)))
    }

    /// Efficiently show only the visible part of a large number of rows.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let text_style = egui::TextStyle::Body;
    /// let row_height = ui.text_style_height(&text_style);
    /// // let row_height = ui.spacing().interact_size.y; // if you are adding buttons instead of labels.
    /// let total_rows = 10_000;
    /// egui::ScrollArea::vertical().show_rows(ui, row_height, total_rows, |ui, row_range| {
    ///     for row in row_range {
    ///         let text = format!("Row {}/{}", row + 1, total_rows);
    ///         ui.label(text);
    ///     }
    /// });
    /// # });
    /// ```
    pub fn show_rows<R>(
        self,
        ui: &mut Ui,
        row_height_sans_spacing: f32,
        total_rows: usize,
        add_contents: impl FnOnce(&mut Ui, std::ops::Range<usize>) -> R,
    ) -> ScrollAreaOutput<R> {
        let spacing = ui.spacing().item_spacing;
        let row_height_with_spacing = row_height_sans_spacing + spacing.y;
        self.show_viewport(ui, |ui, viewport| {
            ui.set_height((row_height_with_spacing * total_rows as f32 - spacing.y).at_least(0.0));

            let mut min_row = (viewport.min.y / row_height_with_spacing).floor() as usize;
            let mut max_row = (viewport.max.y / row_height_with_spacing).ceil() as usize + 1;
            if max_row > total_rows {
                let diff = max_row.saturating_sub(min_row);
                max_row = total_rows;
                min_row = total_rows.saturating_sub(diff);
            }

            let y_min = ui.max_rect().top() + min_row as f32 * row_height_with_spacing;
            let y_max = ui.max_rect().top() + max_row as f32 * row_height_with_spacing;

            let rect = Rect::from_x_y_ranges(ui.max_rect().x_range(), y_min..=y_max);

            ui.allocate_ui_at_rect(rect, |viewport_ui| {
                viewport_ui.skip_ahead_auto_ids(min_row); // Make sure we get consistent IDs.
                add_contents(viewport_ui, min_row..max_row)
            })
            .inner
        })
    }

    /// This can be used to only paint the visible part of the contents.
    ///
    /// `add_contents` is given the viewport rectangle, which is the relative view of the content.
    /// So if the passed rect has min = zero, then show the top left content (the user has not scrolled).
    pub fn show_viewport<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui, Rect) -> R,
    ) -> ScrollAreaOutput<R> {
        self.show_viewport_dyn(ui, Box::new(add_contents))
    }

    fn show_viewport_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui, Rect) -> R + 'c>,
    ) -> ScrollAreaOutput<R> {
        let mut prepared = self.begin(ui);
        let id = prepared.id;
        let inner_rect = prepared.inner_rect;
        let inner = add_contents(&mut prepared.content_ui, prepared.viewport);
        let (content_size, state) = prepared.end(ui);
        ScrollAreaOutput {
            inner,
            id,
            state,
            content_size,
            inner_rect,
        }
    }
}

impl Prepared {
    /// Returns content size and state
    fn end(self, ui: &mut Ui) -> (Vec2, State) {
        let Prepared {
            id,
            mut state,
            inner_rect,
            has_bar,
            auto_shrink,
            mut current_bar_use,
            scroll_bar_visibility,
            content_ui,
            viewport: _,
            scrolling_enabled,
            stick_to_end,
        } = self;

        let content_size = content_ui.min_size();

        for d in 0..2 {
            if has_bar[d] {
                // We take the scroll target so only this ScrollArea will use it:
                let scroll_target = content_ui
                    .ctx()
                    .frame_state_mut(|state| state.scroll_target[d].take());
                if let Some((scroll, align)) = scroll_target {
                    let min = content_ui.min_rect().min[d];
                    let clip_rect = content_ui.clip_rect();
                    let visible_range = min..=min + clip_rect.size()[d];
                    let start = *scroll.start();
                    let end = *scroll.end();
                    let clip_start = clip_rect.min[d];
                    let clip_end = clip_rect.max[d];
                    let mut spacing = ui.spacing().item_spacing[d];

                    let delta = if let Some(align) = align {
                        let center_factor = align.to_factor();

                        let offset =
                            lerp(scroll, center_factor) - lerp(visible_range, center_factor);

                        // Depending on the alignment we need to add or subtract the spacing
                        spacing *= remap(center_factor, 0.0..=1.0, -1.0..=1.0);

                        offset + spacing - state.offset[d]
                    } else if start < clip_start && end < clip_end {
                        -(clip_start - start + spacing).min(clip_end - end - spacing)
                    } else if end > clip_end && start > clip_start {
                        (end - clip_end + spacing).min(start - clip_start - spacing)
                    } else {
                        // Ui is already in view, no need to adjust scroll.
                        0.0
                    };

                    if delta != 0.0 {
                        state.offset[d] += delta;
                        ui.ctx().request_repaint();
                    }
                }
            }
        }

        let inner_rect = {
            // At this point this is the available size for the inner rect.
            let mut inner_size = inner_rect.size();

            for d in 0..2 {
                inner_size[d] = match (has_bar[d], auto_shrink[d]) {
                    (true, true) => inner_size[d].min(content_size[d]), // shrink scroll area if content is small
                    (true, false) => inner_size[d], // let scroll area be larger than content; fill with blank space
                    (false, true) => content_size[d], // Follow the content (expand/contract to fit it).
                    (false, false) => inner_size[d].max(content_size[d]), // Expand to fit content
                };
            }

            Rect::from_min_size(inner_rect.min, inner_size)
        };

        let outer_rect = Rect::from_min_size(inner_rect.min, inner_rect.size() + current_bar_use);

        let content_is_too_large = [
            content_size.x > inner_rect.width(),
            content_size.y > inner_rect.height(),
        ];

        let max_offset = content_size - inner_rect.size();
        if scrolling_enabled && ui.rect_contains_pointer(outer_rect) {
            for d in 0..2 {
                if has_bar[d] {
                    let scroll_delta = ui.ctx().frame_state(|fs| fs.scroll_delta);

                    let scrolling_up = state.offset[d] > 0.0 && scroll_delta[d] > 0.0;
                    let scrolling_down = state.offset[d] < max_offset[d] && scroll_delta[d] < 0.0;

                    if scrolling_up || scrolling_down {
                        state.offset[d] -= scroll_delta[d];
                        // Clear scroll delta so no parent scroll will use it.
                        ui.ctx().frame_state_mut(|fs| fs.scroll_delta[d] = 0.0);
                        state.scroll_stuck_to_end[d] = false;
                    }
                }
            }
        }

        let show_scroll_this_frame = match scroll_bar_visibility {
            ScrollBarVisibility::AlwaysVisible => [true, true],
            ScrollBarVisibility::VisibleWhenNeeded => {
                [content_is_too_large[0], content_is_too_large[1]]
            }
            ScrollBarVisibility::AlwaysHidden => [false, false],
        };

        let max_scroll_bar_width = max_scroll_bar_width_with_margin(ui);

        // Avoid frame delay; start showing scroll bar right away:
        if show_scroll_this_frame[0] && current_bar_use.y <= 0.0 {
            current_bar_use.y = max_scroll_bar_width * ui.ctx().animate_bool(id.with("h"), true);
        }
        if show_scroll_this_frame[1] && current_bar_use.x <= 0.0 {
            current_bar_use.x = max_scroll_bar_width * ui.ctx().animate_bool(id.with("v"), true);
        }

        for d in 0..2 {
            let animation_t = current_bar_use[1 - d] / max_scroll_bar_width;

            if animation_t == 0.0 {
                continue;
            }

            // margin on either side of the scroll bar
            let inner_margin = animation_t * ui.spacing().scroll_bar_inner_margin;
            let outer_margin = animation_t * ui.spacing().scroll_bar_outer_margin;
            let mut min_cross = inner_rect.max[1 - d] + inner_margin; // left of vertical scroll (d == 1)
            let mut max_cross = outer_rect.max[1 - d] - outer_margin; // right of vertical scroll (d == 1)
            let min_main = inner_rect.min[d]; // top of vertical scroll (d == 1)
            let max_main = inner_rect.max[d]; // bottom of vertical scroll (d == 1)

            if ui.clip_rect().max[1 - d] < max_cross + outer_margin {
                // Move the scrollbar so it is visible. This is needed in some cases.
                // For instance:
                // * When we have a vertical-only scroll area in a top level panel,
                //   and that panel is not wide enough for the contents.
                // * When one ScrollArea is nested inside another, and the outer
                //   is scrolled so that the scroll-bars of the inner ScrollArea (us)
                //   is outside the clip rectangle.
                // Really this should use the tighter clip_rect that ignores clip_rect_margin, but we don't store that.
                // clip_rect_margin is quite a hack. It would be nice to get rid of it.
                let width = max_cross - min_cross;
                max_cross = ui.clip_rect().max[1 - d] - outer_margin;
                min_cross = max_cross - width;
            }

            let outer_scroll_rect = if d == 0 {
                Rect::from_min_max(
                    pos2(inner_rect.left(), min_cross),
                    pos2(inner_rect.right(), max_cross),
                )
            } else {
                Rect::from_min_max(
                    pos2(min_cross, inner_rect.top()),
                    pos2(max_cross, inner_rect.bottom()),
                )
            };

            // maybe force increase in offset to keep scroll stuck to end position
            if stick_to_end[d] && state.scroll_stuck_to_end[d] {
                state.offset[d] = content_size[d] - inner_rect.size()[d];
            }

            let from_content =
                |content| remap_clamp(content, 0.0..=content_size[d], min_main..=max_main);

            let handle_rect = if d == 0 {
                Rect::from_min_max(
                    pos2(from_content(state.offset.x), min_cross),
                    pos2(from_content(state.offset.x + inner_rect.width()), max_cross),
                )
            } else {
                Rect::from_min_max(
                    pos2(min_cross, from_content(state.offset.y)),
                    pos2(
                        max_cross,
                        from_content(state.offset.y + inner_rect.height()),
                    ),
                )
            };

            let interact_id = id.with(d);
            let sense = if self.scrolling_enabled {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            };
            let response = ui.interact(outer_scroll_rect, interact_id, sense);

            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let scroll_start_offset_from_top_left = state.scroll_start_offset_from_top_left[d]
                    .get_or_insert_with(|| {
                        if handle_rect.contains(pointer_pos) {
                            pointer_pos[d] - handle_rect.min[d]
                        } else {
                            let handle_top_pos_at_bottom = max_main - handle_rect.size()[d];
                            // Calculate the new handle top position, centering the handle on the mouse.
                            let new_handle_top_pos = (pointer_pos[d] - handle_rect.size()[d] / 2.0)
                                .clamp(min_main, handle_top_pos_at_bottom);
                            pointer_pos[d] - new_handle_top_pos
                        }
                    });

                let new_handle_top = pointer_pos[d] - *scroll_start_offset_from_top_left;
                state.offset[d] = remap(new_handle_top, min_main..=max_main, 0.0..=content_size[d]);

                // some manual action taken, scroll not stuck
                state.scroll_stuck_to_end[d] = false;
            } else {
                state.scroll_start_offset_from_top_left[d] = None;
            }

            let unbounded_offset = state.offset[d];
            state.offset[d] = state.offset[d].max(0.0);
            state.offset[d] = state.offset[d].min(max_offset[d]);

            if state.offset[d] != unbounded_offset {
                state.vel[d] = 0.0;
            }

            if ui.is_rect_visible(outer_scroll_rect) {
                // Avoid frame-delay by calculating a new handle rect:
                let mut handle_rect = if d == 0 {
                    Rect::from_min_max(
                        pos2(from_content(state.offset.x), min_cross),
                        pos2(from_content(state.offset.x + inner_rect.width()), max_cross),
                    )
                } else {
                    Rect::from_min_max(
                        pos2(min_cross, from_content(state.offset.y)),
                        pos2(
                            max_cross,
                            from_content(state.offset.y + inner_rect.height()),
                        ),
                    )
                };
                let min_handle_size = ui.spacing().scroll_handle_min_length;
                if handle_rect.size()[d] < min_handle_size {
                    handle_rect = Rect::from_center_size(
                        handle_rect.center(),
                        if d == 0 {
                            vec2(min_handle_size, handle_rect.size().y)
                        } else {
                            vec2(handle_rect.size().x, min_handle_size)
                        },
                    );
                }

                let visuals = if scrolling_enabled {
                    ui.style().interact(&response)
                } else {
                    &ui.style().visuals.widgets.inactive
                };

                ui.painter().add(epaint::Shape::rect_filled(
                    outer_scroll_rect,
                    visuals.rounding,
                    ui.visuals().extreme_bg_color,
                ));

                ui.painter().add(epaint::Shape::rect_filled(
                    handle_rect,
                    visuals.rounding,
                    visuals.bg_fill,
                ));
            }
        }

        ui.advance_cursor_after_rect(outer_rect);

        if show_scroll_this_frame != state.show_scroll {
            ui.ctx().request_repaint();
        }

        let available_offset = content_size - inner_rect.size();
        state.offset = state.offset.min(available_offset);
        state.offset = state.offset.max(Vec2::ZERO);

        // Is scroll handle at end of content, or is there no scrollbar
        // yet (not enough content), but sticking is requested? If so, enter sticky mode.
        // Only has an effect if stick_to_end is enabled but we save in
        // state anyway so that entering sticky mode at an arbitrary time
        // has appropriate effect.
        state.scroll_stuck_to_end = [
            (state.offset[0] == available_offset[0])
                || (self.stick_to_end[0] && available_offset[0] < 0.),
            (state.offset[1] == available_offset[1])
                || (self.stick_to_end[1] && available_offset[1] < 0.),
        ];

        state.show_scroll = show_scroll_this_frame;
        state.content_is_too_large = content_is_too_large;

        state.store(ui.ctx(), id);

        (content_size, state)
    }
}

/// Width of a vertical scrollbar, or height of a horizontal scroll bar
fn max_scroll_bar_width_with_margin(ui: &Ui) -> f32 {
    ui.spacing().scroll_bar_inner_margin
        + ui.spacing().scroll_bar_width
        + ui.spacing().scroll_bar_outer_margin
}
