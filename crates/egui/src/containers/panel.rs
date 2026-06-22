//! Panels are [`Ui`] regions taking up e.g. the left side of a [`Ui`] or screen.
//!
//! Panels can either be a child of a [`Ui`] (taking up a portion of the parent)
//! or be top-level (taking up a portion of the whole screen).
//!
//! Together with [`crate::Window`] and [`crate::Area`]:s, top-level panels are
//! the only places where you can put you widgets.
//!
//! The order in which you add panels matter!
//! The first panel you add will always be the outermost, and the last you add will always be the innermost.
//!
//! You must never open one top-level panel from within another panel. Add one panel, then the next.
//!
//! ⚠ Always add any [`CentralPanel`] last.
//!
//! Add your [`crate::Window`]:s after any top-level panels.

use emath::GuiRounding as _;

use crate::{
    Align, Context, CursorIcon, Frame, Id, InnerResponse, Layout, NumExt as _, Rangef, Rect,
    Response, Sense, Stroke, Ui, UiBuilder, UiKind, UiStackInfo, Vec2, lerp,
};

fn animate_expansion(ctx: &Context, id: Id, is_expanded: bool) -> f32 {
    ctx.animate_bool_responsive(id, is_expanded)
}

/// State regarding panels.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PanelState {
    /// The _outer_ rect of the panel, i.e. including the [`Frame`] margin & border.
    ///
    /// When animating, this will be a shifted in the animation direction,
    /// so it is really only the size that you can count on.
    #[cfg_attr(feature = "serde", serde(alias = "rect"))]
    pub outer_rect: Rect,
}

impl PanelState {
    pub fn load(ctx: &Context, bar_id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(bar_id))
    }

    /// The _outer_ size of the panel (from previous frame),
    /// i.e. including the [`Frame`] margin & border.
    pub fn size(&self) -> Vec2 {
        self.outer_rect.size()
    }

    fn store(self, ctx: &Context, bar_id: Id) {
        ctx.data_mut(|d| d.insert_persisted(bar_id, self));
    }
}

// ----------------------------------------------------------------------------

/// Which side of a [`Ui`] or screen the panel is attached to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanelSide {
    Left,
    Right,
    Top,
    Bottom,
}

impl PanelSide {
    /// The axis the panel grows along: `0` (x) for left/right panels,
    /// `1` (y) for top/bottom panels.
    ///
    /// Useful as an index into `Vec2`/`Pos2`.
    fn axis(self) -> usize {
        match self {
            Self::Left | Self::Right => 0,
            Self::Top | Self::Bottom => 1,
        }
    }

    /// The axis perpendicular to [`Self::axis`].
    fn cross_axis(self) -> usize {
        1 - self.axis()
    }

    /// Unit vector along [`Self::axis`]: `(1, 0)` for left/right, `(0, 1)` for top/bottom.
    fn axis_unit(self) -> Vec2 {
        match self {
            Self::Left | Self::Right => Vec2::X,
            Self::Top | Self::Bottom => Vec2::Y,
        }
    }

    /// Outward unit vector from the fixed edge:
    /// `(-1, 0)` for [`Left`](Self::Left), `(+1, 0)` for [`Right`](Self::Right),
    /// `(0, -1)` for [`Top`](Self::Top), `(0, +1)` for [`Bottom`](Self::Bottom).
    fn dir_vec2(self) -> Vec2 {
        self.sign() * self.axis_unit()
    }

    /// `-1` for sides at the near edge ([`Left`](Self::Left), [`Top`](Self::Top)),
    /// `+1` for sides at the far edge ([`Right`](Self::Right), [`Bottom`](Self::Bottom)).
    fn sign(self) -> f32 {
        match self {
            Self::Left | Self::Top => -1.0,
            Self::Right | Self::Bottom => 1.0,
        }
    }

    /// Coordinate of the _fixed_ side along the panel's [`axis`](Self::axis).
    fn fixed_pos(self, rect: Rect) -> f32 {
        match self {
            Self::Left => rect.left(),
            Self::Right => rect.right(),
            Self::Top => rect.top(),
            Self::Bottom => rect.bottom(),
        }
    }

    /// Coordinate of the _opposite_ (resizable) side along the panel's [`axis`](Self::axis).
    fn resize_pos(self, rect: Rect) -> f32 {
        match self {
            Self::Left => rect.right(),
            Self::Right => rect.left(),
            Self::Top => rect.bottom(),
            Self::Bottom => rect.top(),
        }
    }

    /// Resize by keeping `self` side fixed, and moving the opposite side.
    fn set_rect_size(self, rect: &mut Rect, size: f32) {
        match self {
            Self::Left => rect.max.x = rect.min.x + size,
            Self::Right => rect.min.x = rect.max.x - size,
            Self::Top => rect.max.y = rect.min.y + size,
            Self::Bottom => rect.min.y = rect.max.y - size,
        }
    }

    fn ui_kind(self) -> UiKind {
        match self {
            Self::Left => UiKind::LeftPanel,
            Self::Right => UiKind::RightPanel,
            Self::Top => UiKind::TopPanel,
            Self::Bottom => UiKind::BottomPanel,
        }
    }
}

