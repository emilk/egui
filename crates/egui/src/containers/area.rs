//! Area is a [`Ui`] that has no parent, it floats on the background.
//! It has no frame or own size. It is potentially movable.
//! It is the foundation for windows and popups.

use emath::{GuiRounding as _, RectAlign};

use crate::{
    Align2, Context, Id, InnerResponse, LayerId, Layout, NumExt as _, Order, Pos2, Rect, Response,
    Sense, Ui, UiBuilder, UiKind, UiStackInfo, Vec2, WidgetRect, WidgetWithState, emath, pos2,
};

/// State of an [`Area`] that is persisted between frames.
///
/// Areas back [`crate::Window`]s and other floating containers,
/// like tooltips and the popups of [`crate::ComboBox`].
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct AreaState {
    /// Last known position of the pivot.
    pub pivot_pos: Option<Pos2>,

    /// The anchor point of the area, i.e. where on the area the [`Self::pivot_pos`] refers to.
    pub pivot: Align2,

    /// Last known size.
    ///
    /// Area size is intentionally NOT persisted between sessions,
    /// so that a bad tooltip or menu size won't be remembered forever.
    /// A resizable [`crate::Window`] remembers the size the user picked using
    /// the state in the [`crate::Resize`] container.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub size: Option<Vec2>,

    /// Placement size an anchored area uses in its anchor derivation instead of its
    /// live size, once a user resize drag has happened on it (see the
    /// `resize_drag_in_flight` flag `Resize` sets): the reading-start edge of the
    /// placed rect pins and the area grows reading-forward from it. Not persisted
    /// between sessions, like [`Self::size`].
    #[cfg_attr(feature = "serde", serde(skip))]
    pub resize_pin_placement: Option<Vec2>,

    /// If false, clicks goes straight through to what is behind us. Useful for tooltips etc.
    pub interactable: bool,

    /// At what time was this area first shown?
    ///
    /// Used to fade in the area.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub last_became_visible_at: Option<f64>,
}

impl Default for AreaState {
    fn default() -> Self {
        Self {
            pivot_pos: None,
            pivot: Align2::LEFT_TOP,
            size: None,
            resize_pin_placement: None,
            interactable: true,
            last_became_visible_at: None,
        }
    }
}

impl AreaState {
    /// Load the state of an [`Area`] from memory.
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        // TODO(emilk): Area state is not currently stored in `Memory::data`, but maybe it should be?
        ctx.memory(|mem| mem.areas().get(id).copied())
    }

    /// The left top positions of the area.
    pub fn left_top_pos(&self) -> Pos2 {
        let pivot_pos = self.pivot_pos.unwrap_or_default();
        let size = self.size.unwrap_or_default();
        pos2(
            pivot_pos.x - self.pivot.x().to_factor() * size.x,
            pivot_pos.y - self.pivot.y().to_factor() * size.y,
        )
        .round_ui()
    }

    /// Move the left top positions of the area.
    pub fn set_left_top_pos(&mut self, pos: Pos2) {
        let size = self.size.unwrap_or_default();
        self.pivot_pos = Some(pos2(
            pos.x + self.pivot.x().to_factor() * size.x,
            pos.y + self.pivot.y().to_factor() * size.y,
        ));
    }

    /// Place the area so that its pivot corner lands where `rect`'s pivot corner is.
    ///
    /// Unlike [`Self::set_left_top_pos`], which folds the given left-top position
    /// through the persisted (and by then stale) [`Self::size`], this derives the
    /// pivot position from `rect` itself, so it stays exact when `rect`'s size
    /// differs from the persisted one — an interactive resize, where the pivot
    /// corner is the un-dragged reading edge and must stay fixed while the drag
    /// changes the size.
    pub(crate) fn set_rect_pivot_pos(&mut self, rect: Rect) {
        self.pivot_pos = Some(pos2(
            rect.left() + self.pivot.x().to_factor() * rect.width(),
            rect.top() + self.pivot.y().to_factor() * rect.height(),
        ));
    }

    /// Where the area is on screen.
    pub fn rect(&self) -> Rect {
        let size = self.size.unwrap_or_default();
        Rect::from_min_size(self.left_top_pos(), size).round_ui()
    }
}

/// An area on the screen that can be moved by dragging.
///
/// This forms the base of the [`crate::Window`] container.
///
/// ```
/// # egui::__run_test_ctx(|ctx| {
/// egui::Area::new(egui::Id::unique("my_area"))
///     .fixed_pos(egui::pos2(32.0, 32.0))
///     .show(ctx, |ui| {
///         ui.label("Floating text!");
///     });
/// # });
/// ```
///
/// The previous rectangle used by this area can be obtained through [`crate::Memory::area_rect()`].
#[must_use = "You should call .show()"]
#[derive(Clone, Debug)]
pub struct Area {
    pub(crate) id: Id,
    info: UiStackInfo,
    sense: Option<Sense>,
    movable: bool,
    interactable: bool,
    enabled: bool,
    constrain: bool,
    constrain_rect: Option<Rect>,
    order: Order,
    default_pos: Option<Pos2>,
    default_size: Vec2,
    /// The pivot the user explicitly set with [`Self::pivot`], or `None` for the default.
    ///
    /// The default pivot is mirrored to the reading direction
    /// ([`Style::right_to_left`]): `RIGHT_TOP` in a right-to-left interface,
    /// `LEFT_TOP` otherwise, so areas widen towards the reading end.
    pivot: Option<Align2>,
    anchor: Option<(Align2, Vec2)>,
    new_pos: Option<Pos2>,
    fade_in: bool,
    layout: Layout,
    sizing_pass: bool,
    accessibility_parent: Option<Id>,
    accessibility_label: Option<String>,
    accessibility_role: Option<crate::accesskit::Role>,
}

