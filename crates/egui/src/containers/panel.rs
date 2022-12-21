//! Panels are [`Ui`] regions taking up e.g. the left side of a [`Ui`] or screen.
//!
//! Panels can either be a child of a [`Ui`] (taking up a portion of the parent)
//! or be top-level (taking up a portion of the whole screen).
//!
//! Together with [`Window`] and [`Area`]:s, top-level panels are
//! the only places where you can put you widgets.
//!
//! The order in which you add panels matter!
//! The first panel you add will always be the outermost, and the last you add will always be the innermost.
//!
//! You must never open one top-level panel from within another panel. Add one panel, then the next.
//!
//! ⚠ Always add any [`CentralPanel`] last.
//!
//! Add your [`Window`]:s after any top-level panels.

use std::ops::RangeInclusive;

use crate::*;

/// State regarding panels.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PanelState {
    pub rect: Rect,
}

impl PanelState {
    pub fn load(ctx: &Context, bar_id: Id) -> Option<Self> {
        ctx.data().get_persisted(bar_id)
    }

    /// The size of the panel (from previous frame).
    pub fn size(&self) -> Vec2 {
        self.rect.size()
    }

    fn store(self, ctx: &Context, bar_id: Id) {
        ctx.data().insert_persisted(bar_id, self);
    }
}

// ----------------------------------------------------------------------------

/// [`Left`](Side::Left) or [`Right`](Side::Right)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
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

/// A panel that covers the entire left or right side of a [`Ui`] or screen.
///
/// The order in which you add panels matter!
/// The first panel you add will always be the outermost, and the last you add will always be the innermost.
///
/// ⚠ Always add any [`CentralPanel`] last.
///
/// See the [module level docs](crate::containers::panel) for more details.
///
/// ```
/// # egui::__run_test_ctx(|ctx| {
/// egui::SidePanel::left("my_left_panel").show(ctx, |ui| {
///    ui.label("Hello World!");
/// });
/// # });
/// ```
///
/// See also [`TopBottomPanel`].
#[must_use = "You should call .show()"]
pub struct SidePanel {
    side: Side,
    id: Id,
    frame: Option<Frame>,
    resizable: bool,
    show_separator_line: bool,
    default_width: f32,
    width_range: RangeInclusive<f32>,
}

impl SidePanel {
    /// The id should be globally unique, e.g. `Id::new("my_left_panel")`.
    pub fn left(id: impl Into<Id>) -> Self {
        Self::new(Side::Left, id)
    }

    /// The id should be globally unique, e.g. `Id::new("my_right_panel")`.
    pub fn right(id: impl Into<Id>) -> Self {
        Self::new(Side::Right, id)
    }

    /// The id should be globally unique, e.g. `Id::new("my_panel")`.
    pub fn new(side: Side, id: impl Into<Id>) -> Self {
        Self {
            side,
            id: id.into(),
            frame: None,
            resizable: true,
            show_separator_line: true,
            default_width: 200.0,
            width_range: 96.0..=f32::INFINITY,
        }
    }

    /// Can panel be resized by dragging the edge of it?
    ///
    /// Default is `true`.
    ///
    /// If you want your panel to be resizable you also need a widget in it that
    /// takes up more space as you resize it, such as:
    /// * Wrapping text ([`Ui::horizontal_wrapped`]).
    /// * A [`ScrollArea`].
    /// * A [`Separator`].
    /// * A [`TextEdit`].
    /// * …
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Show a separator line, even when not interacting with it?
    ///
    /// Default: `true`.
    pub fn show_separator_line(mut self, show_separator_line: bool) -> Self {
        self.show_separator_line = show_separator_line;
        self
    }

    /// The initial wrapping width of the [`SidePanel`].
    pub fn default_width(mut self, default_width: f32) -> Self {
        self.default_width = default_width;
        self.width_range = self.width_range.start().at_most(default_width)
            ..=self.width_range.end().at_least(default_width);
        self
    }

    /// Minimum width of the panel.
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.width_range = min_width..=self.width_range.end().at_least(min_width);
        self
    }

    /// Maximum width of the panel.
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.width_range = self.width_range.start().at_most(max_width)..=max_width;
        self
    }

    /// The allowable width range for the panel.
    pub fn width_range(mut self, width_range: RangeInclusive<f32>) -> Self {
        self.default_width = clamp_to_range(self.default_width, width_range.clone());
        self.width_range = width_range;
        self
    }

    /// Enforce this exact width.
    pub fn exact_width(mut self, width: f32) -> Self {
        self.default_width = width;
        self.width_range = width..=width;
        self
    }

    /// Change the background color, margins, etc.
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }
}