// ----------------------------------------------------------------------------

/// A panel that covers an entire side
/// ([`left`](Panel::left), [`right`](Panel::right),
/// [`top`](Panel::top) or [`bottom`](Panel::bottom))
/// of a [`Ui`] or screen.
///
/// The order in which you add panels matter!
/// The first panel you add will always be the outermost, and the last you add will always be the innermost.
///
/// ⚠ Always add any [`CentralPanel`] last.
///
/// See the [module level docs](crate::containers::panel) for more details.
///
/// # Showing the panel
///
/// Pick the variant that matches the behavior you want:
///
/// * [`Panel::show`]: always show the panel.
/// * [`Panel::show_collapsible`]: show or hide the panel, with a slide animation in between.
/// * [`Panel::show_switched`]: animate between two different panels:
///   a thin/collapsed one and a thick/expanded one.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::Panel::left("my_left_panel").show(ui, |ui| {
///    ui.label("Hello World!");
/// });
/// # });
/// ```
#[must_use = "You should call .show()"]
pub struct Panel {
    side: PanelSide,
    id: Id,
    frame: Option<Frame>,
    resizable: bool,
    show_separator_line: bool,

    /// _Outer_ size (including [`Frame`] margin & border):
    /// the width for a vertical panel, or the height for a horizontal panel.
    default_outer_size: Option<f32>,

    /// _Outer_ size range (including [`Frame`] margin & border):
    /// the width for a vertical panel, or the height for a horizontal panel.
    outer_size_range: Rangef,

    /// `1.0` = panel fully visible (the normal case),
    /// `0.0` = panel fully slid off-screen toward its fixed edge.
    ///
    /// Used by [`Self::show_collapsible`] to animate a panel sliding in/out.
    /// While `slide_fraction != 1.0` the panel does _not_ persist its [`PanelState`].
    slide_fraction: f32,

    /// Override for the [`Id`] under which the resize-handle widget is registered.
    ///
    /// Used by [`Self::show_switched`] so the collapsed and
    /// expanded panels share a single resize widget — that way a drag on either
    /// one can flip `is_expanded` and the gesture survives the swap.
    resize_id_source: Option<Id>,

    /// Size below which drag-to-collapse fires, when set.
    ///
    /// Defaults to `outer_size_range.min`. Used by
    /// [`Self::show_switched`] to set the threshold at the
    /// collapsed panel's size, so the swap happens exactly when the slide
    /// matches the collapsed size visually.
    collapse_threshold: Option<f32>,
}

impl Panel {
    /// Create a left panel.
    ///
    /// The id should be globally unique, e.g. `Id::new("my_left_panel")`.
    pub fn left(id: impl Into<Id>) -> Self {
        Self::new(PanelSide::Left, id)
    }

    /// Create a right panel.
    ///
    /// The id should be globally unique, e.g. `Id::new("my_right_panel")`.
    pub fn right(id: impl Into<Id>) -> Self {
        Self::new(PanelSide::Right, id)
    }

    /// Create a top panel.
    ///
    /// The id should be globally unique, e.g. `Id::new("my_top_panel")`.
    ///
    /// By default this is NOT resizable.
    pub fn top(id: impl Into<Id>) -> Self {
        Self::new(PanelSide::Top, id).resizable(false)
    }

    /// Create a bottom panel.
    ///
    /// The id should be globally unique, e.g. `Id::new("my_bottom_panel")`.
    ///
    /// By default this is NOT resizable.
    pub fn bottom(id: impl Into<Id>) -> Self {
        Self::new(PanelSide::Bottom, id).resizable(false)
    }

    /// Create a panel.
    ///
    /// The id should be globally unique, e.g. `Id::new("my_panel")`.
    fn new(side: PanelSide, id: impl Into<Id>) -> Self {
        let default_outer_size: Option<f32> = match side {
            PanelSide::Left | PanelSide::Right => Some(200.0),
            PanelSide::Top | PanelSide::Bottom => None,
        };

        let outer_size_range: Rangef = match side {
            PanelSide::Left | PanelSide::Right => Rangef::new(96.0, f32::INFINITY),
            PanelSide::Top | PanelSide::Bottom => Rangef::new(20.0, f32::INFINITY),
        };

        Self {
            side,
            id: id.into(),
            frame: None,
            resizable: true,
            show_separator_line: true,
            default_outer_size,
            outer_size_range,
            slide_fraction: 1.0,
            resize_id_source: None,
            collapse_threshold: None,
        }
    }

    /// Can panel be resized by dragging the edge of it?
    ///
    /// Default is `true`.
    ///
    /// If you want your panel to be resizable you also need to make the ui use
    /// the available space.
    ///
    /// This can be done by using [`Ui::take_available_space`], or using a
    /// widget in it that takes up more space as you resize it, such as:
    /// * Wrapping text ([`Ui::horizontal_wrapped`]).
    /// * A [`crate::ScrollArea`].
    /// * A [`crate::Separator`].
    /// * A [`crate::TextEdit`].
    /// * …
    /// If you don't provide an expandable widget, the resize behavior is
    /// undefined.
    #[inline]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Show a separator line, even when not interacting with it?
    ///
    /// Default: `true`.
    #[inline]
    pub fn show_separator_line(mut self, show_separator_line: bool) -> Self {
        self.show_separator_line = show_separator_line;
        self
    }

