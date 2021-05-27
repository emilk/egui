//! Panels are fixed [`Ui`] regions.
//!
//! Together with [`Window`] and [`Area`]:s they are
//! the only places where you can put you widgets.
//!
//! The order in which you add panels matter!
//! The first panel you add will always be the outermost, and the last you add will always be the innermost.
//!
//! Always add any [`CentralPanel`] and [`Window`]:s last.

use std::ops::RangeInclusive;

use crate::*;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct PanelState {
    rect: Rect,
}

// ----------------------------------------------------------------------------

/// `Left` or `Right`
#[derive(Clone, Copy, Debug, PartialEq)]
enum Side {
    Left,
    Right,
}

impl Side {
    fn opposite(self) -> Self {
        match self {
            Side::Left => Self::Right,
            Side::Right => Self::Left,
        }
    }

    fn set_rect_width(self, rect: &mut Rect, width: f32) {
        match self {
            Side::Left => rect.max.x = rect.min.x + width,
            Side::Right => rect.min.x = rect.max.x - width,
        }
    }

    fn side_x(self, rect: Rect) -> f32 {
        match self {
            Side::Left => rect.left(),
            Side::Right => rect.right(),
        }
    }
}

/// A panel that covers the entire left or right side of the screen.
///
/// The order in which you add panels matter!
/// The first panel you add will always be the outermost, and the last you add will always be the innermost.
///
/// Always add any [`CentralPanel`] and [`Window`]:s last.
///
/// ```
/// # let mut ctx = egui::CtxRef::default();
/// # ctx.begin_frame(Default::default());
/// # let ctx = &ctx;
/// egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
///    ui.label("Hello World!");
/// });
/// ```
///
/// See also [`TopBottomPanel`].
#[must_use = "You should call .show()"]
pub struct SidePanel {
    side: Side,
    id: Id,
    frame: Option<Frame>,
    resizable: bool,
    default_width: f32,
    width_range: RangeInclusive<f32>,
}

impl SidePanel {
    /// `id_source`: Something unique, e.g. `"my_left_panel"`.
    pub fn left(id_source: impl std::hash::Hash) -> Self {
        Self::new(Side::Left, id_source)
    }

    /// `id_source`: Something unique, e.g. `"my_right_panel"`.
    pub fn right(id_source: impl std::hash::Hash) -> Self {
        Self::new(Side::Right, id_source)
    }

    /// `id_source`: Something unique, e.g. `"my_panel"`.
    fn new(side: Side, id_source: impl std::hash::Hash) -> Self {
        Self {
            side,
            id: Id::new(id_source),
            frame: None,
            resizable: true,
            default_width: 200.0,
            width_range: 96.0..=f32::INFINITY,
        }
    }

    /// Switch resizable on/off.
    /// Default is `true`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// The initial wrapping width of the `SidePanel`.
    pub fn default_width(mut self, default_width: f32) -> Self {
        self.default_width = default_width;
        self
    }

    pub fn min_width(mut self, min_width: f32) -> Self {
        self.width_range = min_width..=(*self.width_range.end());
        self
    }

    pub fn max_width(mut self, max_width: f32) -> Self {
        self.width_range = (*self.width_range.start())..=max_width;
        self
    }

    /// The allowable width range for resizable panels.
    pub fn width_range(mut self, width_range: RangeInclusive<f32>) -> Self {
        self.width_range = width_range;
        self
    }

    /// Change the background color, margins, etc.
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }
}