impl WidgetWithState for Area {
    type State = AreaState;
}

impl Area {
    /// The `id` must be globally unique.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            info: UiStackInfo::new(UiKind::GenericArea),
            sense: None,
            movable: true,
            interactable: true,
            constrain: true,
            constrain_rect: None,
            enabled: true,
            order: Order::Middle,
            default_pos: None,
            default_size: Vec2::NAN,
            new_pos: None,
            pivot: None,
            anchor: None,
            fade_in: true,
            layout: Layout::default(),
            sizing_pass: false,
            accessibility_parent: None,
            accessibility_label: None,
            accessibility_role: None,
        }
    }

    /// Let's you change the `id` that you assigned in [`Self::new`].
    ///
    /// The `id` must be globally unique.
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    /// Change the [`UiKind`] of the arena.
    ///
    /// Default to [`UiKind::GenericArea`].
    #[inline]
    pub fn kind(mut self, kind: UiKind) -> Self {
        self.info = UiStackInfo::new(kind);
        self
    }

    /// Set the [`UiStackInfo`] of the area's [`Ui`].
    ///
    /// Default to [`UiStackInfo`] with kind [`UiKind::GenericArea`].
    #[inline]
    pub fn info(mut self, info: UiStackInfo) -> Self {
        self.info = info;
        self
    }

    /// Nest the area under this widget in the accessibility tree.
    ///
    /// Popups, menus and tooltips use this to hang under the widget that opened them,
    /// instead of floating at the root of the tree.
    #[inline]
    pub fn accessibility_parent(mut self, widget_id: Id) -> Self {
        self.accessibility_parent = Some(widget_id);
        self
    }

    /// Name the area in the accessibility tree.
    ///
    /// The role comes from the [`UiKind`]; the name is what a screen reader or a test finds it by.
    #[inline]
    pub fn accessible_name(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Give the area a role in the accessibility tree other than the one its [`UiKind`] implies,
    /// e.g. [`Role::Alert`](crate::accesskit::Role::Alert) for a toast.
    #[inline]
    pub fn role(mut self, role: crate::accesskit::Role) -> Self {
        self.accessibility_role = Some(role);
        self
    }

    pub fn layer(&self) -> LayerId {
        LayerId::new(self.order, self.id)
    }

    /// If false, no content responds to click
    /// and widgets will be shown grayed out.
    /// You won't be able to move the window.
    /// Default: `true`.
    #[inline]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Moveable by dragging the area?
    #[inline]
    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self.interactable |= movable;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_movable(&self) -> bool {
        self.movable && self.enabled
    }

    /// If false, clicks goes straight through to what is behind us.
    ///
    /// Can be used for semi-invisible areas that the user should be able to click through.
    ///
    /// Default: `true`.
    #[inline]
    pub fn interactable(mut self, interactable: bool) -> Self {
        self.interactable = interactable;
        self.movable &= interactable;
        self
    }

    /// Explicitly set a sense.
    ///
    /// If not set, this will default to `Sense::drag()` if movable, `Sense::click()` if interactable, and `Sense::hover()` otherwise.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = Some(sense);
        self
    }

    /// `order(Order::Foreground)` for an Area that should always be on top
    #[inline]
    pub fn order(mut self, order: Order) -> Self {
        self.order = order;
        self
    }

    #[inline]
    pub fn default_pos(mut self, default_pos: impl Into<Pos2>) -> Self {
        self.default_pos = Some(default_pos.into());
        self
    }

    /// The size used for the [`Ui::max_rect`] the first frame.
    ///
    /// Text will wrap at this width, and images that expand to fill the available space
    /// will expand to this size.
    ///
    /// If the contents are smaller than this size, the area will shrink to fit the contents.
    /// If the contents overflow, the area will grow.
    ///
    /// If not set, [`crate::style::Spacing::default_area_size`] will be used.
    #[inline]
    pub fn default_size(mut self, default_size: impl Into<Vec2>) -> Self {
        self.default_size = default_size.into();
        self
    }

    /// See [`Self::default_size`].
    #[inline]
    pub fn default_width(mut self, default_width: f32) -> Self {
        self.default_size.x = default_width;
        self
    }

    /// See [`Self::default_size`].
    #[inline]
    pub fn default_height(mut self, default_height: f32) -> Self {
        self.default_size.y = default_height;
        self
    }

    /// Positions the window and prevents it from being moved
    #[inline]
    pub fn fixed_pos(mut self, fixed_pos: impl Into<Pos2>) -> Self {
        self.new_pos = Some(fixed_pos.into());
        self.movable = false;
        self
    }

    /// Constrains this area to [`Context::content_rect`]?
    ///
    /// Default: `true`.
    #[inline]
    pub fn constrain(mut self, constrain: bool) -> Self {
        self.constrain = constrain;
        self
    }

    /// Constrain the movement of the window to the given rectangle.
    ///
    /// For instance: `.constrain_to(ctx.content_rect())`.
    #[inline]
    pub fn constrain_to(mut self, constrain_rect: Rect) -> Self {
        self.constrain = true;
        self.constrain_rect = Some(constrain_rect);
        self
    }

    /// Where the "root" of the area is.
    ///
    /// For instance, if you set this to [`Align2::RIGHT_TOP`]
    /// then [`Self::fixed_pos`] will set the position of the right-top
    /// corner of the area.
    ///
    /// Default: [`Align2::LEFT_TOP`], mirrored to [`Align2::RIGHT_TOP`]
    /// in a right-to-left interface ([`Style::right_to_left`]),
    /// so areas widen towards the reading end.
    #[inline]
    pub fn pivot(mut self, pivot: Align2) -> Self {
        self.pivot = Some(pivot);
        self
    }

    /// Positions the window but you can still move it.
    #[inline]
    pub fn current_pos(mut self, current_pos: impl Into<Pos2>) -> Self {
        self.new_pos = Some(current_pos.into());
        self
    }

    /// Set anchor and distance.
    ///
    /// An anchor of `Align2::RIGHT_TOP` means "put the right-top corner of the window
    /// in the right-top corner of the screen".
    ///
    /// The offset is added to the position, so e.g. an offset of `[-5.0, 5.0]`
    /// would move the window left and down from the given anchor.
    ///
    /// Anchoring also makes the window immovable.
    ///
    /// It is an error to set both an anchor and a position.
    #[inline]
    pub fn anchor(mut self, align: Align2, offset: impl Into<Vec2>) -> Self {
        self.anchor = Some((align, offset.into()));
        self.movable(false)
    }

    /// The pivot in effect: the [`Self::anchor`] pivot if anchored,
    /// the explicitly set [`Self::pivot`] if there is one, and otherwise
    /// the default, mirrored to the reading direction (`rtl`):
    /// `RIGHT_TOP` in a right-to-left interface, `LEFT_TOP` otherwise.
    pub(crate) fn get_pivot(&self, rtl: bool) -> Align2 {
        if let Some((pivot, _)) = self.anchor {
            pivot
        } else {
            self.pivot.unwrap_or_else(|| default_pivot(rtl))
        }
    }

    /// Was this area placed with [`Area::anchor`] (position re-derived from the anchor each frame)?
    pub(crate) fn anchored(&self) -> bool {
        self.anchor.is_some()
    }

    /// Was this area placed with [`Area::fixed_pos`] or [`Area::current_pos`]?
    ///
    /// `Area::begin` re-applies that position every frame, overwriting whatever
    /// was persisted in the state.
    pub(crate) fn has_new_pos(&self) -> bool {
        self.new_pos.is_some()
    }

    /// If `true`, quickly fade in the area.
    ///
    /// Default: `true`.
    #[inline]
    pub fn fade_in(mut self, fade_in: bool) -> Self {
        self.fade_in = fade_in;
        self
    }

    /// Set the layout for the child Ui.
    #[inline]
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    /// While true, a sizing pass will be done. This means the area will be invisible
    /// and the contents will be laid out to estimate the proper containing size of the area.
    /// If false, there will be no change to the default area behavior. This is useful if the
    /// area contents area dynamic and you need to need to make sure the area adjusts its size
    /// accordingly.
    ///
    /// This should only be set to true during the specific frames you want force a sizing pass.
    /// Do NOT hard-code this as `.sizing_pass(true)`, as it will cause the area to never be
    /// visible.
    ///
    /// # Arguments
    /// - resize: If true, the area will be resized to fit its contents. False will keep the
    ///   default area resizing behavior.
    ///
    /// Default: `false`.
    #[inline]
    pub fn sizing_pass(mut self, resize: bool) -> Self {
        self.sizing_pass = resize;
        self
    }
}

