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
    Align, Context, CursorIcon, Frame, Id, InnerResponse, Layout, NumExt as _, Rangef, Rect, Sense,
    Stroke, Ui, UiBuilder, UiKind, UiStackInfo, Vec2, lerp,
};

fn animate_expansion(ctx: &Context, id: Id, is_expanded: bool) -> f32 {
    ctx.animate_bool_responsive(id, is_expanded)
}

/// State regarding panels.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PanelState {
    /// The _outer_ rect of the panel, i.e. including the [`Frame`] margin & border.
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
    /// Useful as an index into [`Vec2`]/[`Pos2`]/[`Rect::size`].
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
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::Panel::left("my_left_panel").show_inside(ui, |ui| {
///    ui.label("Hello World!");
/// });
/// # });
/// ```
#[must_use = "You should call .show_inside()"]
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
    pub fn show_inside<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.show_inside_dyn(ui, Box::new(add_contents))
    }

    /// Show the panel if `is_expanded` is `true`,
    /// otherwise don't show it, but with a nice animation between collapsed and expanded.
    pub fn show_animated_inside<R>(
        self,
        ui: &mut Ui,
        is_expanded: bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let how_expanded = animate_expansion(ui, self.id.with("animation"), is_expanded);

        if how_expanded == 0.0 {
            // Make sure the ids of the next widgets are the same whether we show the panel or not:
            ui.skip_ahead_auto_ids(1);
            return None;
        }

        if how_expanded < 1.0 {
            // Show a fake panel in this in-between animation state:
            // TODO(emilk): move the panel out-of-screen instead of changing its width.
            // Then we can actually paint it as it animates.
            let fake_size = how_expanded * self.outer_size(ui);
            self.into_fake_animating(fake_size)
                .show_inside(ui, |_ui| {});
            return None;
        }

        Some(self.show_inside(ui, add_contents))
    }

    /// Show either a collapsed or a expanded panel, with a nice animation between.
    pub fn show_animated_between_inside<R>(
        ui: &mut Ui,
        is_expanded: bool,
        collapsed_panel: Self,
        expanded_panel: Self,
        add_contents: impl FnOnce(&mut Ui, f32) -> R,
    ) -> InnerResponse<R> {
        let how_expanded = animate_expansion(ui, expanded_panel.id.with("animation"), is_expanded);

        let panel = if how_expanded == 0.0 {
            collapsed_panel
        } else if how_expanded < 1.0 {
            let collapsed_size = collapsed_panel.outer_size(ui);
            let expanded_size = expanded_panel.outer_size(ui);
            let fake_size = lerp(collapsed_size..=expanded_size, how_expanded);
            expanded_panel.into_fake_animating(fake_size)
        } else {
            expanded_panel
        };

        panel.show_inside(ui, |ui| add_contents(ui, how_expanded))
    }
}

// Private methods to support the various show methods
impl Panel {
    /// Show the panel inside a [`Ui`].
    fn show_inside_dyn<'c, R>(
        self,
        parent_ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let side = self.side;
        let id = self.id;
        let resizable = self.resizable;
        let show_separator_line = self.show_separator_line;
        let outer_size_range = self.outer_size_range;

        let frame = self
            .frame
            .unwrap_or_else(|| Frame::side_top_panel(parent_ui.style()));
        let available_rect = parent_ui.available_rect_before_wrap();
        let mut outer_size = self.initial_outer_size(parent_ui, frame);
        let mut outer_rect = self.compute_outer_rect(available_rect, outer_size);

        // Check for duplicate id
        parent_ui.check_for_id_clash(id, outer_rect, "Panel");

        if resizable {
            // Resolve the resize interaction first to avoid frame latency in the resize.
            let resize_id = id.with("__resize");
            if let Some(resize_response) = parent_ui.read_response(resize_id)
                && resize_response.dragged()
                && let Some(pointer) = resize_response.interact_pointer_pos()
            {
                let axis = side.axis();
                outer_size = (pointer[axis] - side.fixed_pos(outer_rect)).abs();
                outer_size = clamp_to_range(outer_size, outer_size_range)
                    .at_most(available_rect.size_along(axis));
                side.set_rect_size(&mut outer_rect, outer_size);
            }
        }

        // NOTE(shark98): This must be **after** the resizable preparation, as the size
        // may change and round_ui() uses the size.
        outer_rect = outer_rect.round_ui();