impl SidePanel {
    pub fn show<R>(
        self,
        ctx: &CtxRef,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let Self {
            side,
            id,
            frame,
            resizable,
            default_width,
            width_range,
        } = self;

        let layer_id = LayerId::background();

        let available_rect = ctx.available_rect();
        let mut panel_rect = available_rect;
        {
            let mut width = default_width;
            if let Some(state) = ctx.memory().id_data.get::<PanelState>(&id) {
                width = state.rect.width();
            }
            width = clamp_to_range(width, width_range.clone()).at_most(available_rect.width());
            side.set_rect_width(&mut panel_rect, width);
        }

        let mut resize_hover = false;
        let mut is_resizing = false;
        if resizable {
            let resize_id = id.with("__resize");
            if let Some(pointer) = ctx.input().pointer.latest_pos() {
                let resize_x = side.opposite().side_x(panel_rect);
                let mouse_over_resize_line = panel_rect.y_range().contains(&pointer.y)
                    && (resize_x - pointer.x).abs()
                        <= ctx.style().interaction.resize_grab_radius_side;

                if ctx.input().pointer.any_pressed()
                    && ctx.input().pointer.any_down()
                    && mouse_over_resize_line
                {
                    ctx.memory().interaction.drag_id = Some(resize_id);
                }
                is_resizing = ctx.memory().interaction.drag_id == Some(resize_id);
                if is_resizing {
                    let width = (pointer.x - side.side_x(panel_rect)).abs();
                    let width = clamp_to_range(width, width_range).at_most(available_rect.width());
                    side.set_rect_width(&mut panel_rect, width);
                }

                let we_are_on_top = ctx
                    .layer_id_at(pointer)
                    .map_or(true, |top_layer_id| top_layer_id == layer_id);
                let dragging_something_else =
                    ctx.input().pointer.any_down() || ctx.input().pointer.any_pressed();
                resize_hover = mouse_over_resize_line && !dragging_something_else && we_are_on_top;

                if resize_hover || is_resizing {
                    ctx.output().cursor_icon = CursorIcon::ResizeHorizontal;
                }
            }
        }

        let clip_rect = ctx.input().screen_rect();
        let mut panel_ui = Ui::new(ctx.clone(), layer_id, id, panel_rect, clip_rect);

        let frame = frame.unwrap_or_else(|| Frame::side_top_panel(&ctx.style()));
        let inner_response = frame.show(&mut panel_ui, |ui| {
            ui.set_min_height(ui.max_rect_finite().height()); // Make sure the frame fills the full height
            add_contents(ui)
        });

        let rect = inner_response.response.rect;
        ctx.memory().id_data.insert(id, PanelState { rect });

        if resize_hover || is_resizing {
            let stroke = if is_resizing {
                ctx.style().visuals.widgets.active.bg_stroke
            } else {
                ctx.style().visuals.widgets.hovered.bg_stroke
            };
            // draw on top of ALL panels so that the resize line won't be covered by subsequent panels
            let resize_layer = LayerId::new(Order::PanelResizeLine, Id::new("panel_resize"));
            let resize_x = side.opposite().side_x(rect);
            let top = pos2(resize_x, rect.top());
            let bottom = pos2(resize_x, rect.bottom());
            ctx.layer_painter(resize_layer)
                .line_segment([top, bottom], stroke);
        }

        match side {
            Side::Left => ctx
                .frame_state()
                .allocate_left_panel(Rect::from_min_max(available_rect.min, rect.max)),
            Side::Right => ctx
                .frame_state()
                .allocate_right_panel(Rect::from_min_max(rect.min, available_rect.max)),
        }

        inner_response
    }
}

// ----------------------------------------------------------------------------

/// `Top` or `Bottom`
#[derive(Clone, Copy, Debug, PartialEq)]
enum TopBottomSide {
    Top,
    Bottom,
}

impl TopBottomSide {
    fn opposite(self) -> Self {
        match self {
            TopBottomSide::Top => Self::Bottom,
            TopBottomSide::Bottom => Self::Top,
        }
    }

    fn set_rect_height(self, rect: &mut Rect, height: f32) {
        match self {
            TopBottomSide::Top => rect.max.y = rect.min.y + height,
            TopBottomSide::Bottom => rect.min.y = rect.max.y - height,
        }
    }

    fn side_y(self, rect: Rect) -> f32 {
        match self {
            TopBottomSide::Top => rect.top(),
            TopBottomSide::Bottom => rect.bottom(),
        }
    }
}

