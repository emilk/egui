//! See [`ScrollArea`] for docs.

#![expect(clippy::needless_range_loop)]

use std::ops::{Add, AddAssign, BitOr, BitOrAssign};

use emath::GuiRounding as _;
use epaint::Margin;

use crate::{
    Context, CursorIcon, Id, NumExt as _, Pos2, Rangef, Rect, Response, Sense, Ui, UiBuilder,
    UiKind, UiStackInfo, Vec2, Vec2b, WidgetInfo, emath, epaint, lerp, pass_state, pos2, remap,
    remap_clamp,
};

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct ScrollingToTarget {
    animation_time_span: (f64, f64),
    target_offset: f32,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct State {
    /// Positive offset means scrolling down/right
    pub offset: Vec2,

    /// If set, quickly but smoothly scroll to this target offset.
    offset_target: [Option<ScrollingToTarget>; 2],

    /// Were the scroll bars visible last frame?
    show_scroll: Vec2b,

    /// The content were to large to fit large frame.
    content_is_too_large: Vec2b,

    /// Did the user interact (hover or drag) the scroll bars last frame?
    scroll_bar_interaction: Vec2b,

    /// Momentum, used for kinetic scrolling
    #[cfg_attr(feature = "serde", serde(skip))]
    vel: Vec2,

    /// Mouse offset relative to the top of the handle when started moving the handle.
    scroll_start_offset_from_top_left: [Option<f32>; 2],

    /// Is the scroll sticky. This is true while scroll handle is in the end position
    /// and remains that way until the user moves the `scroll_handle`. Once unstuck (false)
    /// it remains false until the scroll touches the end position, which reenables stickiness.
    scroll_stuck_to_end: Vec2b,

    /// Area that can be dragged. This is the size of the content from the last frame.
    interact_rect: Option<Rect>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            offset: Vec2::ZERO,
            offset_target: Default::default(),
            show_scroll: Vec2b::FALSE,
            content_is_too_large: Vec2b::FALSE,
            scroll_bar_interaction: Vec2b::FALSE,
            vel: Vec2::ZERO,
            scroll_start_offset_from_top_left: [None; 2],
            scroll_stuck_to_end: Vec2b::TRUE,
            interact_rect: None,
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

    /// Get the current kinetic scrolling velocity.
    pub fn velocity(&self) -> Vec2 {
        self.vel
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ScrollBarVisibility {
    /// Hide scroll bar even if they are needed.
    ///
    /// You can still scroll, with the scroll-wheel
    /// and by dragging the contents, but there is no
    /// visual indication of how far you have scrolled.
    AlwaysHidden,

    /// Show scroll bars only when the content size exceeds the container,
    /// i.e. when there is any need to scroll.
    ///
    /// This is the default.
    VisibleWhenNeeded,

    /// Always show the scroll bar, even if the contents fit in the container
    /// and there is no need to scroll.
    AlwaysVisible,
}

impl Default for ScrollBarVisibility {
    #[inline]
    fn default() -> Self {
        Self::VisibleWhenNeeded
    }
}

impl ScrollBarVisibility {
    pub const ALL: [Self; 3] = [
        Self::AlwaysHidden,
        Self::VisibleWhenNeeded,
        Self::AlwaysVisible,
    ];
}

/// What is the source of scrolling for a [`ScrollArea`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ScrollSource {
    /// Scroll the area by dragging a scroll bar.
    ///
    /// By default the scroll bars remain visible to show current position.
    /// To hide them use [`ScrollArea::scroll_bar_visibility()`].
    pub scroll_bar: bool,

    /// Scroll the area by dragging the contents.
    pub drag: bool,

    /// Scroll the area by scrolling (or shift scrolling) the mouse wheel with
    /// the mouse cursor over the [`ScrollArea`].
    pub mouse_wheel: bool,
}

impl Default for ScrollSource {
    fn default() -> Self {
        Self::ALL
    }
}

impl ScrollSource {
    pub const NONE: Self = Self {
        scroll_bar: false,
        drag: false,
        mouse_wheel: false,
    };
    pub const ALL: Self = Self {
        scroll_bar: true,
        drag: true,
        mouse_wheel: true,
    };
    pub const SCROLL_BAR: Self = Self {
        scroll_bar: true,
        drag: false,
        mouse_wheel: false,
    };
    pub const DRAG: Self = Self {
        scroll_bar: false,
        drag: true,
        mouse_wheel: false,
    };
    pub const MOUSE_WHEEL: Self = Self {
        scroll_bar: false,
        drag: false,
        mouse_wheel: true,
    };

    /// Is everything disabled?
    #[inline]
    pub fn is_none(&self) -> bool {
        self == &Self::NONE
    }

    /// Is anything enabled?
    #[inline]
    pub fn any(&self) -> bool {
        self.scroll_bar | self.drag | self.mouse_wheel
    }

    /// Is everything enabled?
    #[inline]
    pub fn is_all(&self) -> bool {
        self.scroll_bar & self.drag & self.mouse_wheel
    }
}

impl BitOr for ScrollSource {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            scroll_bar: self.scroll_bar | rhs.scroll_bar,
            drag: self.drag | rhs.drag,
            mouse_wheel: self.mouse_wheel | rhs.mouse_wheel,
        }
    }
}

#[expect(clippy::suspicious_arithmetic_impl)]
impl Add for ScrollSource {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self | rhs
    }
}