impl SidePanel {
    /// Show the panel inside a [`Ui`].
    pub fn show_inside<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.show_inside_dyn(ui, Box::new(add_contents))
    }

    /// Show the panel inside a [`Ui`].
    fn show_inside_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let Self {
            side,
            id,
            frame,
            resizable,
            show_separator_line,
            default_width,
            width_range,
        } = self;

        let available_rect = ui.available_rect_before_wrap();
        let mut panel_rect = available_rect;
        {
            let mut width = default_width;
            if let Some(state) = PanelState::load(ui.ctx(), id) {
                width = state.rect.width();
            }
            width = clamp_to_range(width, width_range.clone()).at_most(available_rect.width());
            side.set_rect_width(&mut panel_rect, width);
            ui.ctx().check_for_id_clash(id, panel_rect, "SidePanel");
        }

        let mut resize_hover = false;
        let mut is_resizing = false;
        if resizable {
            let resize_id = id.with("__resize");
            if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                let we_are_on_top = ui
                    .ctx()
                    .layer_id_at(pointer)
                    .map_or(true, |top_layer_id| top_layer_id == ui.layer_id());

                let resize_x = side.opposite().side_x(panel_rect);
                let mouse_over_resize_line = we_are_on_top
                    && panel_rect.y_range().contains(&pointer.y)
                    && (resize_x - pointer.x).abs()
                        <= ui.style().interaction.resize_grab_radius_side;

                let any_pressed = ui.input().pointer.any_pressed(); // avoid deadlocks
                if any_pressed && ui.input().pointer.any_down() && mouse_over_resize_line {
                    ui.memory().set_dragged_id(resize_id);
                }
                is_resizing = ui.memory().is_being_dragged(resize_id);
                if is_resizing {
                    let width = (pointer.x - side.side_x(panel_rect)).abs();
                    let width =
                        clamp_to_range(width, width_range.clone()).at_most(available_rect.width());
                    side.set_rect_width(&mut panel_rect, width);
                }

                let any_down = ui.input().pointer.any_down(); // avoid deadlocks
                let dragging_something_else = any_down || ui.input().pointer.any_pressed();
                resize_hover = mouse_over_resize_line && !dragging_something_else;

                if resize_hover || is_resizing {
                    ui.output().cursor_icon = CursorIcon::ResizeHorizontal;
                }
            }
        }

        let mut panel_ui = ui.child_ui_with_id_source(panel_rect, Layout::top_down(Align::Min), id);
        panel_ui.expand_to_include_rect(panel_rect);
        let frame = frame.unwrap_or_else(|| Frame::side_top_panel(ui.style()));
        let inner_response = frame.show(&mut panel_ui, |ui| {
            ui.set_min_height(ui.max_rect().height()); // Make sure the frame fills the full height
            ui.set_min_width(*width_range.start());
            add_contents(ui)
        });

        let rect = inner_response.response.rect;

        {
            let mut cursor = ui.cursor();
            match side {
                Side::Left => {
                    cursor.min.x = rect.max.x;
                }
                Side::Right => {
                    cursor.max.x = rect.min.x;
                }
            }
            ui.set_cursor(cursor);
        }
        ui.expand_to_include_rect(rect);

        PanelState { rect }.store(ui.ctx(), id);

        {
            let stroke = if is_resizing {
                ui.style().visuals.widgets.active.bg_stroke
            } else if resize_hover {
                ui.style().visuals.widgets.hovered.bg_stroke
            } else if show_separator_line {
                // TOOD(emilk): distinguish resizable from non-resizable
                ui.style().visuals.widgets.noninteractive.bg_stroke
            } else {
                Stroke::NONE
            };
            // TODO(emilk): draw line on top of all panels in this ui when https://github.com/emilk/egui/issues/1516 is done
            // In the meantime: nudge the line so its inside the panel, so it won't be covered by neighboring panel
            // (hence the shrink).
            let resize_x = side.opposite().side_x(rect.shrink(1.0));
            let resize_x = ui.painter().round_to_pixel(resize_x);
            ui.painter().vline(resize_x, rect.y_range(), stroke);
        }

        inner_response
    }

    /// Show the panel at the top level.
    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.show_dyn(ctx, Box::new(add_contents))
    }

    /// Show the panel at the top level.
    fn show_dyn<'c, R>(
        self,
        ctx: &Context,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let layer_id = LayerId::background();
        let side = self.side;
        let available_rect = ctx.available_rect();
        let clip_rect = ctx.input().screen_rect();
        let mut panel_ui = Ui::new(ctx.clone(), layer_id, self.id, available_rect, clip_rect);

        let inner_response = self.show_inside_dyn(&mut panel_ui, add_contents);
        let rect = inner_response.response.rect;

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

    /// Show the panel if `is_expanded` is `true`,
    /// otherwise don't show it, but with a nice animation between collapsed and expanded.
    pub fn show_animated<R>(
        self,
        ctx: &Context,
        is_expanded: bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let how_expanded = ctx.animate_bool(self.id.with("animation"), is_expanded);

        if 0.0 == how_expanded {
            None
        } else if how_expanded < 1.0 {
            // Show a fake panel in this in-between animation state:
            // TODO(emilk): move the panel out-of-screen instead of changing its width.
            // Then we can actually paint it as it animates.
            let expanded_width = PanelState::load(ctx, self.id)
                .map_or(self.default_width, |state| state.rect.width());
            let fake_width = how_expanded * expanded_width;
            Self {
                id: self.id.with("animating_panel"),
                ..self
            }
            .resizable(false)
            .exact_width(fake_width)
            .show(ctx, |_ui| {});
            None
        } else {
            // Show the real panel:
            Some(self.show(ctx, add_contents))
        }
    }

    /// Show the panel if `is_expanded` is `true`,
    /// otherwise don't show it, but with a nice animation between collapsed and expanded.
    pub fn show_animated_inside<R>(
        self,
        ui: &mut Ui,
        is_expanded: bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let how_expanded = ui
            .ctx()
            .animate_bool(self.id.with("animation"), is_expanded);

        if 0.0 == how_expanded {
            None
        } else if how_expanded < 1.0 {
            // Show a fake panel in this in-between animation state:
            // TODO(emilk): move the panel out-of-screen instead of changing its width.
            // Then we can actually paint it as it animates.
            let expanded_width = PanelState::load(ui.ctx(), self.id)
                .map_or(self.default_width, |state| state.rect.width());
            let fake_width = how_expanded * expanded_width;
            Self {
                id: self.id.with("animating_panel"),
                ..self
            }
            .resizable(false)
            .exact_width(fake_width)
            .show_inside(ui, |_ui| {});
            None
        } else {
            // Show the real panel:
            Some(self.show_inside(ui, add_contents))
        }
    }

    /// Show either a collapsed or a expanded panel, with a nice animation between.
    pub fn show_animated_between<R>(
        ctx: &Context,
        is_expanded: bool,
        collapsed_panel: Self,
        expanded_panel: Self,
        add_contents: impl FnOnce(&mut Ui, f32) -> R,
    ) -> Option<InnerResponse<R>> {
        let how_expanded = ctx.animate_bool(expanded_panel.id.with("animation"), is_expanded);

        if 0.0 == how_expanded {
            Some(collapsed_panel.show(ctx, |ui| add_contents(ui, how_expanded)))
        } else if how_expanded < 1.0 {
            // Show animation:
            let collapsed_width = PanelState::load(ctx, collapsed_panel.id)
                .map_or(collapsed_panel.default_width, |state| state.rect.width());
            let expanded_width = PanelState::load(ctx, expanded_panel.id)
                .map_or(expanded_panel.default_width, |state| state.rect.width());
            let fake_width = lerp(collapsed_width..=expanded_width, how_expanded);
            Self {
                id: expanded_panel.id.with("animating_panel"),
                ..expanded_panel
            }
            .resizable(false)
            .exact_width(fake_width)
            .show(ctx, |ui| add_contents(ui, how_expanded));
            None
        } else {
            Some(expanded_panel.show(ctx, |ui| add_contents(ui, how_expanded)))
        }
    }

    /// Show either a collapsed or a expanded panel, with a nice animation between.
    pub fn show_animated_between_inside<R>(
        ui: &mut Ui,
        is_expanded: bool,
        collapsed_panel: Self,
        expanded_panel: Self,
        add_contents: impl FnOnce(&mut Ui, f32) -> R,
    ) -> InnerResponse<R> {
        let how_expanded = ui
            .ctx()
            .animate_bool(expanded_panel.id.with("animation"), is_expanded);

        if 0.0 == how_expanded {
            collapsed_panel.show_inside(ui, |ui| add_contents(ui, how_expanded))
        } else if how_expanded < 1.0 {
            // Show animation:
            let collapsed_width = PanelState::load(ui.ctx(), collapsed_panel.id)
                .map_or(collapsed_panel.default_width, |state| state.rect.width());
            let expanded_width = PanelState::load(ui.ctx(), expanded_panel.id)
                .map_or(expanded_panel.default_width, |state| state.rect.width());
            let fake_width = lerp(collapsed_width..=expanded_width, how_expanded);
            Self {
                id: expanded_panel.id.with("animating_panel"),
                ..expanded_panel
            }
            .resizable(false)
            .exact_width(fake_width)
            .show_inside(ui, |ui| add_contents(ui, how_expanded))
        } else {
            expanded_panel.show_inside(ui, |ui| add_contents(ui, how_expanded))
        }
    }
}