/// A panel that covers the entire top or bottom of the screen.
///
/// The order in which you add panels matter!
/// The first panel you add will always be the outermost, and the last you add will always be the innermost.
///
/// Always add any [`CentralPanel`] and [`Window`]:s last.
///
/// ```
/// # let mut ctx = egui::CtxRef::default();
/// # ctx.begin_frame(Default::default());
/// # let ctx = &ctx;
/// egui::TopBottomPanel::top("my_panel").show(ctx, |ui| {
///    ui.label("Hello World!");
/// });
/// ```
///
/// See also [`SidePanel`].
#[must_use = "You should call .show()"]
pub struct TopBottomPanel {
    side: TopBottomSide,
    id: Id,
    frame: Option<Frame>,
    resizable: bool,
    default_height: Option<f32>,
    height_range: RangeInclusive<f32>,
}

impl TopBottomPanel {
    /// `id_source`: Something unique, e.g. `"my_top_panel"`.
    pub fn top(id_source: impl std::hash::Hash) -> Self {
        Self::new(TopBottomSide::Top, id_source)
    }

    /// `id_source`: Something unique, e.g. `"my_bottom_panel"`.
    pub fn bottom(id_source: impl std::hash::Hash) -> Self {
        Self::new(TopBottomSide::Bottom, id_source)
    }

    /// `id_source`: Something unique, e.g. `"my_panel"`.
    fn new(side: TopBottomSide, id_source: impl std::hash::Hash) -> Self {
        Self {
            side,
            id: Id::new(id_source),
            frame: None,
            resizable: false,
            default_height: None,
            height_range: 20.0..=f32::INFINITY,
        }
    }

    /// Switch resizable on/off.
    /// Default is `false`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// The initial height of the `SidePanel`.
    /// Defaults to [`style::Spacing::interact_size`].y.
    pub fn default_height(mut self, default_height: f32) -> Self {
        self.default_height = Some(default_height);
        self
    }

    pub fn min_height(mut self, min_height: f32) -> Self {
        self.height_range = min_height..=(*self.height_range.end());
        self
    }

    pub fn max_height(mut self, max_height: f32) -> Self {
        self.height_range = (*self.height_range.start())..=max_height;
        self
    }

    /// The allowable height range for resizable panels.
    pub fn height_range(mut self, height_range: RangeInclusive<f32>) -> Self {
        self.height_range = height_range;
        self
    }

    /// Change the background color, margins, etc.
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }
}

impl TopBottomPanel {
    pub fn show<R>(
        self,
        ctx: &CtxRef,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let Self {
            side,
            id,
            frame,
            resizable,
            default_height,
            height_range,
        } = self;

        let layer_id = LayerId::background();

        let available_rect = ctx.available_rect();
        let mut panel_rect = available_rect;
        {
            let state = ctx.memory().id_data.get::<PanelState>(&id).copied();
            let mut height = if let Some(state) = state {
                state.rect.height()
            } else {
                default_height.unwrap_or_else(|| ctx.style().spacing.interact_size.y)
            };
            height = clamp_to_range(height, height_range.clone()).at_most(available_rect.height());
            side.set_rect_height(&mut panel_rect, height);
        }

        let mut resize_hover = false;
        let mut is_resizing = false;
        if resizable {
            let resize_id = id.with("__resize");
            if let Some(pointer) = ctx.input().pointer.latest_pos() {
                let resize_y = side.opposite().side_y(panel_rect);
                let mouse_over_resize_line = panel_rect.x_range().contains(&pointer.x)
                    && (resize_y - pointer.y).abs()
                        <= ctx.style().interaction.resize_grab_radius_side;

                if ctx.input().pointer.any_pressed()
                    && ctx.input().pointer.any_down()
                    && mouse_over_resize_line
                {
                    ctx.memory().interaction.drag_id = Some(resize_id);
                }
                is_resizing = ctx.memory().interaction.drag_id == Some(resize_id);
                if is_resizing {
                    let height = (pointer.y - side.side_y(panel_rect)).abs();
                    let height =
                        clamp_to_range(height, height_range).at_most(available_rect.height());
                    side.set_rect_height(&mut panel_rect, height);
                }

                let we_are_on_top = ctx
                    .layer_id_at(pointer)
                    .map_or(true, |top_layer_id| top_layer_id == layer_id);
                let dragging_something_else =
                    ctx.input().pointer.any_down() || ctx.input().pointer.any_pressed();
                resize_hover = mouse_over_resize_line && !dragging_something_else && we_are_on_top;

                if resize_hover || is_resizing {
                    ctx.output().cursor_icon = CursorIcon::ResizeVertical;
                }
            }
        }

        let clip_rect = ctx.input().screen_rect();
        let mut panel_ui = Ui::new(ctx.clone(), layer_id, id, panel_rect, clip_rect);

        let frame = frame.unwrap_or_else(|| Frame::side_top_panel(&ctx.style()));
        let inner_response = frame.show(&mut panel_ui, |ui| {
            ui.set_min_width(ui.max_rect_finite().width()); // Make the frame fill full width
            add_contents(ui)
        });

        let rect = inner_response.response.rect;
        ctx.memory().id_data.insert(id, PanelState { rect });

        if resize_hover || is_resizing {
            let stroke = if is_resizing {
                ctx.style().visuals.widgets.active.bg_stroke
            } else {
                ctx.style().visuals.widgets.hovered.bg_stroke
            };
            // draw on top of ALL panels so that the resize line won't be covered by subsequent panels
            let resize_layer = LayerId::new(Order::PanelResizeLine, Id::new("panel_resize"));
            let resize_y = side.opposite().side_y(rect);
            let left = pos2(rect.left(), resize_y);
            let right = pos2(rect.right(), resize_y);
            ctx.layer_painter(resize_layer)
                .line_segment([left, right], stroke);
        }

        match side {
            TopBottomSide::Top => {
                ctx.frame_state()
                    .allocate_top_panel(Rect::from_min_max(available_rect.min, rect.max));
            }
            TopBottomSide::Bottom => {
                ctx.frame_state()
                    .allocate_bottom_panel(Rect::from_min_max(rect.min, available_rect.max));
            }
        }

        inner_response
    }
}