    /// The initial wrapping width of the [`Panel`], including margins.
    #[inline]
    pub fn default_size(mut self, default_size: f32) -> Self {
        self.default_outer_size = Some(default_size);
        self.outer_size_range = Rangef::new(
            self.outer_size_range.min.at_most(default_size),
            self.outer_size_range.max.at_least(default_size),
        );
        self
    }

    /// Minimum size of the panel, including margins.
    #[inline]
    pub fn min_size(mut self, min_size: f32) -> Self {
        self.outer_size_range = Rangef::new(min_size, self.outer_size_range.max.at_least(min_size));
        self
    }

    /// Maximum size of the panel, including margins.
    #[inline]
    pub fn max_size(mut self, max_size: f32) -> Self {
        self.outer_size_range = Rangef::new(self.outer_size_range.min.at_most(max_size), max_size);
        self
    }

    /// The allowable size range for the panel, including margins.
    #[inline]
    pub fn size_range(mut self, size_range: impl Into<Rangef>) -> Self {
        let size_range = size_range.into();
        self.default_outer_size = self
            .default_outer_size
            .map(|default_size| clamp_to_range(default_size, size_range));
        self.outer_size_range = size_range;
        self
    }

    /// Enforce this exact size, including margins.
    #[inline]
    pub fn exact_size(mut self, size: f32) -> Self {
        self.default_outer_size = Some(size);
        self.outer_size_range = Rangef::point(size);
        self
    }

    /// Change the background color, margins, etc.
    #[inline]
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }
}