// ----------------------------------------------------------------------------

/// [`Top`](TopBottomSide::Top) or [`Bottom`](TopBottomSide::Bottom)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopBottomSide {
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

/// A panel that covers the entire top or bottom of a [`Ui`] or screen.
///
/// The order in which you add panels matter!
/// The first panel you add will always be the outermost, and the last you add will always be the innermost.
///
/// ⚠ Always add any [`CentralPanel`] last.
///
/// See the [module level docs](crate::containers::panel) for more details.
///
/// ```
/// # egui::__run_test_ctx(|ctx| {
/// egui::TopBottomPanel::top("my_panel").show(ctx, |ui| {
///    ui.label("Hello World!");
/// });
/// # });
/// ```
///
/// See also [`SidePanel`].
#[must_use = "You should call .show()"]
pub struct TopBottomPanel {
    side: TopBottomSide,
    id: Id,
    frame: Option<Frame>,
    resizable: bool,
    show_separator_line: bool,
    default_height: Option<f32>,
    height_range: RangeInclusive<f32>,
}

impl TopBottomPanel {
    /// The id should be globally unique, e.g. `Id::new("my_top_panel")`.
    pub fn top(id: impl Into<Id>) -> Self {
        Self::new(TopBottomSide::Top, id)
    }

    /// The id should be globally unique, e.g. `Id::new("my_bottom_panel")`.
    pub fn bottom(id: impl Into<Id>) -> Self {
        Self::new(TopBottomSide::Bottom, id)
    }