// ----------------------------------------------------------------------------

#[deprecated = "Use TopBottomPanel::top instead"]
pub struct TopPanel {}

#[allow(deprecated)]
impl TopPanel {
    #[deprecated = "Use TopBottomPanel::top instead"]
    pub fn top(id_source: impl std::hash::Hash) -> TopBottomPanel {
        TopBottomPanel::top(id_source)
    }
}

// ----------------------------------------------------------------------------

/// A panel that covers the remainder of the screen,
/// i.e. whatever area is left after adding other panels.
///
/// `CentralPanel` must be added after all other panels.
/// Any [`Window`]s and [`Area`]s will cover the `CentralPanel`.
///
/// ```
/// # let mut ctx = egui::CtxRef::default();
/// # ctx.begin_frame(Default::default());
/// # let ctx = &ctx;
/// egui::CentralPanel::default().show(ctx, |ui| {
///    ui.label("Hello World!");
/// });
/// ```
#[must_use = "You should call .show()"]
#[derive(Default)]
pub struct CentralPanel {
    frame: Option<Frame>,
}

impl CentralPanel {
    /// Change the background color, margins, etc.
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }
}

impl CentralPanel {
    pub fn show<R>(
        self,
        ctx: &CtxRef,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let Self { frame } = self;

        let panel_rect = ctx.available_rect();

        let layer_id = LayerId::background();
        let id = Id::new("central_panel");

        let clip_rect = ctx.input().screen_rect();
        let mut panel_ui = Ui::new(ctx.clone(), layer_id, id, panel_rect, clip_rect);

        let frame = frame.unwrap_or_else(|| Frame::central_panel(&ctx.style()));
        let inner_response = frame.show(&mut panel_ui, |ui| {
            ui.expand_to_include_rect(ui.max_rect()); // Expand frame to include it all
            add_contents(ui)
        });

        // Only inform ctx about what we actually used, so we can shrink the native window to fit.
        ctx.frame_state()
            .allocate_central_panel(inner_response.response.rect);

        inner_response
    }
}

fn clamp_to_range(x: f32, range: RangeInclusive<f32>) -> f32 {
    x.clamp(
        range.start().min(*range.end()),
        range.start().max(*range.end()),
    )
}