        let mut panel_ui = parent_ui.new_child(
            UiBuilder::new()
                .id_salt(id)
                .ui_stack_info(UiStackInfo::new(side.ui_kind()))
                .max_rect(outer_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        panel_ui.expand_to_include_rect(outer_rect);
        panel_ui.set_clip_rect(outer_rect); // If we overflow, don't do so visibly (#4475)

        let axis = side.axis();
        let panel_axis_min =
            (outer_size_range.min - frame.total_margin().sum()[axis]).at_least(0.0);
        let inner_response = frame.show(&mut panel_ui, |content_ui| {
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

        // `Frame::show` returns the _outer_ rect (including margin & border).
        let outer_rect = inner_response.response.rect;

        {
            let mut cursor = parent_ui.cursor();
            match side {
                PanelSide::Left | PanelSide::Top => {
                    cursor.min[axis] = outer_rect.max[axis];
                }
                PanelSide::Right | PanelSide::Bottom => {
                    cursor.max[axis] = outer_rect.min[axis];
                }
            }
            parent_ui.set_cursor(cursor);
        }

        parent_ui.expand_to_include_rect(outer_rect);

        let (resize_hover, is_resizing) = if resizable {
            // Now we do the actual resize interaction, on top of all the contents,
            // otherwise its input could be eaten by the contents, e.g. a
            // `ScrollArea` on either side of the panel boundary.
            self.resize_panel(outer_rect, parent_ui)
        } else {
            (false, false)
        };

        if resize_hover || is_resizing {
            parent_ui.set_cursor_icon(self.cursor_icon(outer_size));
        }

        PanelState { outer_rect }.store(parent_ui, id);

        {
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
            let line_pos = side.resize_pos(outer_rect) + 0.5 * side.sign() * stroke.width;
            let cross_range = outer_rect.range_along(side.cross_axis());
            if axis == 0 {
                parent_ui.painter().vline(line_pos, cross_range, stroke);
            } else {
                parent_ui.painter().hline(cross_range, line_pos, stroke);
            }
        }

        inner_response
    }

    /// Outer size to start the frame with: from persisted state, or a sensible default.
    fn initial_outer_size(&self, ui: &Ui, frame: Frame) -> f32 {
        let axis = self.side.axis();
        if let Some(state) = PanelState::load(ui, self.id) {
            state.outer_rect.size_along(axis)
        } else {
            self.default_outer_size.unwrap_or_else(|| {
                ui.style().spacing.interact_size[axis] + frame.total_margin().sum()[axis]
            })
        }
    }

    /// Clamp `outer_size` to the allowed range / available space, then compute the panel rect.
    fn compute_outer_rect(&self, available_rect: Rect, mut outer_size: f32) -> Rect {
        let mut outer_rect = available_rect;
        outer_size = clamp_to_range(outer_size, self.outer_size_range)
            .at_most(available_rect.size_along(self.side.axis()));
        self.side.set_rect_size(&mut outer_rect, outer_size);
        outer_rect
    }

    fn resize_panel(&self, outer_rect: Rect, ui: &Ui) -> (bool, bool) {
        let resize_pos = self.side.resize_pos(outer_rect);
        let panel_axis_range = Rangef::point(resize_pos);
        let cross_range = outer_rect.range_along(self.side.cross_axis());
        let (resize_x, resize_y) = if self.side.axis() == 0 {
            (panel_axis_range, cross_range)
        } else {
            (cross_range, panel_axis_range)
        };
        let amount = ui.style().interaction.resize_grab_radius_side * self.side.axis_unit();

        let resize_id = self.id.with("__resize");
        let resize_rect = Rect::from_x_y_ranges(resize_x, resize_y).expand2(amount);
        let resize_response = ui.interact(resize_rect, resize_id, Sense::drag());

        (resize_response.hovered(), resize_response.dragged())
    }

    fn cursor_icon(&self, outer_size: f32) -> CursorIcon {
        if outer_size <= self.outer_size_range.min {
            // Can only grow (toward the resizable side):
            match self.side {
                PanelSide::Left => CursorIcon::ResizeEast,
                PanelSide::Right => CursorIcon::ResizeWest,
                PanelSide::Top => CursorIcon::ResizeSouth,
                PanelSide::Bottom => CursorIcon::ResizeNorth,
            }
        } else if outer_size < self.outer_size_range.max {
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

    /// Build a non-resizable, fixed-size clone of this panel for animating between sizes.
    ///
    /// Uses a distinct id so the resulting panel doesn't clash with the real one.
    fn into_fake_animating(self, outer_size: f32) -> Self {
        Self {
            id: self.id.with("animating_panel"),
            ..self
        }
        .resizable(false)
        .exact_size(outer_size)
    }

    /// Get the current _outer_ width or height of the panel (from previous frame),
    /// including the [`Frame`] margin & border,
    /// or fall back to some default.
    fn outer_size(&self, ctx: &Context) -> f32 {
        let axis = self.side.axis();
        if let Some(state) = PanelState::load(ctx, self.id) {
            state.outer_rect.size_along(axis)
        } else {
            ctx.global_style().spacing.interact_size[axis]
        }
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
/// egui::Panel::top("my_panel").show_inside(ui, |ui| {
///    ui.label("Hello World! From `Panel`, that must be before `CentralPanel`!");
/// });
/// egui::CentralPanel::default().show_inside(ui, |ui| {
///    ui.label("Hello World!");
/// });
/// # });
/// ```
#[must_use = "You should call .show_inside()"]
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