    /// The id should be globally unique, e.g. `Id::new("my_panel")`.
    pub fn new(side: TopBottomSide, id: impl Into<Id>) -> Self {
        Self {
            side,
            id: id.into(),
            frame: None,
            resizable: false,
            show_separator_line: true,
            default_height: None,
            height_range: 20.0..=f32::INFINITY,
        }
    }

    /// Can panel be resized by dragging the edge of it?
    ///
    /// Default is `false`.
    ///
    /// If you want your panel to be resizable you also need a widget in it that
    /// takes up more space as you resize it, such as:
    /// * Wrapping text ([`Ui::horizontal_wrapped`]).
    /// * A [`ScrollArea`].
    /// * A [`Separator`].
    /// * A [`TextEdit`].
    /// * …
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Show a separator line, even when not interacting with it?
    ///
    /// Default: `true`.
    pub fn show_separator_line(mut self, show_separator_line: bool) -> Self {
        self.show_separator_line = show_separator_line;
        self
    }

    /// The initial height of the [`SidePanel`].
    /// Defaults to [`style::Spacing::interact_size`].y.
    pub fn default_height(mut self, default_height: f32) -> Self {
        self.default_height = Some(default_height);
        self.height_range = self.height_range.start().at_most(default_height)
            ..=self.height_range.end().at_least(default_height);
        self
    }

    /// Minimum height of the panel.
    pub fn min_height(mut self, min_height: f32) -> Self {
        self.height_range = min_height..=self.height_range.end().at_least(min_height);
        self
    }

