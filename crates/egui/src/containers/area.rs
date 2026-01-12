//! Area is a [`Ui`] that has no parent, it floats on the background.
//! It has no frame or own size. It is potentially movable.
//! It is the foundation for windows and popups.

use emath::GuiRounding as _;

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
/// egui::Area::new(egui::Id::new("my_area"))
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
    pivot: Align2,
    anchor: Option<(Align2, Vec2)>,
    new_pos: Option<Pos2>,
    fade_in: bool,
    layout: Layout,
    sizing_pass: bool,
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
            pivot: Align2::LEFT_TOP,
            anchor: None,
            fade_in: true,
            layout: Layout::default(),
            sizing_pass: false,
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

    /// Constrains this area to [`Context::screen_rect`]?
    ///
    /// Default: `true`.
    #[inline]
    pub fn constrain(mut self, constrain: bool) -> Self {
        self.constrain = constrain;
        self
    }

    /// Constrain the movement of the window to the given rectangle.
    ///
    /// For instance: `.constrain_to(ctx.screen_rect())`.
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
    /// Default: [`Align2::LEFT_TOP`].
    #[inline]
    pub fn pivot(mut self, pivot: Align2) -> Self {
        self.pivot = pivot;
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

    pub(crate) fn get_pivot(&self) -> Align2 {
        if let Some((pivot, _)) = self.anchor {
            pivot
        } else {
            Align2::LEFT_TOP
        }
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
        } = self;

        let constrain_rect = constrain_rect.unwrap_or_else(|| ctx.content_rect());

        let layer_id = LayerId::new(order, id);

        let state = AreaState::load(ctx, id);
        let mut sizing_pass = state.is_none();
        let mut state = state.unwrap_or(AreaState {
            pivot_pos: None,
            pivot,
            size: None,
            interactable,
            last_became_visible_at: None,
        });
        if force_sizing_pass {
            sizing_pass = true;
            state.size = None;
        }
        state.pivot = pivot;
        state.interactable = interactable;
        if let Some(new_pos) = new_pos {
            state.pivot_pos = Some(new_pos);
        }
        state.pivot_pos.get_or_insert_with(|| {
            default_pos.unwrap_or_else(|| automatic_area_position(ctx, constrain_rect, layer_id))
        });
        state.interactable = interactable;

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

        // TODO(emilk): if last frame was sizing pass, it should be considered invisible for smoother fade-in
        let visible_last_frame = ctx.memory(|mem| mem.areas().visible_last_frame(&layer_id));

        if !visible_last_frame || state.last_became_visible_at.is_none() {
            state.last_became_visible_at = Some(ctx.input(|i| i.time));
        }

        if let Some((anchor, offset)) = anchor {
            state.set_left_top_pos(
                anchor
                    .align_size_within_rect(size, constrain_rect)
                    .left_top()
                    + offset,
            );
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

            let move_response = ctx.create_widget(
                WidgetRect {
                    id: interact_id,
                    layer_id,
                    rect: state.rect(),
                    interact_rect: state.rect().intersect(constrain_rect),
                    sense,
                    enabled,
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
        }
    }
}

fn round_area_position(ctx: &Context, pos: Pos2) -> Pos2 {
    // We round a lot of rendering to pixels, so we round the whole
    // area positions to pixels too, so avoid widgets appearing to float
    // around independently of each other when the area is dragged.
    // But just in case pixels_per_point is irrational,
    // we then also round to ui coordinates:

    pos.round_to_pixels(ctx.pixels_per_point()).round_ui()
}

impl Prepared {
    pub(crate) fn state(&self) -> &AreaState {
        &self.state
    }

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
                ctx.input(|i| (i.time - last_became_visible_at) as f32 + i.predicted_dt / 2.0);
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

fn automatic_area_position(ctx: &Context, constrain_rect: Rect, layer_id: LayerId) -> Pos2 {
    let mut existing: Vec<Rect> = ctx.memory(|mem| {
        mem.areas()
            .visible_windows()
            .filter(|(id, _)| id != &layer_id) // ignore ourselves
            .filter(|(_, state)| state.pivot_pos.is_some() && state.size.is_some())
            .map(|(_, state)| state.rect())
            .collect()
    });
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