// Public showing methods
impl Panel {
    /// Show the panel inside a [`Ui`].
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        self.show_inside_dyn(ui, None, Box::new(add_contents))
    }

    /// Renamed to [`Self::show`].
    #[deprecated = "Renamed to `show`"]
    pub fn show_inside<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.show(ui, add_contents)
    }

    /// Show the panel if `*is_expanded` is `true`,
    /// otherwise hide it, with a slide animation in between.
    ///
    /// During the animation `add_contents` runs against the real panel, and the
    /// panel slides off-screen toward its fixed edge (clipped against the parent).
    /// The parent only reserves the _visible_ portion, so neighboring widgets follow.
    ///
    /// `is_expanded` is taken by `&mut` so the panel can flip it to `false` when
    /// the user drags the resize handle past the panel's minimum size, and back
    /// to `true` if the user drags the handle outward while the panel is closed.
    /// When [`Self::resizable`] is `true`, double-clicking the resize edge also
    /// flips `*is_expanded`.
    pub fn show_collapsible<R>(
        self,
        ui: &mut Ui,
        is_expanded: &mut bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let how_expanded = animate_expansion(ui, self.id.with("animation"), *is_expanded);

        if how_expanded == 0.0 {
            // Panel is fully closed. If the user is still dragging the resize handle
            // from a previous frame, keep its widget id alive so they can drag the
            // panel back out without releasing.
            self.keep_drag_alive_for_reopen(ui, is_expanded);

            // Make sure the ids of the next widgets are the same whether we show the panel or not:
            ui.skip_ahead_auto_ids(1);
            return None;
        }

        // Don't lose the drag during the slide-back-open animation:
        let drag_in_progress = ui
            .read_response(self.id.with("__resize"))
            .is_some_and(|r| r.dragged());

        let panel = if how_expanded < 1.0 {
            if drag_in_progress {
                // Mid-animation but the user is dragging — keep resize live so the
                // drag-to-reopen gesture flows straight into a normal resize.
                self.with_slide_fraction(how_expanded)
            } else {
                self.with_slide_fraction(how_expanded).resizable(false) // avoid flicker when the handle moved under the pointer during the animation
            }
        } else {
            self
        };

        Some(panel.show_inside_dyn(ui, Some(is_expanded), Box::new(add_contents)))
    }

    /// Renamed to [`Self::show_collapsible`].
    ///
    /// Note: [`Self::show_collapsible`] takes `is_expanded` by `&mut` so it can
    /// flip it to `false` when the user drags the panel closed. To opt in,
    /// migrate to the new name.
    #[deprecated = "Renamed to `show_collapsible`"]
    pub fn show_animated_inside<R>(
        self,
        ui: &mut Ui,
        mut is_expanded: bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        self.show_collapsible(ui, &mut is_expanded, add_contents)
    }

    /// Show either a collapsed or expanded panel, with a nice slide animation between.
    ///
    /// The `collapsed_panel` is shown only when fully collapsed; during the
    /// animation, the `expanded_panel` slides in/out toward its fixed edge,
    /// interpolating its visible size between the two panels' sizes.
    /// `add_contents` receives `expanded = true` whenever the expanded panel is
    /// rendered (including mid-animation), and `false` for the collapsed view.
    ///
    /// **Give the two panels distinct ids** so their persisted sizes don't
    /// overwrite each other.
    ///
    /// # Drag-to-collapse / drag-to-expand
    ///
    /// The user can resize the panel by dragging its edge. Pulling that edge
    /// past the size limits flips `*is_expanded`:
    ///
    /// * `.resizable(true)` on the **expanded** panel enables **drag-to-collapse**:
    ///   shrinking past `min_size` sets `*is_expanded = false`.
    /// * `.resizable(true)` on the **collapsed** panel enables **drag-to-expand**:
    ///   growing past `max_size` sets `*is_expanded = true`. (Use
    ///   [`Self::exact_size`] or [`Self::max_size`] to set a tight cap so a small
    ///   outward drag is enough to trigger the swap.)
    ///
    /// Both panels share a single resize-handle widget under the hood (keyed to
    /// the expanded panel's id), so a single uninterrupted drag can collapse and
    /// re-expand the panel without releasing.
    ///
    /// Double-clicking the resize edge also flips `*is_expanded` (whichever
    /// panel is currently shown is the one whose edge you click).
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let mut is_expanded = true;
    /// // `.resizable(true)` on both panels enables drag-to-collapse + drag-to-expand:
    /// let collapsed = egui::Panel::top("top_collapsed")
    ///     .resizable(true)
    ///     .default_size(20.0);
    /// let expanded = egui::Panel::top("top_expanded")
    ///     .resizable(true)
    ///     .default_size(120.0);
    /// egui::Panel::show_switched(
    ///     ui,
    ///     &mut is_expanded,
    ///     collapsed,
    ///     expanded,
    ///     |ui, expanded| {
    ///         if expanded {
    ///             ui.heading("Expanded");
    ///             ui.label("More content here…");
    ///         } else {
    ///             ui.label("Collapsed toolbar");
    ///         }
    ///     },
    /// );
    /// ui.toggle_value(&mut is_expanded, "Expand");
    /// # });
    /// ```
    pub fn show_switched<R>(
        ui: &mut Ui,
        is_expanded: &mut bool,
        collapsed_panel: Self,
        expanded_panel: Self,
        add_contents: impl FnOnce(&mut Ui, bool) -> R,
    ) -> InnerResponse<R> {
        debug_assert!(
            collapsed_panel.id != expanded_panel.id,
            "show_switched: the collapsed and expanded panels must have distinct ids \
             (their persisted sizes are stored per-id, and sharing one id would let the collapsed \
             size overwrite the expanded size)."
        );
        // Share one resize-handle widget across the collapsed and expanded panels
        // by routing both through the expanded panel's id. A drag that starts on
        // either panel survives the swap to the other view.
        let resize_id_source = expanded_panel.id;
        // Drag-to-collapse fires when the drag crosses the collapsed panel's
        // size, so the swap lines up with the visual size at that moment.
        let collapse_threshold = collapsed_panel.outer_size(ui);

        // Is the resize handle currently being dragged?
        let drag_in_progress = ui
            .read_response(resize_id_source.with("__resize"))
            .is_some_and(|r| r.dragged());

        let animation_id = expanded_panel.id.with("animation");
        // While the user is dragging, snap the animation to the target so the
        // drag (which sets `outer_size` directly from the pointer) doesn't fight
        // a simultaneous slide. Without this, drag-to-expand visibly jumps as
        // the slide animation tries to grow from 0 while the pointer is already
        // at the expanded size.
        let how_expanded = if drag_in_progress {
            ui.animate_bool_with_time(animation_id, *is_expanded, 0.0)
        } else {
            animate_expansion(ui, animation_id, *is_expanded)
        };

        // When expanding, the user sees the expanded content the moment animation starts.
        // When collapsing, keep showing the expanded content until past the midpoint,
        // then swap to the collapsed content for the rest of the slide-out.
        let show_expanded_contents = *is_expanded || 0.5 < how_expanded;

        if how_expanded == 0.0 {
            // Fully collapsed. The collapsed panel registers the shared resize
            // widget so drag-to-expand works, and `is_expanded` is flipped to
            // `true` when the user drags past its `max_size`.
            collapsed_panel
                .with_resize_id_source(resize_id_source)
                .show_inside_dyn(
                    ui,
                    Some(is_expanded),
                    Box::new(|ui| add_contents(ui, false)),
                )
        } else {
            let expanded_panel = expanded_panel.with_collapse_threshold(collapse_threshold);
            let panel = if how_expanded < 1.0 {
                // Animate the visible size from collapsed_size to expanded_size,
                // so the slide picks up where the collapsed panel left off.
                let expanded_size = expanded_panel.outer_size(ui);
                let visible_size = lerp(collapse_threshold..=expanded_size, how_expanded);
                let slide_fraction = if 0.0 < expanded_size {
                    visible_size / expanded_size
                } else {
                    1.0
                };
                let panel = expanded_panel.with_slide_fraction(slide_fraction);
                // Keep the resize handle live during the slide if the drag is
                // ongoing — otherwise disabling it would kill the gesture.
                if drag_in_progress {
                    panel
                } else {
                    panel.resizable(false) // avoid flicker when the handle moved under the pointer during the animation
                }
            } else {
                expanded_panel
            };
            // Pass `is_expanded` so dragging the resize handle past the
            // collapsed panel's size collapses to `collapsed_panel`.
            panel.show_inside_dyn(
                ui,
                Some(is_expanded),
                Box::new(|ui| add_contents(ui, show_expanded_contents)),
            )
        }
    }

    /// Renamed to [`Self::show_switched`].
    ///
    /// Note: [`Self::show_switched`] takes `is_expanded` by `&mut` (to allow
    /// drag-to-collapse / drag-to-expand to flip it) and passes a `bool` to
    /// `add_contents` instead of an `f32` animation fraction. To opt in,
    /// migrate to the new name.
    #[deprecated = "Renamed to `show_switched`"]
    pub fn show_animated_between_inside<R>(
        ui: &mut Ui,
        is_expanded: bool,
        collapsed_panel: Self,
        expanded_panel: Self,
        add_contents: impl FnOnce(&mut Ui, f32) -> R,
    ) -> InnerResponse<R> {
        let mut is_expanded = is_expanded;
        Self::show_switched(
            ui,
            &mut is_expanded,
            collapsed_panel,
            expanded_panel,
            |ui, expanded| add_contents(ui, if expanded { 1.0 } else { 0.0 }),
        )
    }
}