impl Area {
    /// Find the pivot and position that place an area of `size` with its `pivot` corner
    /// at `anchor`, fully inside `screen_rect`.
    ///
    /// Like [`crate::Popup`], the preferred side is kept when it fits; otherwise the flipped
    /// side is tried ([`emath::RectAlign::symmetries`]). If no side fits (the area is larger
    /// than `screen_rect`, or the anchor is pinned at its far edge) the rect is clamped inside.
    ///
    /// The search covers the preferred alignment and its symmetries only — unlike
    /// [`crate::Popup::get_best_align`] it does not fall back to [`emath::RectAlign::MENU_ALIGNS`],
    /// because the caller asked for the area to stay anchored (with its `pivot` corner) at
    /// `anchor`, not to jump to a different edge.
    ///
    /// The returned position is the position of the *pivot* (what [`Self::fixed_pos`] expects),
    /// not the top-left corner — feed the pair straight back into `.pivot(pivot).fixed_pos(pos)`.
    ///
    /// The function is direction-agnostic: an interface reading right-to-left passes the
    /// mirrored preferred pivot (e.g. [`Align2::RIGHT_TOP`] instead of [`Align2::LEFT_TOP`]).
    ///
    /// Degenerate inputs never panic: a `screen_rect` with NaN bounds returns `(pivot, anchor)`
    /// unchanged; other values outside the domain of `f32::clamp` (inverted or single-NaN
    /// bounds, NaN or negative `size`) yield clamped or degenerate geometry, never a panic.
    ///
    /// ```
    /// # use egui::{pos2, vec2, Area, Align2, Id, Order};
    /// # egui::__run_test_ctx(|ctx| {
    /// let anchor = pos2(790.0, 100.0); // e.g. the pointer position
    /// let (pivot, pos) =
    ///     Area::fit_pos(anchor, vec2(200.0, 140.0), Align2::LEFT_TOP, ctx.content_rect());
    /// Area::new(Id::unique("fit_pos_example"))
    ///     .order(Order::Foreground)
    ///     .pivot(pivot)
    ///     .fixed_pos(pos)
    ///     .show(ctx, |ui| ui.set_min_size(vec2(200.0, 140.0)));
    /// # });
    /// ```
    pub fn fit_pos(anchor: Pos2, size: Vec2, pivot: Align2, screen_rect: Rect) -> (Align2, Pos2) {
        // Degenerate screen rects (unmeasured viewports) must not panic: `f32::clamp`
        // asserts `min <= max`, which NaN bounds violate.
        if screen_rect.any_nan() {
            return (pivot, anchor);
        }
        let screen_rect = Rect::from_two_pos(screen_rect.min, screen_rect.max); // normalize inverted rects

        let anchor_rect = Rect::from_min_size(anchor, Vec2::ZERO);
        let preferred = RectAlign::from_align2(pivot);

        let best = RectAlign::find_best_align(
            core::iter::chain(core::iter::once(preferred), preferred.symmetries()),
            screen_rect,
            anchor_rect,
            0.0,
            size,
        )
        .unwrap_or(preferred); // find_best_align never returns None when given alternatives

        let mut pos = best.align_rect(&anchor_rect, size, 0.0).min;
        // No side fits: clamp what's left. The `.max` keeps the upper bound >= the lower one
        // when `size` is larger than `screen_rect` (or NaN), so `clamp` cannot assert.
        pos.x = pos.x.clamp(
            screen_rect.min.x,
            (screen_rect.max.x - size.x).max(screen_rect.min.x),
        );
        pos.y = pos.y.clamp(
            screen_rect.min.y,
            (screen_rect.max.y - size.y).max(screen_rect.min.y),
        );

        let rect = Rect::from_min_size(pos, size);
        (best.child(), best.child().pos_in_rect(&rect))
    }
}