impl BitOrAssign for ScrollSource {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl AddAssign for ScrollSource {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

/// Add vertical and/or horizontal scrolling to a contained [`Ui`].
///
/// By default, scroll bars only show up when needed, i.e. when the contents
/// is larger than the container.
/// This is controlled by [`Self::scroll_bar_visibility`].
///
/// There are two flavors of scroll areas: solid and floating.
/// Solid scroll bars use up space, reducing the amount of space available
/// to the contents. Floating scroll bars float on top of the contents, covering it.
/// You can change the scroll style by changing the [`crate::style::Spacing::scroll`].
///
/// ### Coordinate system
/// * content: size of contents (generally large; that's why we want scroll bars)
/// * outer: size of scroll area including scroll bar(s)
/// * inner: excluding scroll bar(s). The area we clip the contents to. Includes `content_margin`.
///
/// If the floating scroll bars settings is turned on then `inner == outer`.
///
/// ## Example
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::ScrollArea::vertical().show(ui, |ui| {
///     // Add a lot of widgets here.
/// });
/// # });
/// ```
///
/// You can scroll to an element using [`crate::Response::scroll_to_me`], [`Ui::scroll_to_cursor`] and [`Ui::scroll_to_rect`].
///
/// ## See also
/// If you want to allow zooming, use [`crate::Scene`].
#[derive(Clone, Debug)]
#[must_use = "You should call .show()"]
pub struct ScrollArea {
    /// Do we have horizontal/vertical scrolling enabled?
    direction_enabled: Vec2b,

    auto_shrink: Vec2b,
    max_size: Vec2,
    min_scrolled_size: Vec2,
    scroll_bar_visibility: ScrollBarVisibility,
    scroll_bar_rect: Option<Rect>,
    id_salt: Option<Id>,
    offset_x: Option<f32>,
    offset_y: Option<f32>,
    on_hover_cursor: Option<CursorIcon>,
    on_drag_cursor: Option<CursorIcon>,
    scroll_source: ScrollSource,
    wheel_scroll_multiplier: Vec2,

    content_margin: Option<Margin>,

    /// If true for vertical or horizontal the scroll wheel will stick to the
    /// end position until user manually changes position. It will become true
    /// again once scroll handle makes contact with end.
    stick_to_end: Vec2b,

    /// If false, `scroll_to_*` functions will not be animated
    animated: bool,
}

impl ScrollArea {
    /// Create a horizontal scroll area.
    #[inline]
    pub fn horizontal() -> Self {
        Self::new([true, false])
    }

    /// Create a vertical scroll area.
    #[inline]
    pub fn vertical() -> Self {
        Self::new([false, true])
    }

    /// Create a bi-directional (horizontal and vertical) scroll area.
    #[inline]
    pub fn both() -> Self {
        Self::new([true, true])
    }

    /// Create a scroll area where both direction of scrolling is disabled.
    /// It's unclear why you would want to do this.
    #[inline]
    pub fn neither() -> Self {
        Self::new([false, false])
    }

    /// Create a scroll area where you decide which axis has scrolling enabled.
    /// For instance, `ScrollArea::new([true, false])` enables horizontal scrolling.
    pub fn new(direction_enabled: impl Into<Vec2b>) -> Self {
        Self {
            direction_enabled: direction_enabled.into(),
            auto_shrink: Vec2b::TRUE,
            max_size: Vec2::INFINITY,
            min_scrolled_size: Vec2::splat(64.0),
            scroll_bar_visibility: Default::default(),
            scroll_bar_rect: None,
            id_salt: None,
            offset_x: None,
            offset_y: None,
            on_hover_cursor: None,
            on_drag_cursor: None,
            scroll_source: ScrollSource::default(),
            wheel_scroll_multiplier: Vec2::splat(1.0),
            content_margin: None,
            stick_to_end: Vec2b::FALSE,
            animated: true,
        }
    }

    /// The maximum width of the outer frame of the scroll area.
    ///
    /// Use `f32::INFINITY` if you want the scroll area to expand to fit the surrounding [`Ui`] (default).
    ///
    /// See also [`Self::auto_shrink`].
    #[inline]
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.max_size.x = max_width;
        self
    }

    /// The maximum height of the outer frame of the scroll area.
    ///
    /// Use `f32::INFINITY` if you want the scroll area to expand to fit the surrounding [`Ui`] (default).
    ///
    /// See also [`Self::auto_shrink`].
    #[inline]
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
    #[inline]
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
    #[inline]
    pub fn min_scrolled_height(mut self, min_scrolled_height: f32) -> Self {
        self.min_scrolled_size.y = min_scrolled_height;
        self
    }

    /// Set the visibility of both horizontal and vertical scroll bars.
    ///
    /// With `ScrollBarVisibility::VisibleWhenNeeded` (default), the scroll bar will be visible only when needed.
    #[inline]
    pub fn scroll_bar_visibility(mut self, scroll_bar_visibility: ScrollBarVisibility) -> Self {
        self.scroll_bar_visibility = scroll_bar_visibility;
        self
    }

    /// Specify within which screen-space rectangle to show the scroll bars.
    ///
    /// This can be used to move the scroll bars to a smaller region of the `ScrollArea`,
    /// for instance if you are painting a sticky header on top of it.
    #[inline]
    pub fn scroll_bar_rect(mut self, scroll_bar_rect: Rect) -> Self {
        self.scroll_bar_rect = Some(scroll_bar_rect);
        self
    }

    /// A source for the unique [`Id`], e.g. `.id_source("second_scroll_area")` or `.id_source(loop_index)`.
    #[inline]
    #[deprecated = "Renamed id_salt"]
    pub fn id_source(self, id_salt: impl std::hash::Hash) -> Self {
        self.id_salt(id_salt)
    }

    /// A source for the unique [`Id`], e.g. `.id_salt("second_scroll_area")` or `.id_salt(loop_index)`.
    #[inline]
    pub fn id_salt(mut self, id_salt: impl std::hash::Hash) -> Self {
        self.id_salt = Some(Id::new(id_salt));
        self
    }

    /// Set the horizontal and vertical scroll offset position.
    ///
    /// Positive offset means scrolling down/right.
    ///
    /// See also: [`Self::vertical_scroll_offset`], [`Self::horizontal_scroll_offset`],
    /// [`Ui::scroll_to_cursor`](crate::ui::Ui::scroll_to_cursor) and
    /// [`Response::scroll_to_me`](crate::Response::scroll_to_me)
    #[inline]
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
    #[inline]
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
    #[inline]
    pub fn horizontal_scroll_offset(mut self, offset: f32) -> Self {
        self.offset_x = Some(offset);
        self
    }

    /// Set the cursor used when the mouse pointer is hovering over the [`ScrollArea`].
    ///
    /// Only applies if [`Self::scroll_source()`] has set [`ScrollSource::drag`] to `true`.
    ///
    /// Any changes to the mouse cursor made within the contents of the [`ScrollArea`] will
    /// override this setting.
    #[inline]
    pub fn on_hover_cursor(mut self, cursor: CursorIcon) -> Self {
        self.on_hover_cursor = Some(cursor);
        self
    }

    /// Set the cursor used when the [`ScrollArea`] is being dragged.
    ///
    /// Only applies if [`Self::scroll_source()`] has set [`ScrollSource::drag`] to `true`.
    ///
    /// Any changes to the mouse cursor made within the contents of the [`ScrollArea`] will
    /// override this setting.
    #[inline]
    pub fn on_drag_cursor(mut self, cursor: CursorIcon) -> Self {
        self.on_drag_cursor = Some(cursor);
        self
    }

    /// Turn on/off scrolling on the horizontal axis.
    #[inline]
    pub fn hscroll(mut self, hscroll: bool) -> Self {
        self.direction_enabled[0] = hscroll;
        self
    }

    /// Turn on/off scrolling on the vertical axis.
    #[inline]
    pub fn vscroll(mut self, vscroll: bool) -> Self {
        self.direction_enabled[1] = vscroll;
        self
    }

    /// Turn on/off scrolling on the horizontal/vertical axes.
    ///
    /// You can pass in `false`, `true`, `[false, true]` etc.
    #[inline]
    pub fn scroll(mut self, direction_enabled: impl Into<Vec2b>) -> Self {
        self.direction_enabled = direction_enabled.into();
        self
    }

    /// Control the scrolling behavior.
    ///
    /// * If `true` (default), the scroll area will respond to user scrolling.
    /// * If `false`, the scroll area will not respond to user scrolling.
    ///
    /// This can be used, for example, to optionally freeze scrolling while the user
    /// is typing text in a [`crate::TextEdit`] widget contained within the scroll area.
    ///
    /// This controls both scrolling directions.
    #[deprecated = "Use `ScrollArea::scroll_source()"]
    #[inline]
    pub fn enable_scrolling(mut self, enable: bool) -> Self {
        self.scroll_source = if enable {
            ScrollSource::ALL
        } else {
            ScrollSource::NONE
        };
        self
    }

    /// Can the user drag the scroll area to scroll?
    ///
    /// This is useful for touch screens.
    ///
    /// If `true`, the [`ScrollArea`] will sense drags.
    ///
    /// Default: `true`.
    #[deprecated = "Use `ScrollArea::scroll_source()"]
    #[inline]
    pub fn drag_to_scroll(mut self, drag_to_scroll: bool) -> Self {
        self.scroll_source.drag = drag_to_scroll;
        self
    }

    /// What sources does the [`ScrollArea`] use for scrolling the contents.
    #[inline]
    pub fn scroll_source(mut self, scroll_source: ScrollSource) -> Self {
        self.scroll_source = scroll_source;
        self
    }

    /// The scroll amount caused by a mouse wheel scroll is multiplied by this amount.
    ///
    /// Independent for each scroll direction. Defaults to `Vec2{x: 1.0, y: 1.0}`.
    ///
    /// This can invert or effectively disable mouse scrolling.
    #[inline]
    pub fn wheel_scroll_multiplier(mut self, multiplier: Vec2) -> Self {
        self.wheel_scroll_multiplier = multiplier;
        self
    }

    /// For each axis, should the containing area shrink if the content is small?
    ///
    /// * If `true`, egui will add blank space outside the scroll area.
    /// * If `false`, egui will add blank space inside the scroll area.
    ///
    /// Default: `true`.
    #[inline]
    pub fn auto_shrink(mut self, auto_shrink: impl Into<Vec2b>) -> Self {
        self.auto_shrink = auto_shrink.into();
        self
    }

    /// Should the scroll area animate `scroll_to_*` functions?
    ///
    /// Default: `true`.
    #[inline]
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    /// Is any scrolling enabled?
    pub(crate) fn is_any_scroll_enabled(&self) -> bool {
        self.direction_enabled[0] || self.direction_enabled[1]
    }

    /// Extra margin added around the contents.
    ///
    /// The scroll bars will be either on top of this margin, or outside of it,
    /// depending on the value of [`crate::style::ScrollStyle::floating`].
    ///
    /// Default: [`crate::style::ScrollStyle::content_margin`].
    #[inline]
    pub fn content_margin(mut self, margin: impl Into<Margin>) -> Self {
        self.content_margin = Some(margin.into());
        self
    }

    /// The scroll handle will stick to the rightmost position even while the content size
    /// changes dynamically. This can be useful to simulate text scrollers coming in from right
    /// hand side. The scroll handle remains stuck until user manually changes position. Once "unstuck"
    /// it will remain focused on whatever content viewport the user left it on. If the scroll
    /// handle is dragged all the way to the right it will again become stuck and remain there
    /// until manually pulled from the end position.
    #[inline]
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
    #[inline]
    pub fn stick_to_bottom(mut self, stick: bool) -> Self {
        self.stick_to_end[1] = stick;
        self
    }
}

struct Prepared {
    id: Id,
    state: State,

    auto_shrink: Vec2b,

    /// Does this `ScrollArea` have horizontal/vertical scrolling enabled?
    direction_enabled: Vec2b,

    /// Smoothly interpolated boolean of whether or not to show the scroll bars.
    show_bars_factor: Vec2,

    /// How much horizontal and vertical space are used up by the
    /// width of the vertical bar, and the height of the horizontal bar?
    ///
    /// This is always zero for floating scroll bars.
    ///
    /// Note that this is a `yx` swizzling of [`Self::show_bars_factor`]
    /// times the maximum bar with.
    /// That's because horizontal scroll uses up vertical space,
    /// and vice versa.
    current_bar_use: Vec2,

    scroll_bar_visibility: ScrollBarVisibility,
    scroll_bar_rect: Option<Rect>,

    /// Where on the screen the content is (excludes scroll bars; includes `content_margin`).
    inner_rect: Rect,

    content_ui: Ui,

    /// Relative coordinates: the offset and size of the view of the inner UI.
    /// `viewport.min == ZERO` means we scrolled to the top.
    viewport: Rect,

    scroll_source: ScrollSource,
    wheel_scroll_multiplier: Vec2,
    stick_to_end: Vec2b,

    /// If there was a scroll target before the [`ScrollArea`] was added this frame, it's
    /// not for us to handle so we save it and restore it after this [`ScrollArea`] is done.
    saved_scroll_target: [Option<pass_state::ScrollTarget>; 2],

    /// The response from dragging the background (if enabled)
    background_drag_response: Option<Response>,

    animated: bool,
}

impl ScrollArea {
    fn begin(self, ui: &mut Ui) -> Prepared {
        let Self {
            direction_enabled,
            auto_shrink,
            max_size,
            min_scrolled_size,
            scroll_bar_visibility,
            scroll_bar_rect,
            id_salt,
            offset_x,
            offset_y,
            on_hover_cursor,
            on_drag_cursor,
            scroll_source,
            wheel_scroll_multiplier,
            content_margin: _, // Used elsewhere
            stick_to_end,
            animated,
        } = self;

        let ctx = ui.ctx().clone();

        let id_salt = id_salt.unwrap_or_else(|| Id::new("scroll_area"));
        let id = ui.make_persistent_id(id_salt);
        ctx.check_for_id_clash(
            id,
            Rect::from_min_size(ui.available_rect_before_wrap().min, Vec2::ZERO),
            "ScrollArea",
        );
        let mut state = State::load(&ctx, id).unwrap_or_default();

        state.offset.x = offset_x.unwrap_or(state.offset.x);
        state.offset.y = offset_y.unwrap_or(state.offset.y);

        let show_bars: Vec2b = match scroll_bar_visibility {
            ScrollBarVisibility::AlwaysHidden => Vec2b::FALSE,
            ScrollBarVisibility::VisibleWhenNeeded => state.show_scroll,
            ScrollBarVisibility::AlwaysVisible => direction_enabled,
        };

        let show_bars_factor = Vec2::new(
            ctx.animate_bool_responsive(id.with("h"), show_bars[0]),
            ctx.animate_bool_responsive(id.with("v"), show_bars[1]),
        );

        let current_bar_use = show_bars_factor.yx() * ui.spacing().scroll.allocated_width();

        let available_outer = ui.available_rect_before_wrap();

        let outer_size = available_outer.size().at_most(max_size);

        let inner_size = {
            let mut inner_size = outer_size - current_bar_use;

            // Don't go so far that we shrink to zero.
            // In particular, if we put a [`ScrollArea`] inside of a [`ScrollArea`], the inner
            // one shouldn't collapse into nothingness.
            // See https://github.com/emilk/egui/issues/1097
            for d in 0..2 {
                if direction_enabled[d] {
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
                if direction_enabled[d] {
                    content_max_size[d] = f32::INFINITY;
                }
            }
        }

        let content_max_rect = Rect::from_min_size(inner_rect.min - state.offset, content_max_size);

        // Round to pixels to avoid widgets appearing to "float" when scrolling fractional amounts:
        let content_max_rect = content_max_rect
            .round_to_pixels(ui.pixels_per_point())
            .round_ui();

        let mut content_ui = ui.new_child(
            UiBuilder::new()
                .ui_stack_info(UiStackInfo::new(UiKind::ScrollArea))
                .max_rect(content_max_rect),
        );

        {
            // Clip the content, but only when we really need to:
            let clip_rect_margin = ui.visuals().clip_rect_margin;
            let mut content_clip_rect = ui.clip_rect();
            for d in 0..2 {
                if direction_enabled[d] {
                    content_clip_rect.min[d] = inner_rect.min[d] - clip_rect_margin;
                    content_clip_rect.max[d] = inner_rect.max[d] + clip_rect_margin;
                } else {
                    // Nice handling of forced resizing beyond the possible:
                    content_clip_rect.max[d] = ui.clip_rect().max[d] - current_bar_use[d];
                }
            }
            // Make sure we didn't accidentally expand the clip rect
            content_clip_rect = content_clip_rect.intersect(ui.clip_rect());
            content_ui.set_clip_rect(content_clip_rect);
        }

        let viewport = Rect::from_min_size(Pos2::ZERO + state.offset, inner_size);
        let dt = ui.input(|i| i.stable_dt).at_most(0.1);

        let background_drag_response =
            if scroll_source.drag && ui.is_enabled() && state.content_is_too_large.any() {
                // Drag contents to scroll (for touch screens mostly).
                // We must do this BEFORE adding content to the `ScrollArea`,
                // or we will steal input from the widgets we contain.
                let content_response_option = state
                    .interact_rect
                    .map(|rect| ui.interact(rect, id.with("area"), Sense::DRAG));

                if content_response_option
                    .as_ref()
                    .is_some_and(|response| response.dragged())
                {
                    for d in 0..2 {
                        if direction_enabled[d] {
                            ui.input(|input| {
                                state.offset[d] -= input.pointer.delta()[d];
                            });
                            state.scroll_stuck_to_end[d] = false;
                            state.offset_target[d] = None;
                        }
                    }
                } else {
                    // Apply the cursor velocity to the scroll area when the user releases the drag.
                    if content_response_option
                        .as_ref()
                        .is_some_and(|response| response.drag_stopped())
                    {
                        state.vel = direction_enabled.to_vec2()
                            * ui.input(|input| input.pointer.velocity());
                    }
                    for d in 0..2 {
                        // Kinetic scrolling
                        let stop_speed = 20.0; // Pixels per second.
                        let friction_coeff = 1000.0; // Pixels per second squared.

                        let friction = friction_coeff * dt;
                        if friction > state.vel[d].abs() || state.vel[d].abs() < stop_speed {
                            state.vel[d] = 0.0;
                        } else {
                            state.vel[d] -= friction * state.vel[d].signum();
                            // Offset has an inverted coordinate system compared to
                            // the velocity, so we subtract it instead of adding it
                            state.offset[d] -= state.vel[d] * dt;
                            ctx.request_repaint();
                        }
                    }
                }

                // Set the desired mouse cursors.
                if let Some(response) = &content_response_option {
                    if response.dragged()
                        && let Some(cursor) = on_drag_cursor
                    {
                        ui.set_cursor_icon(cursor);
                    } else if response.hovered()
                        && let Some(cursor) = on_hover_cursor
                    {
                        ui.set_cursor_icon(cursor);
                    }
                }

                content_response_option
            } else {
                None
            };

        // Scroll with an animation if we have a target offset (that hasn't been cleared by the code
        // above).
        for d in 0..2 {
            if let Some(scroll_target) = state.offset_target[d] {
                state.vel[d] = 0.0;

                if (state.offset[d] - scroll_target.target_offset).abs() < 1.0 {
                    // Arrived
                    state.offset[d] = scroll_target.target_offset;
                    state.offset_target[d] = None;
                } else {
                    // Move towards target
                    let t = emath::interpolation_factor(
                        scroll_target.animation_time_span,
                        ui.input(|i| i.time),
                        dt,
                        emath::ease_in_ease_out,
                    );
                    if t < 1.0 {
                        state.offset[d] =
                            emath::lerp(state.offset[d]..=scroll_target.target_offset, t);
                        ctx.request_repaint();
                    } else {
                        // Arrived
                        state.offset[d] = scroll_target.target_offset;
                        state.offset_target[d] = None;
                    }
                }
            }
        }

        let saved_scroll_target = content_ui
            .ctx()
            .pass_state_mut(|state| std::mem::take(&mut state.scroll_target));

        Prepared {
            id,
            state,
            auto_shrink,
            direction_enabled,
            show_bars_factor,
            current_bar_use,
            scroll_bar_visibility,
            scroll_bar_rect,
            inner_rect,
            content_ui,
            viewport,
            scroll_source,
            wheel_scroll_multiplier,
            stick_to_end,
            saved_scroll_target,
            background_drag_response,
            animated,
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

            ui.scope_builder(UiBuilder::new().max_rect(rect), |viewport_ui| {
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
        let margin = self
            .content_margin
            .unwrap_or_else(|| ui.spacing().scroll.content_margin);

        let mut prepared = self.begin(ui);
        let id = prepared.id;
        let inner_rect = prepared.inner_rect;

        let inner = crate::Frame::NONE
            .inner_margin(margin)
            .show(&mut prepared.content_ui, |ui| {
                add_contents(ui, prepared.viewport)
            })
            .inner;

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
        let Self {
            id,
            mut state,
            inner_rect,
            auto_shrink,
            direction_enabled,
            mut show_bars_factor,
            current_bar_use,
            scroll_bar_visibility,
            scroll_bar_rect,
            content_ui,
            viewport: _,
            scroll_source,
            wheel_scroll_multiplier,
            stick_to_end,
            saved_scroll_target,
            background_drag_response,
            animated,
        } = self;

        let content_size = content_ui.min_size();

        let scroll_delta = content_ui
            .ctx()
            .pass_state_mut(|state| std::mem::take(&mut state.scroll_delta));

        for d in 0..2 {
            // PassState::scroll_delta is inverted from the way we apply the delta, so we need to negate it.
            let mut delta = -scroll_delta.0[d];
            let mut animation = scroll_delta.1;

            // We always take both scroll targets regardless of which scroll axes are enabled. This
            // is to avoid them leaking to other scroll areas.
            let scroll_target = content_ui
                .ctx()
                .pass_state_mut(|state| state.scroll_target[d].take());

            if direction_enabled[d] {
                if let Some(target) = scroll_target {
                    let pass_state::ScrollTarget {
                        range,
                        align,
                        animation: animation_update,
                    } = target;
                    let min = content_ui.min_rect().min[d];
                    let clip_rect = content_ui.clip_rect();
                    let visible_range = min..=min + clip_rect.size()[d];
                    let (start, end) = (range.min, range.max);
                    let clip_start = clip_rect.min[d];
                    let clip_end = clip_rect.max[d];
                    let mut spacing = content_ui.spacing().item_spacing[d];

                    let delta_update = if let Some(align) = align {
                        let center_factor = align.to_factor();

                        let offset =
                            lerp(range, center_factor) - lerp(visible_range, center_factor);

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

                    delta += delta_update;
                    animation = animation_update;
                }

                if delta != 0.0 {
                    let target_offset = state.offset[d] + delta;

                    if !animated {
                        state.offset[d] = target_offset;
                    } else if let Some(animation) = &mut state.offset_target[d] {
                        // For instance: the user is continuously calling `ui.scroll_to_cursor`,
                        // so we don't want to reset the animation, but perhaps update the target:
                        animation.target_offset = target_offset;
                    } else {
                        // The further we scroll, the more time we take.
                        let now = ui.input(|i| i.time);
                        let animation_duration = (delta.abs() / animation.points_per_second)
                            .clamp(animation.duration.min, animation.duration.max);
                        state.offset_target[d] = Some(ScrollingToTarget {
                            animation_time_span: (now, now + animation_duration as f64),
                            target_offset,
                        });
                    }
                    ui.request_repaint();
                }
            }
        }

        // Restore scroll target meant for ScrollAreas up the stack (if any)
        ui.ctx().pass_state_mut(|state| {
            for d in 0..2 {
                if saved_scroll_target[d].is_some() {
                    state.scroll_target[d] = saved_scroll_target[d].clone();
                }
            }
        });

        let inner_rect = {
            // At this point this is the available size for the inner rect.
            let mut inner_size = inner_rect.size();

            for d in 0..2 {
                inner_size[d] = match (direction_enabled[d], auto_shrink[d]) {
                    (true, true) => inner_size[d].min(content_size[d]), // shrink scroll area if content is small
                    (true, false) => inner_size[d], // let scroll area be larger than content; fill with blank space
                    (false, true) => content_size[d], // Follow the content (expand/contract to fit it).
                    (false, false) => inner_size[d].max(content_size[d]), // Expand to fit content
                };
            }

            Rect::from_min_size(inner_rect.min, inner_size)
        };

        let outer_rect = Rect::from_min_size(inner_rect.min, inner_rect.size() + current_bar_use);

        let content_is_too_large = Vec2b::new(
            direction_enabled[0] && inner_rect.width() < content_size.x,
            direction_enabled[1] && inner_rect.height() < content_size.y,
        );

        let max_offset = content_size - inner_rect.size();

        // Drag-to-scroll?
        let is_dragging_background = background_drag_response
            .as_ref()
            .is_some_and(|r| r.dragged());

        let is_hovering_outer_rect = ui.rect_contains_pointer(outer_rect)
            && ui.ctx().dragged_id().is_none()
            || is_dragging_background;

        if scroll_source.mouse_wheel && ui.is_enabled() && is_hovering_outer_rect {
            let always_scroll_enabled_direction = ui.style().always_scroll_the_only_direction
                && direction_enabled[0] != direction_enabled[1];
            for d in 0..2 {
                if direction_enabled[d] {
                    let scroll_delta = ui.input(|input| {
                        if always_scroll_enabled_direction {
                            // no bidirectional scrolling; allow horizontal scrolling without pressing shift
                            input.smooth_scroll_delta()[0] + input.smooth_scroll_delta()[1]
                        } else {
                            input.smooth_scroll_delta()[d]
                        }
                    });
                    let scroll_delta = scroll_delta * wheel_scroll_multiplier[d];

                    let scrolling_up = state.offset[d] > 0.0 && scroll_delta > 0.0;
                    let scrolling_down = state.offset[d] < max_offset[d] && scroll_delta < 0.0;

                    if scrolling_up || scrolling_down {
                        state.offset[d] -= scroll_delta;

                        // Clear scroll delta so no parent scroll will use it:
                        ui.input_mut(|input| {
                            if always_scroll_enabled_direction {
                                input.smooth_scroll_delta()[0] = 0.0;
                                input.smooth_scroll_delta()[1] = 0.0;
                            } else {
                                input.smooth_scroll_delta()[d] = 0.0;
                            }
                        });

                        state.scroll_stuck_to_end[d] = false;
                        state.offset_target[d] = None;
                    }
                }
            }
        }

        let show_scroll_this_frame = match scroll_bar_visibility {
            ScrollBarVisibility::AlwaysHidden => Vec2b::FALSE,
            ScrollBarVisibility::VisibleWhenNeeded => content_is_too_large,
            ScrollBarVisibility::AlwaysVisible => direction_enabled,
        };

        // Avoid frame delay; start showing scroll bar right away:
        if show_scroll_this_frame[0] && show_bars_factor.x <= 0.0 {
            show_bars_factor.x = ui.ctx().animate_bool_responsive(id.with("h"), true);
        }
        if show_scroll_this_frame[1] && show_bars_factor.y <= 0.0 {
            show_bars_factor.y = ui.ctx().animate_bool_responsive(id.with("v"), true);
        }

        let scroll_style = ui.spacing().scroll;

        // Paint the bars:
        let scroll_bar_rect = scroll_bar_rect.unwrap_or(inner_rect);
        for d in 0..2 {
            // maybe force increase in offset to keep scroll stuck to end position
            if stick_to_end[d] && state.scroll_stuck_to_end[d] {
                state.offset[d] = content_size[d] - inner_rect.size()[d];
            }

            let show_factor = show_bars_factor[d];
            if show_factor == 0.0 {
                state.scroll_bar_interaction[d] = false;
                continue;
            }

            let interact_id = id.with(d);

            // Margin on either side of the scroll bar:
            let inner_margin = show_factor * scroll_style.bar_inner_margin;
            let outer_margin = show_factor * scroll_style.bar_outer_margin;

            // bottom of a horizontal scroll (d==0).
            // right of a vertical scroll (d==1).
            let mut max_cross = outer_rect.max[1 - d] - outer_margin;

            if ui.clip_rect().max[1 - d] - outer_margin < max_cross {
                // Move the scrollbar so it is visible. This is needed in some cases.
                // For instance:
                // * When we have a vertical-only scroll area in a top level panel,
                //   and that panel is not wide enough for the contents.
                // * When one ScrollArea is nested inside another, and the outer
                //   is scrolled so that the scroll-bars of the inner ScrollArea (us)
                //   is outside the clip rectangle.
                // Really this should use the tighter clip_rect that ignores clip_rect_margin, but we don't store that.
                // clip_rect_margin is quite a hack. It would be nice to get rid of it.
                max_cross = ui.clip_rect().max[1 - d] - outer_margin;
            }

            let full_width = scroll_style.bar_width;

            // The bounding rect of a fully visible bar.
            // When we hover this area, we should show the full bar:
            let max_bar_rect = if d == 0 {
                outer_rect.with_min_y(max_cross - full_width)
            } else {
                outer_rect.with_min_x(max_cross - full_width)
            };

            let sense = if scroll_source.scroll_bar && ui.is_enabled() {
                Sense::CLICK | Sense::DRAG
            } else {
                Sense::hover()
            };

            // We always sense interaction with the full width, even if we antimate it growing/shrinking.
            // This is to present a more consistent target for our hit test code,
            // and to avoid producing jitter in "thin widget" heuristics there.
            // Also: it make sense to detect any hover where the scroll bar _will_ be.
            let response = ui.interact(max_bar_rect, interact_id, sense);

            response.widget_info(|| WidgetInfo::new(crate::WidgetType::ScrollBar));

            // top/bottom of a horizontal scroll (d==0).
            // left/rigth of a vertical scroll (d==1).
            let cross = if scroll_style.floating {
                let is_hovering_bar_area = response.hovered() || state.scroll_bar_interaction[d];

                let is_hovering_bar_area_t = ui
                    .ctx()
                    .animate_bool_responsive(id.with((d, "bar_hover")), is_hovering_bar_area);

                let width = show_factor
                    * lerp(
                        scroll_style.floating_width..=full_width,
                        is_hovering_bar_area_t,
                    );

                let min_cross = max_cross - width;
                Rangef::new(min_cross, max_cross)
            } else {
                let min_cross = inner_rect.max[1 - d] + inner_margin;
                Rangef::new(min_cross, max_cross)
            };

            let outer_scroll_bar_rect = if d == 0 {
                Rect::from_x_y_ranges(scroll_bar_rect.x_range(), cross)
            } else {
                Rect::from_x_y_ranges(cross, scroll_bar_rect.y_range())
            };

            let from_content = |content| {
                remap_clamp(
                    content,
                    0.0..=content_size[d],
                    scroll_bar_rect.min[d]..=scroll_bar_rect.max[d],
                )
            };

            let calculate_handle_rect = |d, offset: &Vec2| {
                let handle_size = if d == 0 {
                    from_content(offset.x + inner_rect.width()) - from_content(offset.x)
                } else {
                    from_content(offset.y + inner_rect.height()) - from_content(offset.y)
                }
                .max(scroll_style.handle_min_length);

                let handle_start_point = remap_clamp(
                    offset[d],
                    0.0..=max_offset[d],
                    scroll_bar_rect.min[d]..=(scroll_bar_rect.max[d] - handle_size),
                );

                if d == 0 {
                    Rect::from_min_max(
                        pos2(handle_start_point, cross.min),
                        pos2(handle_start_point + handle_size, cross.max),
                    )
                } else {
                    Rect::from_min_max(
                        pos2(cross.min, handle_start_point),
                        pos2(cross.max, handle_start_point + handle_size),
                    )
                }
            };

            let handle_rect = calculate_handle_rect(d, &state.offset);

            state.scroll_bar_interaction[d] = response.hovered() || response.dragged();

            if let Some(pointer_pos) = response.interact_pointer_pos() {
                let scroll_start_offset_from_top_left = state.scroll_start_offset_from_top_left[d]
                    .get_or_insert_with(|| {
                        if handle_rect.contains(pointer_pos) {
                            pointer_pos[d] - handle_rect.min[d]
                        } else {
                            let handle_top_pos_at_bottom =
                                scroll_bar_rect.max[d] - handle_rect.size()[d];
                            // Calculate the new handle top position, centering the handle on the mouse.
                            let new_handle_top_pos = (pointer_pos[d] - handle_rect.size()[d] / 2.0)
                                .clamp(scroll_bar_rect.min[d], handle_top_pos_at_bottom);
                            pointer_pos[d] - new_handle_top_pos
                        }
                    });

                let new_handle_top = pointer_pos[d] - *scroll_start_offset_from_top_left;
                let handle_travel =
                    scroll_bar_rect.min[d]..=(scroll_bar_rect.max[d] - handle_rect.size()[d]);
                state.offset[d] = if handle_travel.start() == handle_travel.end() {
                    0.0
                } else {
                    remap(new_handle_top, handle_travel, 0.0..=max_offset[d])
                };

                // some manual action taken, scroll not stuck
                state.scroll_stuck_to_end[d] = false;
                state.offset_target[d] = None;
            } else {
                state.scroll_start_offset_from_top_left[d] = None;
            }

            let unbounded_offset = state.offset[d];
            state.offset[d] = state.offset[d].max(0.0);
            state.offset[d] = state.offset[d].min(max_offset[d]);

            if state.offset[d] != unbounded_offset {
                state.vel[d] = 0.0;
            }

            if ui.is_rect_visible(outer_scroll_bar_rect) {
                // Avoid frame-delay by calculating a new handle rect:
                let handle_rect = calculate_handle_rect(d, &state.offset);

                let visuals = if scroll_source.scroll_bar && ui.is_enabled() {
                    // Pick visuals based on interaction with the handle.
                    // Remember that the response is for the whole scroll bar!
                    let is_hovering_handle = response.hovered()
                        && ui.input(|i| {
                            i.pointer
                                .latest_pos()
                                .is_some_and(|p| handle_rect.contains(p))
                        });
                    let visuals = ui.visuals();
                    if response.is_pointer_button_down_on() {
                        &visuals.widgets.active
                    } else if is_hovering_handle {
                        &visuals.widgets.hovered
                    } else {
                        &visuals.widgets.inactive
                    }
                } else {
                    &ui.visuals().widgets.inactive
                };

                let handle_opacity = if scroll_style.floating {
                    if response.hovered() || response.dragged() {
                        scroll_style.interact_handle_opacity
                    } else {
                        let is_hovering_outer_rect_t = ui.ctx().animate_bool_responsive(
                            id.with((d, "is_hovering_outer_rect")),
                            is_hovering_outer_rect,
                        );
                        lerp(
                            scroll_style.dormant_handle_opacity
                                ..=scroll_style.active_handle_opacity,
                            is_hovering_outer_rect_t,
                        )
                    }
                } else {
                    1.0
                };

                let background_opacity = if scroll_style.floating {
                    if response.hovered() || response.dragged() {
                        scroll_style.interact_background_opacity
                    } else if is_hovering_outer_rect {
                        scroll_style.active_background_opacity
                    } else {
                        scroll_style.dormant_background_opacity
                    }
                } else {
                    1.0
                };

                let handle_color = if scroll_style.foreground_color {
                    visuals.fg_stroke.color
                } else {
                    visuals.bg_fill
                };

                // Background:
                ui.painter().add(epaint::Shape::rect_filled(
                    outer_scroll_bar_rect,
                    visuals.corner_radius,
                    ui.visuals()
                        .extreme_bg_color
                        .gamma_multiply(background_opacity),
                ));

                // Handle:
                ui.painter().add(epaint::Shape::rect_filled(
                    handle_rect,
                    visuals.corner_radius,
                    handle_color.gamma_multiply(handle_opacity),
                ));
            }
        }

        ui.advance_cursor_after_rect(outer_rect);

        if show_scroll_this_frame != state.show_scroll {
            ui.request_repaint();
        }

        let available_offset = content_size - inner_rect.size();
        state.offset = state.offset.min(available_offset);
        state.offset = state.offset.max(Vec2::ZERO);

        // Is scroll handle at end of content, or is there no scrollbar
        // yet (not enough content), but sticking is requested? If so, enter sticky mode.
        // Only has an effect if stick_to_end is enabled but we save in
        // state anyway so that entering sticky mode at an arbitrary time
        // has appropriate effect.
        state.scroll_stuck_to_end = Vec2b::new(
            (state.offset[0] == available_offset[0])
                || (self.stick_to_end[0] && available_offset[0] < 0.0),
            (state.offset[1] == available_offset[1])
                || (self.stick_to_end[1] && available_offset[1] < 0.0),
        );

        state.show_scroll = show_scroll_this_frame;
        state.content_is_too_large = content_is_too_large;
        state.interact_rect = Some(inner_rect);

        state.store(ui.ctx(), id);

        (content_size, state)
    }
}