// Private methods to support the various show methods
impl Panel {
    /// Show the panel inside a [`Ui`].
    ///
    /// `is_expanded` is `Some` for the animated entry points
    /// ([`Self::show_collapsible`], [`Self::show_switched`]);
    /// when present, dragging the resize handle past the minimum size collapses
    /// the panel by setting `*is_expanded = false`.
    fn show_inside_dyn<'c, R>(
        mut self,
        parent_ui: &mut Ui,
        mut is_expanded: Option<&mut bool>,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let side = self.side;
        let id = self.id;
        let resizable = self.resizable;
        let show_separator_line = self.show_separator_line;

        let available_rect = parent_ui.available_rect_before_wrap();

        {
            // Never overflow out parent's available width:
            self.outer_size_range = self.outer_size_range.as_positive();
            self.outer_size_range.max = f32::min(
                self.outer_size_range.max,
                available_rect.size_along(side.axis()),
            );
        }

        let frame = self.resolve_frame(parent_ui);

        // We are NEVER allowed to overflow over this.
        // If we do, we do so by clipping the contents,
        // without reporting that extra size to the parent!
        let max_rect = {
            let mut max_rect = available_rect;
            self.side
                .set_rect_size(&mut max_rect, self.outer_size_range.max);
            max_rect
        };

        let mut outer_size = self
            .outer_size(parent_ui)
            .at_most(available_rect.size_along(self.side.axis()));

        let mut outer_rect = {
            let mut outer_rect = available_rect;
            self.side.set_rect_size(&mut outer_rect, outer_size);
            outer_rect
        };

        // Check for duplicate id
        parent_ui.check_for_id_clash(id, outer_rect, "Panel");

        // True iff the user is currently dragging the resize handle (set in the block below).
        let mut resize_drag_in_progress = false;

        if resizable {
            // Resolve the resize interaction first to avoid frame latency in the resize.
            // We also recompute the size on the release frame (`drag_stopped`) so the
            // released size gets persisted into [`PanelState`] — without this the
            // store-skipped-during-drag rule would leave the stored size at the
            // pre-drag value.
            let resize_id = self.resize_id_source.unwrap_or(id).with("__resize");
            let resize_response = parent_ui.read_response(resize_id);

            // Double-click on the resize edge toggles `*is_expanded` for the
            // animated entry points (`show_collapsible` / `show_switched`).
            if let Some(resize_response) = resize_response.as_ref()
                && resize_response.double_clicked()
                && let Some(is_expanded) = is_expanded.as_deref_mut()
            {
                *is_expanded = !*is_expanded;
            }

            if let Some(resize_response) = resize_response
                && (resize_response.dragged() || resize_response.drag_stopped())
                && let Some(pointer) = resize_response.interact_pointer_pos()
            {
                resize_drag_in_progress = resize_response.dragged();
                let axis = side.axis();
                let prev_outer_size = outer_size;
                // Signed distance from the fixed edge to the pointer along the
                // panel's axis. Going past the fixed edge yields a negative size,
                // which `clamp_to_range` then snaps up to `min` — DON'T use
                // `.abs()` here, that would mirror the drag and spuriously
                // trigger drag-to-expand once the pointer crosses the edge.
                let raw_outer_size = -side.sign() * (pointer[axis] - side.fixed_pos(outer_rect));
                outer_size = clamp_to_range(raw_outer_size, self.outer_size_range)
                    .at_most(available_rect.size_along(axis));
                side.set_rect_size(&mut outer_rect, outer_size);

                if let Some(is_expanded) = is_expanded {
                    // Drag-to-collapse: shrink past the threshold → close.
                    // The threshold defaults to `min_size`, but
                    // `show_switched` overrides it to the
                    // collapsed panel's size so the swap happens exactly when
                    // the drag visually crosses the collapsed size.
                    // Use `raw_outer_size` (pre-clamp) so a tight `exact_size`
                    // panel can still detect inward overshoot.
                    let collapse_threshold =
                        self.collapse_threshold.unwrap_or(self.outer_size_range.min);
                    if raw_outer_size < collapse_threshold && raw_outer_size < prev_outer_size {
                        *is_expanded = false;
                    }
                    // Drag-to-expand: pointer pulled outward past `max_size` → open.
                    // Triggers when this panel is acting as the collapsed view of
                    // `show_switched`, with `resize_id_source` set
                    // to the expanded panel's id. `raw_outer_size` is required
                    // because `outer_size` is clamped to `max` and would never
                    // exceed it (so `exact_size` panels couldn't otherwise expand).
                    if self.outer_size_range.max < raw_outer_size {
                        *is_expanded = true;
                    }
                }
            }
        }