pub(crate) struct Prepared {
    info: Option<UiStackInfo>,
    layer_id: LayerId,
    state: AreaState,
    move_response: Response,
    enabled: bool,
    constrain: bool,
    constrain_rect: Rect,

    /// We always make windows invisible the first frame to hide "first-frame-jitters".
    ///
    /// This is so that we use the first frame to calculate the window size,
    /// and then can correctly position the window and its contents the next frame,
    /// without having one frame where the window is wrongly positioned or sized.
    sizing_pass: bool,

    fade_in: bool,
    layout: Layout,
    accessibility_label: Option<String>,
    accessibility_role: Option<crate::accesskit::Role>,
}

impl Area {
    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let mut prepared = self.begin(ctx);
        let mut content_ui = prepared.content_ui(ctx);
        let inner = add_contents(&mut content_ui);
        let response = prepared.end(ctx, content_ui);
        InnerResponse { inner, response }
    }

    pub(crate) fn begin(self, ctx: &Context) -> Prepared {
        let Self {
            id,
            info,
            sense,
            movable,
            order,
            interactable,
            enabled,
            default_pos,
            default_size,
            new_pos,
            pivot,
            anchor,
            constrain,
            constrain_rect,
            fade_in,
            layout,
            sizing_pass: force_sizing_pass,
            accessibility_parent,
            accessibility_label,
            accessibility_role,
        } = self;

        // An explicit pivot wins; the default pivot follows the reading direction
        // so areas widen towards the reading end. Anchored areas are placed by
        // their anchor, so their state pivot stays the plain default.
        let rtl = ctx.is_rtl();
        let pivot = if let Some(pivot) = pivot {
            pivot
        } else if anchor.is_none() {
            default_pivot(rtl)
        } else {
            Align2::LEFT_TOP
        };

        let constrain_rect = constrain_rect.unwrap_or_else(|| ctx.content_rect());

        let layer_id = LayerId::new(order, id);

        let state = AreaState::load(ctx, id);
        let mut sizing_pass = state.is_none();

        // The size is not persisted, so after a restart we have a restored position but no size.
        // That position was already constrained in the previous session,
        // and will be constrained again next frame, once we know the actual size.
        // So we don't constrain it with the guessed size of this sizing pass,
        // which could move the area (e.g. a shrunk window placed against the edge of the screen).
        let restored_without_size = !force_sizing_pass
            && state
                .as_ref()
                .is_some_and(|state| state.pivot_pos.is_some() && state.size.is_none());

        let mut state = state.unwrap_or(AreaState {
            pivot_pos: None,
            pivot,
            size: None,
            resize_pin_placement: None,
            interactable,
            last_became_visible_at: None,
        });
        if force_sizing_pass {
            sizing_pass = true;
            state.size = None;
        }
        let old_pivot = state.pivot;
        state.pivot = pivot;
        if let Some(new_pos) = new_pos {
            state.pivot_pos = Some(new_pos);
        }
        state.pivot_pos.get_or_insert_with(|| {
            default_pos
                .unwrap_or_else(|| automatic_area_position(ctx, constrain_rect, layer_id, rtl))
        });

        let size = *state.size.get_or_insert_with(|| {
            sizing_pass = true;

            // during the sizing pass we will use this as the max size
            let mut size = default_size;

            let default_area_size = ctx.global_style().spacing.default_area_size;
            if size.x.is_nan() {
                size.x = default_area_size.x;
            }
            if size.y.is_nan() {
                size.y = default_area_size.y;
            }

            if constrain {
                size = size.at_most(constrain_rect.size());
            }

            size
        });

        let constrain = constrain && !restored_without_size;

        // The stored pivot position was encoded in the previous pivot's space; when the
        // pivot changes (the default pivot follows the interface direction), re-express
        // it so the area's left-top stays where it was. Without a known size the stored
        // point is treated as the old left-top (the default pivots anchor a corner).
        // A caller re-asserting an explicit position re-pins below and wins, as today.
        if pivot != old_pivot
            && new_pos.is_none()
            && let Some(pivot_pos) = state.pivot_pos
        {
            let size = state.size.unwrap_or_default();
            let left_top = pos2(
                pivot_pos.x - old_pivot.x().to_factor() * size.x,
                pivot_pos.y - old_pivot.y().to_factor() * size.y,
            );
            state.pivot_pos = Some(pos2(
                left_top.x + pivot.x().to_factor() * size.x,
                left_top.y + pivot.y().to_factor() * size.y,
            ));
        }

        // We should never be interactable during a sizing pass, since then we are shown at a different
        // size which might interfere with hover state of the hovered widget causing popup feedback loops.
        state.interactable = interactable && !sizing_pass;

        // TODO(emilk): if last frame was sizing pass, it should be considered invisible for smoother fade-in
        let visible_last_frame = ctx.memory(|mem| mem.areas().visible_last_frame(&layer_id));

        if !visible_last_frame || state.last_became_visible_at.is_none() {
            state.last_became_visible_at = Some(ctx.input(|i| i.time));
        }

        if let Some((anchor, offset)) = anchor {
            // A `Resize` inside this area sets `resize_drag_in_flight` on its layer
            // while a user resize drag is in flight. On such a frame the anchored
            // placement is frozen: from then on the placement is derived from the
            // frozen size instead of the live one, so the reading-start edge of the
            // placed rect pins and the area grows reading-forward from it (a centered
            // `Modal` with a resize grip, for instance, stops re-centering around the
            // dragged size — which would otherwise grow it at double pointer rate,
            // both edges moving by half the delta). The pin persists after the drag:
            // unfreezing would re-derive the placement from the final size and jump
            // the area back towards its anchor by half the growth, and a user-resized
            // area stays where the user left it, like a user-resized window.
            let resize_drag_in_flight = ctx.data(|d| {
                d.get_temp::<()>(layer_id.id.with("resize_drag_in_flight"))
                    .is_some()
            });
            if resize_drag_in_flight {
                state.resize_pin_placement.get_or_insert(size);
            }
            let placement_size = state.resize_pin_placement.unwrap_or(size);
            let placed = anchor
                .align_size_within_rect(placement_size, constrain_rect)
                .translate(offset);
            // While the placement is pinned, the reading-start edge of the placed
            // rect is the frozen-size placement's reading-start edge: under RTL the
            // rect's right edge, under LTR its left edge, and in both directions its
            // top edge (growth is downward). Anchors that already pin that edge for
            // any size (corner anchors like `RIGHT_TOP` under RTL) are unaffected —
            // the pin only changes anchors whose reading-start edge moves with size
            // (`CENTER_*` and the mirrored `LEFT_*`/`RIGHT_*` cases).
            let left_top = if state.resize_pin_placement.is_some() && rtl {
                pos2(placed.right() - size.x, placed.top())
            } else {
                placed.left_top()
            };
            state.set_left_top_pos(left_top);
        }

        // interact right away to prevent frame-delay
        let mut move_response = {
            let interact_id = layer_id.id.with("move");
            let sense = sense.unwrap_or_else(|| {
                if movable {
                    Sense::DRAG
                } else if interactable {
                    Sense::CLICK // allow clicks to bring to front
                } else {
                    Sense::hover()
                }
            });

            // Must come before the widget is created, since that is when its node is placed.
            if let Some(parent) = accessibility_parent {
                ctx.register_accesskit_parent(interact_id, parent);
            }

            let move_response = ctx.create_widget(
                WidgetRect {
                    id: interact_id,
                    parent_id: id,
                    layer_id,
                    rect: state.rect(),
                    interact_rect: state.rect().intersect(constrain_rect),
                    sense,
                    enabled,
                    visible: !sizing_pass,
                },
                true,
                Default::default(),
            );

            // Used to prevent drift
            let pivot_at_start_of_drag_id = id.with("pivot_at_drag_start");

            if movable
                && move_response.dragged()
                && let Some(pivot_pos) = &mut state.pivot_pos
            {
                let pivot_at_start_of_drag = ctx.data_mut(|data| {
                    *data.get_temp_mut_or::<Pos2>(pivot_at_start_of_drag_id, *pivot_pos)
                });

                *pivot_pos =
                    pivot_at_start_of_drag + move_response.total_drag_delta().unwrap_or_default();
            } else {
                ctx.data_mut(|data| data.remove::<Pos2>(pivot_at_start_of_drag_id));
            }

            if (move_response.dragged() || move_response.clicked())
                || pointer_pressed_on_area(ctx, layer_id)
                || !ctx.memory(|m| m.areas().visible_last_frame(&layer_id))
            {
                ctx.memory_mut(|m| m.areas_mut().move_to_top(layer_id));
                ctx.request_repaint();
            }

            move_response
        };

        state.set_left_top_pos(round_area_position(
            ctx,
            if constrain {
                Context::constrain_window_rect_to_area(state.rect(), constrain_rect).min
            } else {
                state.left_top_pos()
            },
        ));

        // Update response with possibly moved/constrained rect:
        move_response.rect = state.rect();
        move_response.interact_rect = state.rect();

        Prepared {
            info: Some(info),
            layer_id,
            state,
            move_response,
            enabled,
            constrain,
            constrain_rect,
            sizing_pass,
            fade_in,
            layout,
            accessibility_label,
            accessibility_role,
        }
    }
}