    /// Maximum height of the panel.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.height_range = self.height_range.start().at_most(max_height)..=max_height;
        self
    }

    /// The allowable height range for the panel.
    pub fn height_range(mut self, height_range: RangeInclusive<f32>) -> Self {
        self.default_height = self
            .default_height
            .map(|default_height| clamp_to_range(default_height, height_range.clone()));
        self.height_range = height_range;
        self
    }

    /// Enforce this exact height.
    pub fn exact_height(mut self, height: f32) -> Self {
        self.default_height = Some(height);
        self.height_range = height..=height;
        self
    }

    /// Change the background color, margins, etc.
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }
}

impl TopBottomPanel {
    /// Show the panel inside a [`Ui`].
    pub fn show_inside<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.show_inside_dyn(ui, Box::new(add_contents))
    }

    /// Show the panel inside a [`Ui`].
    fn show_inside_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let Self {
            side,
            id,
            frame,
            resizable,
            show_separator_line,
            default_height,
            height_range,
        } = self;

        let available_rect = ui.available_rect_before_wrap();
        let mut panel_rect = available_rect;
        {
            let mut height = if let Some(state) = PanelState::load(ui.ctx(), id) {
                state.rect.height()
            } else {
                default_height.unwrap_or_else(|| ui.style().spacing.interact_size.y)
            };
            height = clamp_to_range(height, height_range.clone()).at_most(available_rect.height());
            side.set_rect_height(&mut panel_rect, height);
            ui.ctx()
                .check_for_id_clash(id, panel_rect, "TopBottomPanel");
        }

        let mut resize_hover = false;
        let mut is_resizing = false;
        if resizable {
            let resize_id = id.with("__resize");
            let latest_pos = ui.input().pointer.latest_pos();
            if let Some(pointer) = latest_pos {
                let we_are_on_top = ui
                    .ctx()
                    .layer_id_at(pointer)
                    .map_or(true, |top_layer_id| top_layer_id == ui.layer_id());

                let resize_y = side.opposite().side_y(panel_rect);
                let mouse_over_resize_line = we_are_on_top
                    && panel_rect.x_range().contains(&pointer.x)
                    && (resize_y - pointer.y).abs()
                        <= ui.style().interaction.resize_grab_radius_side;

                if ui.input().pointer.any_pressed()
                    && ui.input().pointer.any_down()
                    && mouse_over_resize_line
                {
                    ui.memory().interaction.drag_id = Some(resize_id);
                }
                is_resizing = ui.memory().interaction.drag_id == Some(resize_id);
                if is_resizing {
                    let height = (pointer.y - side.side_y(panel_rect)).abs();
                    let height = clamp_to_range(height, height_range.clone())
                        .at_most(available_rect.height());
                    side.set_rect_height(&mut panel_rect, height);
                }

                let any_down = ui.input().pointer.any_down(); // avoid deadlocks
                let dragging_something_else = any_down || ui.input().pointer.any_pressed();
                resize_hover = mouse_over_resize_line && !dragging_something_else;

                if resize_hover || is_resizing {
                    ui.output().cursor_icon = CursorIcon::ResizeVertical;
                }
            }
        }

        let mut panel_ui = ui.child_ui_with_id_source(panel_rect, Layout::top_down(Align::Min), id);
        panel_ui.expand_to_include_rect(panel_rect);
        let frame = frame.unwrap_or_else(|| Frame::side_top_panel(ui.style()));
        let inner_response = frame.show(&mut panel_ui, |ui| {
            ui.set_min_width(ui.max_rect().width()); // Make the frame fill full width
            ui.set_min_height(*height_range.start());
            add_contents(ui)
        });

        let rect = inner_response.response.rect;

        {
            let mut cursor = ui.cursor();
            match side {
                TopBottomSide::Top => {
                    cursor.min.y = rect.max.y;
                }
                TopBottomSide::Bottom => {
                    cursor.max.y = rect.min.y;
                }
            }
            ui.set_cursor(cursor);
        }
        ui.expand_to_include_rect(rect);

        PanelState { rect }.store(ui.ctx(), id);

        {
            let stroke = if is_resizing {
                ui.style().visuals.widgets.active.bg_stroke
            } else if resize_hover {
                ui.style().visuals.widgets.hovered.bg_stroke
            } else if show_separator_line {
                // TOOD(emilk): distinguish resizable from non-resizable
                ui.style().visuals.widgets.noninteractive.bg_stroke
            } else {
                Stroke::NONE
            };
            // TODO(emilk): draw line on top of all panels in this ui when https://github.com/emilk/egui/issues/1516 is done
            // In the meantime: nudge the line so its inside the panel, so it won't be covered by neighboring panel
            // (hence the shrink).
            let resize_y = side.opposite().side_y(rect.shrink(1.0));
            let resize_y = ui.painter().round_to_pixel(resize_y);
            ui.painter().hline(rect.x_range(), resize_y, stroke);
        }

        inner_response
    }

    /// Show the panel at the top level.
    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.show_dyn(ctx, Box::new(add_contents))
    }

    /// Show the panel at the top level.
    fn show_dyn<'c, R>(
        self,
        ctx: &Context,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let layer_id = LayerId::background();
        let available_rect = ctx.available_rect();
        let side = self.side;

        let clip_rect = ctx.input().screen_rect();
        let mut panel_ui = Ui::new(ctx.clone(), layer_id, self.id, available_rect, clip_rect);

        let inner_response = self.show_inside_dyn(&mut panel_ui, add_contents);
        let rect = inner_response.response.rect;

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

    /// Show the panel if `is_expanded` is `true`,
    /// otherwise don't show it, but with a nice animation between collapsed and expanded.
    pub fn show_animated<R>(
        self,
        ctx: &Context,
        is_expanded: bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let how_expanded = ctx.animate_bool(self.id.with("animation"), is_expanded);

        if 0.0 == how_expanded {
            None
        } else if how_expanded < 1.0 {
            // Show a fake panel in this in-between animation state:
            // TODO(emilk): move the panel out-of-screen instead of changing its height.
            // Then we can actually paint it as it animates.
            let expanded_height = PanelState::load(ctx, self.id)
                .map(|state| state.rect.height())
                .or(self.default_height)
                .unwrap_or_else(|| ctx.style().spacing.interact_size.y);
            let fake_height = how_expanded * expanded_height;
            Self {
                id: self.id.with("animating_panel"),
                ..self
            }
            .resizable(false)
            .exact_height(fake_height)
            .show(ctx, |_ui| {});
            None
        } else {
            // Show the real panel:
            Some(self.show(ctx, add_contents))
        }
    }

    /// Show the panel if `is_expanded` is `true`,
    /// otherwise don't show it, but with a nice animation between collapsed and expanded.
    pub fn show_animated_inside<R>(
        self,
        ui: &mut Ui,
        is_expanded: bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let how_expanded = ui
            .ctx()
            .animate_bool(self.id.with("animation"), is_expanded);

        if 0.0 == how_expanded {
            None
        } else if how_expanded < 1.0 {
            // Show a fake panel in this in-between animation state:
            // TODO(emilk): move the panel out-of-screen instead of changing its height.
            // Then we can actually paint it as it animates.
            let expanded_height = PanelState::load(ui.ctx(), self.id)
                .map(|state| state.rect.height())
                .or(self.default_height)
                .unwrap_or_else(|| ui.style().spacing.interact_size.y);
            let fake_height = how_expanded * expanded_height;
            Self {
                id: self.id.with("animating_panel"),
                ..self
            }
            .resizable(false)
            .exact_height(fake_height)
            .show_inside(ui, |_ui| {});
            None
        } else {
            // Show the real panel:
            Some(self.show_inside(ui, add_contents))
        }
    }

    /// Show either a collapsed or a expanded panel, with a nice animation between.
    pub fn show_animated_between<R>(
        ctx: &Context,
        is_expanded: bool,
        collapsed_panel: Self,
        expanded_panel: Self,
        add_contents: impl FnOnce(&mut Ui, f32) -> R,
    ) -> Option<InnerResponse<R>> {
        let how_expanded = ctx.animate_bool(expanded_panel.id.with("animation"), is_expanded);

        if 0.0 == how_expanded {
            Some(collapsed_panel.show(ctx, |ui| add_contents(ui, how_expanded)))
        } else if how_expanded < 1.0 {
            // Show animation:
            let collapsed_height = PanelState::load(ctx, collapsed_panel.id)
                .map(|state| state.rect.height())
                .or(collapsed_panel.default_height)
                .unwrap_or_else(|| ctx.style().spacing.interact_size.y);

            let expanded_height = PanelState::load(ctx, expanded_panel.id)
                .map(|state| state.rect.height())
                .or(expanded_panel.default_height)
                .unwrap_or_else(|| ctx.style().spacing.interact_size.y);

            let fake_height = lerp(collapsed_height..=expanded_height, how_expanded);
            Self {
                id: expanded_panel.id.with("animating_panel"),
                ..expanded_panel
            }
            .resizable(false)
            .exact_height(fake_height)
            .show(ctx, |ui| add_contents(ui, how_expanded));
            None
        } else {
            Some(expanded_panel.show(ctx, |ui| add_contents(ui, how_expanded)))
        }
    }

    /// Show either a collapsed or a expanded panel, with a nice animation between.
    pub fn show_animated_between_inside<R>(
        ui: &mut Ui,
        is_expanded: bool,
        collapsed_panel: Self,
        expanded_panel: Self,
        add_contents: impl FnOnce(&mut Ui, f32) -> R,
    ) -> InnerResponse<R> {
        let how_expanded = ui
            .ctx()
            .animate_bool(expanded_panel.id.with("animation"), is_expanded);

        if 0.0 == how_expanded {
            collapsed_panel.show_inside(ui, |ui| add_contents(ui, how_expanded))
        } else if how_expanded < 1.0 {
            // Show animation:
            let collapsed_height = PanelState::load(ui.ctx(), collapsed_panel.id)
                .map(|state| state.rect.height())
                .or(collapsed_panel.default_height)
                .unwrap_or_else(|| ui.style().spacing.interact_size.y);

            let expanded_height = PanelState::load(ui.ctx(), expanded_panel.id)
                .map(|state| state.rect.height())
                .or(expanded_panel.default_height)
                .unwrap_or_else(|| ui.style().spacing.interact_size.y);

            let fake_height = lerp(collapsed_height..=expanded_height, how_expanded);
            Self {
                id: expanded_panel.id.with("animating_panel"),
                ..expanded_panel
            }
            .resizable(false)
            .exact_height(fake_height)
            .show_inside(ui, |ui| add_contents(ui, how_expanded))
        } else {
            expanded_panel.show_inside(ui, |ui| add_contents(ui, how_expanded))
        }
    }
}