        // NOTE(shark98): This must be **after** the resizable preparation, as the size
        // may change and round_ui() uses the size.
        outer_rect = outer_rect.round_ui();

        // Slide animation: translate the panel off-screen toward its fixed edge.
        // When `slide_fraction == 1.0` this is a no-op.
        let slide_distance = (1.0 - self.slide_fraction) * outer_size;
        let shifted_outer_rect = if slide_distance == 0.0 {
            outer_rect
        } else {
            outer_rect
                .translate(slide_distance * side.dir_vec2())
                .round_ui()
        };

        // The portion of the panel actually visible inside the parent's available area.
        // The parent only allocates this much; neighbors follow the slide.
        let visible_outer_rect = shifted_outer_rect.intersect(max_rect);

        let mut panel_ui = parent_ui.new_child(
            UiBuilder::new()
                .id_salt(id)
                .ui_stack_info(UiStackInfo::new(side.ui_kind()))
                .max_rect(shifted_outer_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        panel_ui.expand_to_include_rect(shifted_outer_rect);
        panel_ui.set_clip_rect(visible_outer_rect); // Hides the off-screen part during a slide; also prevents overflow (#4475).

        let axis = side.axis();
        let panel_axis_min =
            (self.outer_size_range.min - frame.total_margin().sum()[axis]).at_least(0.0);
        let mut inner_response = frame.show(&mut panel_ui, |content_ui| {
            // Make sure the frame fills the cross-axis fully:
            let cross_axis_size = content_ui.max_rect().size_along(side.cross_axis());
            if axis == 0 {
                content_ui.set_min_height(cross_axis_size);
                content_ui.set_min_width(panel_axis_min);
            } else {
                content_ui.set_min_width(cross_axis_size);
                content_ui.set_min_height(panel_axis_min);
            }

            add_contents(content_ui)
        });

        if self.outer_size_range.max < inner_response.response.rect.size_along(axis) {
            self.side
                .set_rect_size(&mut inner_response.response.rect, self.outer_size_range.max);
        }

        // `Frame::show` returns the panel's (shifted) _outer_ rect, including margin & border.
        let shifted_outer_rect = inner_response.response.rect;
        let visible_outer_rect = shifted_outer_rect.intersect(max_rect);

        {
            let mut cursor = parent_ui.cursor();
            match side {
                PanelSide::Left | PanelSide::Top => {
                    cursor.min[axis] = visible_outer_rect.max[axis];
                }
                PanelSide::Right | PanelSide::Bottom => {
                    cursor.max[axis] = visible_outer_rect.min[axis];
                }
            }
            parent_ui.set_cursor(cursor);
        }

        parent_ui.expand_to_include_rect(visible_outer_rect);

        let (resize_hover, is_resizing) = if resizable {
            // Now we do the actual resize interaction, on top of all the contents,
            // otherwise its input could be eaten by the contents, e.g. a
            // `ScrollArea` on either side of the panel boundary.
            let resize_response = self.resize_panel(shifted_outer_rect, parent_ui);
            (resize_response.hovered(), resize_response.dragged())
        } else {
            (false, false)
        };

        if resize_hover || is_resizing {
            parent_ui.set_cursor_icon(self.cursor_icon(outer_size));
        }

        let is_animating = 0.0 < self.slide_fraction && self.slide_fraction < 1.0;
        if !resize_drag_in_progress && !is_animating || PanelState::load(parent_ui, id).is_none() {
            // We skip stoing state during a drag, so that the
            // stored size reflects the panel's pre-drag size.
            // This is so that drag-to-close followed by a drag-to-reopen restores the original size.

            // Skipping when `!persist_state` keeps interpolated sizes (set by the
            // collapse animation in `show_switched`) from polluting the panel's
            // natural persisted size.

            // Finally, we always store the state if it's not already stored,
            // so we get a good estimate for the final size when first expanding a panel.

            PanelState {
                outer_rect: shifted_outer_rect,
            }
            .store(parent_ui, id);
        }

        // Hide the separator once the panel is mostly slid off — at that point
        // the line would just be a stray dash hovering near the parent edge.
        if 0.01 < self.slide_fraction {
            let stroke = if is_resizing {
                parent_ui.style().visuals.widgets.active.fg_stroke // highly visible
            } else if resize_hover {
                parent_ui.style().visuals.widgets.hovered.fg_stroke // highly visible
            } else if show_separator_line {
                // TODO(emilk): distinguish resizable from non-resizable
                parent_ui.style().visuals.widgets.noninteractive.bg_stroke // dim
            } else {
                Stroke::NONE
            };
            // TODO(emilk): draw line on top of all panels in this ui when https://github.com/emilk/egui/issues/1516 is done
            let line_pos = side.resize_pos(shifted_outer_rect) + 0.5 * side.sign() * stroke.width;
            let cross_range = shifted_outer_rect.range_along(side.cross_axis());
            if axis == 0 {
                parent_ui.painter().vline(line_pos, cross_range, stroke);
            } else {
                parent_ui.painter().hline(cross_range, line_pos, stroke);
            }
        }

        inner_response
    }

    /// The configured [`Frame`], or the default side/top panel frame for this [`Ui`].
    fn resolve_frame(&self, ui: &Ui) -> Frame {
        self.frame
            .unwrap_or_else(|| Frame::side_top_panel(ui.style()))
    }

    /// Panel is fully closed. If the user is still dragging the resize handle
    /// from the frame the panel closed on, keep its widget id registered so the
    /// drag survives, and reopen if they drag back past the minimum size.
    fn keep_drag_alive_for_reopen(&self, ui: &Ui, is_expanded: &mut bool) {
        let resize_id = self.id.with("__resize");
        let Some(resize_response) = ui.read_response(resize_id) else {
            return;
        };
        if !resize_response.dragged() {
            return;
        }
        let Some(pointer) = resize_response.interact_pointer_pos() else {
            return;
        };

        // Re-register the resize widget at the (now collapsed) fixed edge so its
        // id stays alive in egui's interaction state.
        let available_rect = ui.available_rect_before_wrap();
        let fixed_edge_pos = self.side.fixed_pos(available_rect);
        let cross_range = available_rect.range_along(self.side.cross_axis());
        let resize_rect = if self.side.axis() == 0 {
            Rect::from_x_y_ranges(Rangef::point(fixed_edge_pos), cross_range)
        } else {
            Rect::from_x_y_ranges(cross_range, Rangef::point(fixed_edge_pos))
        };
        let grab = ui.style().interaction.resize_grab_radius_side;
        let resize_rect = resize_rect.expand2(grab * self.side.axis_unit());
        ui.interact(resize_rect, resize_id, Sense::drag());

        // Keep the resize cursor while the user is still holding the drag.
        // Otherwise the cursor would snap back to the default the moment the
        // panel closed, even though the gesture is still ongoing.
        ui.set_cursor_icon(self.cursor_icon(0.0));

        // Signed distance from the fixed edge to the pointer along the panel's
        // axis. Only counts as "pulled outward" while positive — going past the
        // fixed edge gives a negative value, NOT a mirrored positive one (no
        // `.abs()`), so dragging past the screen edge can't spuriously reopen.
        let dragged_size = -self.side.sign() * (pointer[self.side.axis()] - fixed_edge_pos);
        if self.outer_size_range.min < dragged_size {
            *is_expanded = true;
        }
    }

    /// Get the current _outer_ width or height of the panel (from previous frame),
    /// including the [`Frame`] margin & border, or fall back to some default.
    ///
    /// Always clamped to [`Self::outer_size_range`] so callers get the size the
    /// panel would actually render at — never a stale persisted size from a
    /// previous build with a different range.
    fn outer_size(&self, ui: &Ui) -> f32 {
        let axis = self.side.axis();
        let raw = if let Some(state) = PanelState::load(ui, self.id) {
            state.outer_rect.size_along(axis)
        } else if let Some(default_outer_size) = self.default_outer_size {
            default_outer_size
        } else {
            let frame = self.resolve_frame(ui);
            ui.style().spacing.interact_size[axis] + frame.total_margin().sum()[axis]
        };
        clamp_to_range(raw, self.outer_size_range)
    }

    fn resize_panel(&self, outer_rect: Rect, ui: &Ui) -> Response {
        let resize_pos = self.side.resize_pos(outer_rect);
        let panel_axis_range = Rangef::point(resize_pos);
        let cross_range = outer_rect.range_along(self.side.cross_axis());
        let (resize_x, resize_y) = if self.side.axis() == 0 {
            (panel_axis_range, cross_range)
        } else {
            (cross_range, panel_axis_range)
        };
        let amount = ui.style().interaction.resize_grab_radius_side * self.side.axis_unit();

        // Use `resize_id_source` so collapsed/expanded panels in
        // `show_switched` share one resize widget.
        let resize_id = self.resize_id_source.unwrap_or(self.id).with("__resize");
        let resize_rect = Rect::from_x_y_ranges(resize_x, resize_y).expand2(amount);
        ui.interact(resize_rect, resize_id, Sense::click_and_drag())
    }

    fn cursor_icon(&self, outer_size: f32) -> CursorIcon {
        // When this panel is the collapsed view of `show_switched`
        // (`resize_id_source` is set), dragging past `max_size` triggers
        // drag-to-expand — so the user can always grow further. Treat the cap
        // as `INFINITY` for cursor purposes, otherwise we'd advertise
        // "can only shrink" while sitting on a drag-to-expand affordance.
        let can_drag_to_expand = self.resize_id_source.is_some();
        let max_for_cursor = if can_drag_to_expand {
            f32::INFINITY
        } else {
            self.outer_size_range.max
        };

        if outer_size <= self.outer_size_range.min {
            // Can only grow (toward the resizable side):
            match self.side {
                PanelSide::Left => CursorIcon::ResizeEast,
                PanelSide::Right => CursorIcon::ResizeWest,
                PanelSide::Top => CursorIcon::ResizeSouth,
                PanelSide::Bottom => CursorIcon::ResizeNorth,
            }
        } else if outer_size < max_for_cursor {
            if self.side.axis() == 0 {
                CursorIcon::ResizeHorizontal
            } else {
                CursorIcon::ResizeVertical
            }
        } else {
            // Can only shrink (toward the fixed side):
            match self.side {
                PanelSide::Left => CursorIcon::ResizeWest,
                PanelSide::Right => CursorIcon::ResizeEast,
                PanelSide::Top => CursorIcon::ResizeNorth,
                PanelSide::Bottom => CursorIcon::ResizeSouth,
            }
        }
    }

    /// Slide the panel toward its fixed edge. `1.0` = fully visible, `0.0` = fully off-screen.
    #[inline]
    fn with_slide_fraction(mut self, slide_fraction: f32) -> Self {
        self.slide_fraction = slide_fraction;
        self
    }

    /// Register the resize-handle widget under this `Id` instead of `self.id`.
    ///
    /// Used by [`Self::show_switched`] to share one widget across
    /// the collapsed and expanded panels.
    #[inline]
    fn with_resize_id_source(mut self, id: Id) -> Self {
        self.resize_id_source = Some(id);
        self
    }

    /// Override the drag-to-collapse threshold (defaults to `min_size`).
    #[inline]
    fn with_collapse_threshold(mut self, threshold: f32) -> Self {
        self.collapse_threshold = Some(threshold);
        self
    }
}

// ----------------------------------------------------------------------------

/// A panel that covers the remainder of the screen,
/// i.e. whatever area is left after adding other panels.
///
/// This acts very similar to [`Frame::central_panel`], but always expands
/// to use up all available space.
///
/// The order in which you add panels matter!
/// The first panel you add will always be the outermost, and the last you add will always be the innermost.
///
/// ⚠ [`CentralPanel`] must be added after all other panels!
///
/// NOTE: Any [`crate::Window`]s and [`crate::Area`]s will cover the top-level [`CentralPanel`].
///
/// See the [module level docs](crate::containers::panel) for more details.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::Panel::top("my_panel").show(ui, |ui| {
///    ui.label("Hello World! From `Panel`, that must be before `CentralPanel`!");
/// });
/// egui::CentralPanel::default().show(ui, |ui| {
///    ui.label("Hello World!");
/// });
/// # });
/// ```
#[must_use = "You should call .show()"]
#[derive(Default)]
pub struct CentralPanel {
    frame: Option<Frame>,
}

impl CentralPanel {
    /// A central panel with no margin or background color
    pub fn no_frame() -> Self {
        Self {
            frame: Some(Frame::NONE),
        }
    }