/// The default [`Area`] pivot, mirrored to the reading direction: `RIGHT_TOP`
/// in a right-to-left interface, `LEFT_TOP` otherwise, so areas widen towards
/// the reading end.
fn default_pivot(rtl: bool) -> Align2 {
    if rtl {
        Align2::RIGHT_TOP
    } else {
        Align2::LEFT_TOP
    }
}

pub(crate) fn round_area_position(ctx: &Context, pos: Pos2) -> Pos2 {
    // We round a lot of rendering to pixels, so we round the whole
    // area positions to pixels too, so avoid widgets appearing to float
    // around independently of each other when the area is dragged.
    // But just in case pixels_per_point is irrational,
    // we then also round to ui coordinates:

    pos.round_to_pixels(ctx.pixels_per_point()).round_ui()
}

impl Prepared {
    pub(crate) fn state_mut(&mut self) -> &mut AreaState {
        &mut self.state
    }

    pub(crate) fn constrain(&self) -> bool {
        self.constrain
    }

    pub(crate) fn constrain_rect(&self) -> Rect {
        self.constrain_rect
    }

    pub(crate) fn content_ui(&mut self, ctx: &Context) -> Ui {
        let max_rect = self.state.rect();

        let mut ui_builder = UiBuilder::new()
            .ui_stack_info(self.info.take().unwrap_or_default())
            .layer_id(self.layer_id)
            .max_rect(max_rect)
            .layout(self.layout)
            .accessibility_parent(self.move_response.id)
            .closable();

        if let Some(label) = self.accessibility_label.take() {
            ui_builder = ui_builder.accessibility_label(label);
        }
        if let Some(role) = self.accessibility_role {
            ui_builder = ui_builder.accessibility_role(role);
        }

        if !self.enabled {
            ui_builder = ui_builder.disabled();
        }
        if self.sizing_pass {
            ui_builder = ui_builder.sizing_pass().invisible();
        }

        let mut ui = Ui::new(ctx.clone(), self.layer_id.id, ui_builder);
        ui.set_clip_rect(self.constrain_rect); // Don't paint outside our bounds

        if self.fade_in
            && let Some(last_became_visible_at) = self.state.last_became_visible_at
        {
            let age =
                ctx.input(|i| (i.time - last_became_visible_at) as f32 + i.predicted_dt * 0.5);
            let opacity =
                crate::remap_clamp(age, 0.0..=ctx.global_style().animation_time, 0.0..=1.0);
            let opacity = emath::easing::quadratic_out(opacity); // slow fade-out = quick fade-in
            ui.multiply_opacity(opacity);
            if opacity < 1.0 {
                ctx.request_repaint();
            }
        }

        ui
    }