// ----------------------------------------------------------------------------

/// A panel that covers the remainder of the screen,
/// i.e. whatever area is left after adding other panels.
///
/// The order in which you add panels matter!
/// The first panel you add will always be the outermost, and the last you add will always be the innermost.
///
/// ⚠ [`CentralPanel`] must be added after all other panels!
///
/// NOTE: Any [`Window`]s and [`Area`]s will cover the top-level [`CentralPanel`].
///
/// See the [module level docs](crate::containers::panel) for more details.
///
/// ```
/// # egui::__run_test_ctx(|ctx| {
/// egui::TopBottomPanel::top("my_panel").show(ctx, |ui| {
///    ui.label("Hello World! From `TopBottomPanel`, that must be before `CentralPanel`!");
/// });
/// egui::CentralPanel::default().show(ctx, |ui| {
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
    /// Change the background color, margins, etc.
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }
}

impl CentralPanel {
    /// Show the panel inside a [`Ui`].
    pub fn show_inside<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.show_inside_dyn(ui, Box::new(add_contents))
    }

    /// Show the panel inside a [`Ui`].
    fn show_inside_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let Self { frame } = self;

        let panel_rect = ui.available_rect_before_wrap();
        let mut panel_ui = ui.child_ui(panel_rect, Layout::top_down(Align::Min));

        let frame = frame.unwrap_or_else(|| Frame::central_panel(ui.style()));
        frame.show(&mut panel_ui, |ui| {
            ui.expand_to_include_rect(ui.max_rect()); // Expand frame to include it all
            add_contents(ui)
        })
    }

    /// Show the panel at the top level.
    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.show_dyn(ctx, Box::new(add_contents))
    }

    /// Show the panel at the top level.
    fn show_dyn<'c, R>(
        self,
        ctx: &Context,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let available_rect = ctx.available_rect();
        let layer_id = LayerId::background();
        let id = Id::new("central_panel");

        let clip_rect = ctx.input().screen_rect();
        let mut panel_ui = Ui::new(ctx.clone(), layer_id, id, available_rect, clip_rect);

        let inner_response = self.show_inside_dyn(&mut panel_ui, add_contents);

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