    /// A central panel with a background color and some inner margins
    pub fn default_margins() -> Self {
        Self { frame: None }
    }

    /// Change the background color, margins, etc.
    #[inline]
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Show the panel inside a [`Ui`].
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        self.show_inside_dyn(ui, Box::new(add_contents))
    }

    /// Renamed to [`Self::show`].
    #[deprecated = "Renamed to `show`"]
    pub fn show_inside<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.show(ui, add_contents)
    }

    /// Show the panel inside a [`Ui`].
    fn show_inside_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let Self { frame } = self;

        let outer_rect = ui.available_rect_before_wrap();
        let mut panel_ui = ui.new_child(
            UiBuilder::new()
                .ui_stack_info(UiStackInfo::new(UiKind::CentralPanel))
                .max_rect(outer_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        panel_ui.set_clip_rect(outer_rect); // If we overflow, don't do so visibly (#4475)

        let frame = frame.unwrap_or_else(|| Frame::central_panel(ui.style()));
        let response = frame.show(&mut panel_ui, |ui| {
            ui.expand_to_include_rect(ui.max_rect()); // Expand frame to include it all
            add_contents(ui)
        });

        // Use up space in the parent:
        ui.advance_cursor_after_rect(response.response.rect);

        response
    }
}

fn clamp_to_range(x: f32, range: Rangef) -> f32 {
    let range = range.as_positive();
    x.clamp(range.min, range.max)
}