    pub(crate) fn with_widget_info(&self, make_info: impl Fn() -> crate::WidgetInfo) {
        self.move_response.widget_info(make_info);
    }

    pub(crate) fn id(&self) -> Id {
        self.move_response.id
    }

    #[expect(clippy::needless_pass_by_value)] // intentional to swallow up `content_ui`.
    pub(crate) fn end(self, ctx: &Context, content_ui: Ui) -> Response {
        let Self {
            info: _,
            layer_id,
            mut state,
            move_response: mut response,
            sizing_pass,
            ..
        } = self;

        state.size = Some(content_ui.min_size());

        // Make sure we report back the correct size.
        // Very important after the initial sizing pass, when the initial estimate of the size is way off.
        let final_rect = state.rect();
        response.rect = final_rect;
        response.interact_rect = final_rect;

        // TODO(lucasmerlin): Can the area response be based on Ui::response? Then this won't be needed
        // Bubble up the close event
        if content_ui.should_close() {
            response.set_close();
        }

        ctx.memory_mut(|m| m.areas_mut().set_state(layer_id, state));

        if sizing_pass {
            // If we didn't know the size, we were likely drawing the area in the wrong place.
            ctx.request_repaint();
        }

        response
    }
}

fn pointer_pressed_on_area(ctx: &Context, layer_id: LayerId) -> bool {
    if let Some(pointer_pos) = ctx.pointer_interact_pos() {
        let any_pressed = ctx.input(|i| i.pointer.any_pressed());
        any_pressed && ctx.layer_id_at(pointer_pos) == Some(layer_id)
    } else {
        false
    }
}

fn automatic_area_position(
    ctx: &Context,
    constrain_rect: Rect,
    layer_id: LayerId,
    rtl: bool,
) -> Pos2 {
    let existing: Vec<Rect> = ctx.memory(|mem| {
        mem.areas()
            .visible_windows()
            .filter(|(id, _)| id != &layer_id) // ignore ourselves
            .filter(|(_, state)| state.pivot_pos.is_some() && state.size.is_some())
            .map(|(_, state)| state.rect())
            .collect()
    });

    // The packing below computes a left-top position for the default
    // `LEFT_TOP` pivot. In a right-to-left interface, mirror the horizontal
    // axis, run the same packing, and mirror the result back, so new areas
    // appear at the reading-start edge and widen leftward.
    if rtl {
        let mirrored = existing
            .iter()
            .map(|&rect| mirror_rect_x(rect, constrain_rect))
            .collect();
        let pos = automatic_area_position_in(constrain_rect, mirrored);
        return pos2(mirror_x(pos.x, constrain_rect), pos.y);
    }

    automatic_area_position_in(constrain_rect, existing)
}

/// Mirror a rect horizontally within `within`.
fn mirror_rect_x(rect: Rect, within: Rect) -> Rect {
    let left = mirror_x(rect.right(), within);
    let right = mirror_x(rect.left(), within);
    Rect::from_min_max(pos2(left, rect.top()), pos2(right, rect.bottom()))
}

/// Mirror an x coordinate horizontally within `within`.
fn mirror_x(x: f32, within: Rect) -> f32 {
    within.left() + within.right() - x
}

/// Place a new area so it doesn't overlap the (sorted) existing ones.
fn automatic_area_position_in(constrain_rect: Rect, mut existing: Vec<Rect>) -> Pos2 {
    existing.sort_by_key(|r| r.left().round() as i32);

    let spacing = 16.0;
    let left = constrain_rect.left() + spacing;
    let top = constrain_rect.top() + spacing;

    if existing.is_empty() {
        return pos2(left, top);
    }

    // Separate existing rectangles into columns:
    let mut column_bbs = vec![existing[0]];

    for &rect in &existing {
        #[expect(clippy::unwrap_used)]
        let current_column_bb = column_bbs.last_mut().unwrap();
        if rect.left() < current_column_bb.right() {
            // same column
            *current_column_bb |= rect;
        } else {
            // new column
            column_bbs.push(rect);
        }
    }

    {
        // Look for large spaces between columns (empty columns):
        let mut x = left;
        for col_bb in &column_bbs {
            let available = col_bb.left() - x;
            if available >= 300.0 {
                return pos2(x, top);
            }
            x = col_bb.right() + spacing;
        }
    }

    // Find first column with some available space at the bottom of it:
    for col_bb in &column_bbs {
        if col_bb.bottom() < constrain_rect.center().y {
            return pos2(col_bb.left(), col_bb.bottom() + spacing);
        }
    }

    // Maybe we can fit a new column?
    #[expect(clippy::unwrap_used)]
    let rightmost = column_bbs.last().unwrap().right();
    if rightmost + 200.0 < constrain_rect.right() {
        return pos2(rightmost + spacing, top);
    }

    // Ok, just put us in the column with the most space at the bottom:
    let mut best_pos = pos2(left, column_bbs[0].bottom() + spacing);
    for col_bb in &column_bbs {
        let col_pos = pos2(col_bb.left(), col_bb.bottom() + spacing);
        if col_pos.y < best_pos.y {
            best_pos = col_pos;
        }
    }
    best_pos
}

#[cfg(test)]
mod tests {
    use crate::{Align2, Context, Id, Pos2, RawInput, Rect, Vec2, pos2, vec2};

    use super::Area;

    fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))
    }

    /// Show an area of `size` best-fit at `anchor` with the preferred `pivot`, and return
    /// the rect it actually landed on (single `run_ui` frame; `Area::end` corrects the
    /// response rect to the measured size in the same frame).
    fn shown_rect(anchor: Pos2, size: Vec2, pivot: Align2) -> Rect {
        let ctx = Context::default();
        let (pivot, pos) = Area::fit_pos(anchor, size, pivot, screen());
        let mut rect = None;
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(screen()),
                ..Default::default()
            },
            |ui| {
                let response = Area::new(Id::unique("test"))
                    .pivot(pivot)
                    .fixed_pos(pos)
                    .show(ui.ctx(), |ui| {
                        ui.set_min_size(size);
                    });
                rect = Some(response.response.rect);
            },
        );
        output.drop_without_applying_deltas();
        rect.expect("run_ui callback ran")
    }

    #[test]
    fn fit_pos_keeps_the_preferred_side_when_it_fits() {
        let (pivot, pos) = Area::fit_pos(
            pos2(400.0, 300.0),
            vec2(200.0, 140.0),
            Align2::LEFT_TOP,
            screen(),
        );
        assert_eq!(pivot, Align2::LEFT_TOP);
        assert_eq!(pos, pos2(400.0, 300.0));
    }

    // The issue reproduction: a 200x140 area with a LEFT_TOP pivot at x=790 on an
    // 800-wide screen used to extend to x=990 with nothing in the Area API to correct it.
    #[test]
    fn fit_pos_flips_to_the_left_near_the_right_edge() {
        let (pivot, pos) = Area::fit_pos(
            pos2(790.0, 100.0),
            vec2(200.0, 140.0),
            Align2::LEFT_TOP,
            screen(),
        );
        assert_eq!(pivot, Align2::RIGHT_TOP);
        assert_eq!(pos, pos2(790.0, 100.0)); // right edge stays at the anchor, grows leftward
        let rect = shown_rect(pos2(790.0, 100.0), vec2(200.0, 140.0), Align2::LEFT_TOP);
        assert_eq!(
            rect,
            Rect::from_min_size(pos2(590.0, 100.0), vec2(200.0, 140.0))
        );
    }

    #[test]
    fn fit_pos_flips_upward_near_the_bottom_edge() {
        let (pivot, _) = Area::fit_pos(
            pos2(400.0, 590.0),
            vec2(200.0, 140.0),
            Align2::LEFT_TOP,
            screen(),
        );
        assert_eq!(pivot, Align2::LEFT_BOTTOM);
    }

    #[test]
    fn fit_pos_flipped_side_is_clamped_too() {
        // Anchor at the very right edge: flipping alone is not enough, the clamp must pull
        // the rect back inside (an app anchoring a picker at the screen's reading-end edge).
        let rect = shown_rect(pos2(800.0, 300.0), vec2(200.0, 140.0), Align2::LEFT_TOP);
        assert!(
            screen().contains_rect(rect),
            "{rect:?} must stay inside the screen"
        );
    }

    #[test]
    fn fit_pos_clamps_when_no_side_fits() {
        let rect = shown_rect(pos2(790.0, 590.0), vec2(900.0, 700.0), Align2::LEFT_TOP);
        assert_eq!(
            rect.min,
            screen().min,
            "no side fits: clamped to the screen corner"
        );
    }

    #[test]
    fn fit_pos_never_panics_on_degenerate_input() {
        let nan_rect = Rect::from_min_size(pos2(f32::NAN, 0.0), vec2(800.0, 600.0));
        assert_eq!(
            Area::fit_pos(
                pos2(100.0, 100.0),
                vec2(200.0, 140.0),
                Align2::LEFT_TOP,
                nan_rect
            ),
            (Align2::LEFT_TOP, pos2(100.0, 100.0)),
            "NaN screen bounds return the request unchanged"
        );
        let inverted = Rect::from_min_max(pos2(800.0, 600.0), Pos2::ZERO);
        let (_, pos) = Area::fit_pos(
            pos2(400.0, 300.0),
            vec2(200.0, 140.0),
            Align2::LEFT_TOP,
            inverted,
        );
        assert!(
            pos.is_finite(),
            "inverted screen rect clamps, never panics: {pos:?}"
        );
        let _ = Area::fit_pos(
            pos2(400.0, 300.0),
            vec2(f32::NAN, 140.0),
            Align2::LEFT_TOP,
            screen(),
        );
        let _ = Area::fit_pos(
            pos2(400.0, 300.0),
            vec2(-200.0, 140.0),
            Align2::LEFT_TOP,
            screen(),
        );
    }
}
