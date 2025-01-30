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

use emath::{GuiRounding as _, Pos2};

use crate::{
    lerp, vec2, Align, Context, CursorIcon, Frame, Id, InnerResponse, LayerId, Layout, NumExt,
    Rangef, Rect, Sense, Stroke, Ui, UiBuilder, UiKind, UiStackInfo, Vec2,
};

fn animate_expansion(ctx: &Context, id: Id, is_expanded: bool) -> f32 {
    ctx.animate_bool_responsive(id, is_expanded)
}

/// State regarding panels.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PanelState {
    pub rect: Rect,
}

impl PanelState {
    pub fn load(ctx: &Context, bar_id: Id) -> Option<Self> {
        ctx.data_mut(|d| d.get_persisted(bar_id))
    }

    /// The size of the panel (from previous frame).
    pub fn size(&self) -> Vec2 {
        self.rect.size()
    }

    fn store(self, ctx: &Context, bar_id: Id) {
        ctx.data_mut(|d| d.insert_persisted(bar_id, self));
    }
}

// ----------------------------------------------------------------------------

/// [`Left`](VerticalSide::Left) or [`Right`](VerticalSide::Right)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalSide {
    Left,
    Right,
}

/// [`Top`](HorizontalSide::Top) or [`Bottom`](HorizontalSide::Bottom)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizontalSide {
    Top,
    Bottom,
}

/// [`Horizontal`](Side::Horizontal) or [`Vertical`](Side::Vertical)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Vertical(VerticalSide),
    Horizontal(HorizontalSide),
}

impl Side {
    fn opposite(self) -> Self {
        match self {
            Side::Vertical(side) => opposite_vertical(side),
            Side::Horizontal(side) => opposite_horizontal(side),
        }

        fn opposite_vertical(
            side: VerticalSide,
        ) -> Side {
            match side {
                VerticalSide::Left => {
                    Side::Vertical(
                        VerticalSide::Right,
                    )
                }
                VerticalSide::Right => {
                    Side::Vertical(
                        VerticalSide::Left,
                    )
                }
            }
        }

        fn opposite_horizontal(
            side: HorizontalSide,
        ) -> Side {
            match side {
                HorizontalSide::Top => {
                    Side::Horizontal(
                        HorizontalSide::Bottom,
                    )
                }
                HorizontalSide::Bottom => {
                    Side::Horizontal(
                        HorizontalSide::Top,
                    )
                }
            }
        }
    }

    fn set_rect_size(self, rect: &mut Rect, size: f32) {
        let set_rect_size_vertical = |side: VerticalSide| match side {
            VerticalSide::Left => rect.max.x = rect.min.x + size,
            VerticalSide::Right => rect.min.x = rect.max.x - size,
        };

        let set_rect_size_horizontal =
            |side: HorizontalSide| match side {
                HorizontalSide::Top => rect.max.y = rect.min.y + size,
                HorizontalSide::Bottom => {
                    rect.min.y = rect.max.y - size
                }
            };

        match self {
            Side::Vertical(side) => set_rect_size_vertical(side),
            Side::Horizontal(side) => set_rect_size_horizontal(side),
        }
    }

    fn side_axe(self, rect: Rect) -> f32 {
        match self {
            Side::Vertical(side) => side_axe_vertical(side, rect),
            Side::Horizontal(side) => side_axe_horizontal(side, rect),
        }

        fn side_axe_vertical(side: VerticalSide, rect: Rect) -> f32 {
            match side {
                VerticalSide::Left => rect.left(),
                VerticalSide::Right => rect.right(),
            }
        }

        fn side_axe_horizontal(
            side: HorizontalSide,
            rect: Rect,
        ) -> f32 {
            match side {
                HorizontalSide::Top => rect.top(),
                HorizontalSide::Bottom => rect.bottom(),
            }
        }
    }

    fn sign(self) -> f32 {
        match self {
            Side::Vertical(side) => sign_vertical(side),
            Side::Horizontal(side) => sign_horizontal(side),
        }

        fn sign_vertical(side: VerticalSide) -> f32 {
            match side {
                VerticalSide::Left => -1.0,
                VerticalSide::Right => 1.0,
            }
        }

        fn sign_horizontal(side: HorizontalSide) -> f32 {
            match side {
                HorizontalSide::Top => -1.0,
                HorizontalSide::Bottom => 1.0,
            }
        }
    }
}

// ----------------------------------------------------------------------------

/// Intermediate structure to abstract some portion of [`Panel::show_inside`](Panel::show_inside).
struct PanelSizer<'a> {
    panel: &'a Panel,
    frame: Frame,
    available_rect: Rect,
    size: f32,
    panel_rect: Rect,
}

impl PanelSizer {
    fn new(panel: &Panel, ui: &mut Ui) -> Self {
        let frame = panel
            .frame
            .unwrap_or_else(|| Frame::side_top_panel(ui.style()));
        let available_rect = ui.available_rect_before_wrap();
        let mut size = PanelSizer::get_size_from_state_or_default(panel, ui, frame);
        let mut panel_rect = PanelSizer::get_panel_rect(panel, available_rect, &mut size);

        Self {
            panel,
            frame,
            available_rect,
            size,
            panel_rect,
        }
    }

    fn get_size_from_state_or_default(panel: &Panel, ui: &mut Ui, frame: Frame) -> f32 {
        if let Some(state) = PanelState::load(ui.ctx(), panel.id) {
            match panel.side {
                Side::Vertical(_) => state.rect.width(),
                Side::Horizontal(_) => state.rect.height(),
            }
        } else {
            match panel.side {
                Side::Vertical(_) => panel.default_size.unwrap_or_else(|| {
                    ui.style().spacing.interact_size.x + frame.inner_margin.sum().x
                }),
                Side::Horizontal(_) => panel.default_size.unwrap_or_else(|| {
                    ui.style().spacing.interact_size.y + frame.inner_margin.sum().y
                }),
            }
        }
    }

    fn get_panel_rect(panel: &Panel, available_rect: Rect, mut size: &mut f32) -> Rect {
        let side = panel.side;
        let size_range = panel.size_range;

        let mut panel_rect = available_rect;

        match side {
            Side::Vertical(_) => {
                size = &mut clamp_to_range(*size, size_range).at_most(available_rect.width());
            }
            Side::Horizontal(_) => {
                size = &mut clamp_to_range(*size, size_range).at_most(available_rect.height());
            }
        }
        side.set_rect_size(&mut panel_rect, *size);
        panel_rect
    }

    fn prepare_resizing_response(&mut self, is_resizing: bool, pointer: Option<Pos2>) {
        let side = self.panel.side;
        let size_range = self.panel.size_range;

        let prepare_resizing_response_vertical = |pointer: Pos2| {
            self.size = (pointer.x - side.side_axe(self.panel_rect)).abs();
            self.size = clamp_to_range(self.size, size_range).at_most(self.available_rect.width());
        };

        let prepare_resizing_response_horizontal = |pointer: Pos2| {
            self.size = (pointer.y - side.side_axe(self.panel_rect)).abs();
            self.size = clamp_to_range(self.size, size_range).at_most(self.available_rect.height());
        };

        if is_resizing && pointer.is_some() {
            let pointer = pointer.unwrap();

            match side {
                Side::Vertical(_) => prepare_resizing_response_vertical(pointer),
                Side::Horizontal(_) => prepare_resizing_response_horizontal(pointer),
            }

            side.set_rect_size(&mut self.panel_rect, self.size);
        }
    }
}

// ----------------------------------------------------------------------------

/// A panel that covers an entire side
/// ([`Left`](VerticalSide::Left), [`Right`](VerticalSide::Right),
/// [`Top`](HorizontalSide::Top) or [`Bottom`](HorizontalSide::Bottom))
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
/// # egui::__run_test_ctx(|ctx| {
/// egui::Panel::left("my_left_panel").show(ctx, |ui| {
///    ui.label("Hello World!");
/// });
/// # });
/// ```
#[must_use = "You should call .show()"]
pub struct Panel {
    side: Side,
    id: Id,
    frame: Option<Frame>,
    resizable: bool,
    show_separator_line: bool,

    /// The size is defined as being either the width for a Vertical Panel
    /// or the height for a Horizontal Panel.
    default_size: Option<f32>,

    /// The size is defined as being either the width for a Vertical Panel
    /// or the height for a Horizontal Panel.
    size_range: Rangef,
}

impl Panel {
    /// Create a left panel.
    ///
    /// The id should be globally unique, e.g. `Id::new("my_left_panel")`.
    pub fn left(id: impl Into<Id>) -> Self {
        Self::new(Side::Vertical(VerticalSide::Left), id)
    }

    /// Create a right panel.
    ///
    /// The id should be globally unique, e.g. `Id::new("my_right_panel")`.
    pub fn right(id: impl Into<Id>) -> Self {
        Self::new(Side::Vertical(VerticalSide::Right), id)
    }

    /// Create a top panel.
    ///
    /// The id should be globally unique, e.g. `Id::new("my_top_panel")`.
    pub fn top(id: impl Into<Id>) -> Self {
        Self::new(Side::Horizontal(HorizontalSide::Top), id)
    }

    /// Create a bottom panel.
    ///
    /// The id should be globally unique, e.g. `Id::new("my_bottom_panel")`.
    pub fn bottom(id: impl Into<Id>) -> Self {
        Self::new(Side::Horizontal(HorizontalSide::Bottom), id)
    }

    /// Create a panel.
    ///
    /// The id should be globally unique, e.g. `Id::new("my_panel")`.
    pub fn new(side: Side, id: impl Into<Id>) -> Self {
        let default_size: Option<f32> = match side {
            Side::Vertical(_) => Some(200.0),
            Side::Horizontal(_) => None,
        };

        let size_range: Rangef = match side {
            Side::Vertical(_) => Rangef::new(96.0, f32::INFINITY),
            Side::Horizontal(_) => Rangef::new(20.0, f32::INFINITY),
        };

        Self {
            side,
            id: id.into(),
            frame: None,
            resizable: true,
            show_separator_line: true,
            default_size,
            size_range,
        }
    }

    /// Can panel be resized by dragging the edge of it?
    ///
    /// Default is `true`.
    ///
    /// If you want your panel to be resizable you also need a widget in it that
    /// takes up more space as you resize it, such as:
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
        self.default_size = Some(default_size);
        self.size_range = Rangef::new(
            self.size_range.min.at_most(default_size),
            self.size_range.max.at_least(default_size),
        );
        self
    }

    /// Minimum size of the panel, including margins.
    #[inline]
    pub fn min_size(mut self, min_size: f32) -> Self {
        self.size_range = Rangef::new(min_size, self.size_range.max.at_least(min_size));
        self
    }

    /// Maximum size of the panel, including margins.
    #[inline]
    pub fn max_size(mut self, max_size: f32) -> Self {
        self.size_range = Rangef::new(self.size_range.min.at_most(max_size), max_size);
        self
    }

    /// The allowable size range for the panel, including margins.
    #[inline]
    pub fn size_range(mut self, size_range: impl Into<Rangef>) -> Self {
        let size_range = size_range.into();
        self.default_size = self
            .default_size
            .map(|default_size| clamp_to_range(default_size, size_range));
        self.size_range = size_range;
        self
    }

    /// Enforce this exact size, including margins.
    #[inline]
    pub fn exact_size(mut self, size: f32) -> Self {
        self.default_size = Some(size);
        self.size_range = Rangef::point(size);
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

    /// Show the panel at the top level.
    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        self.show_dyn(ctx, Box::new(add_contents))
    }

    /// Show the panel if `is_expanded` is `true`,
    /// otherwise don't show it, but with a nice animation between collapsed and expanded.
    pub fn show_animated<R>(
        self,
        ctx: &Context,
        is_expanded: bool,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let how_expanded = animate_expansion(ctx(), self.id.with("animation"), is_expanded);

        let animated_panel = self.get_animated_panel(ctx(), is_expanded);

        if animated_panel.is_none() {
            None
        } else if how_expanded < 1.0 {
            // Show a fake panel in this in-between animation state:
            animated_panel.unwrap().show(ctx, |_ui| {});
            None
        } else {
            // Show the real panel:
            Some(animated_panel.unwrap().show(ctx, add_contents))
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
        let how_expanded = animate_expansion(ui.ctx(), self.id.with("animation"), is_expanded);

        // Get either the fake or the real panel to animate
        let animated_panel = self.get_animated_panel(ui.ctx(), is_expanded);

        if animated_panel.is_none() {
            None
        } else if how_expanded < 1.0 {
            // Show a fake panel in this in-between animation state:
            animated_panel.unwrap().show_inside(ui, |_ui| {});
            None
        } else {
            // Show the real panel:
            Some(animated_panel.unwrap().show_inside(ui, add_contents))
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
        let how_expanded = animate_expansion(ctx, expanded_panel.id.with("animation"), is_expanded);

        // Get either the fake or the real panel to animate
        let animated_between_panel =
            Self::get_animated_between_panel(ctx(), is_expanded, collapsed_panel, expanded_panel);

        if 0.0 == how_expanded {
            Some(animated_between_panel.show(ctx, |ui| add_contents(ui, how_expanded)))
        } else if how_expanded < 1.0 {
            // Show animation:
            animated_between_panel.show(ctx, |ui| add_contents(ui, how_expanded));
            None
        } else {
            Some(animated_between_panel.show(ctx, |ui| add_contents(ui, how_expanded)))
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
        let how_expanded =
            animate_expansion(ui.ctx(), expanded_panel.id.with("animation"), is_expanded);

        let animated_between_panel = Self::get_animated_between_panel(
            ui.ctx(),
            is_expanded,
            collapsed_panel,
            expanded_panel,
        );

        if 0.0 == how_expanded {
            animated_between_panel.show_inside(ui, |ui| add_contents(ui, how_expanded))
        } else if how_expanded < 1.0 {
            // Show animation:
            animated_between_panel.show_inside(ui, |ui| add_contents(ui, how_expanded))
        } else {
            animated_between_panel.show_inside(ui, |ui| add_contents(ui, how_expanded))
        }
    }
}

// Private methods to support the various show methods
impl Panel {
    /// Show the panel inside a [`Ui`].
    fn show_inside_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let side = self.side;
        let id = self.id;
        let resizable = self.resizable;
        let show_separator_line = self.show_separator_line;
        let size_range = self.size_range;

        // Define the sizing of the panel.
        let mut panel_sizer = PanelSizer::new(&self, ui);

        // Check for duplicate id
        ui.ctx()
            .check_for_id_clash(id, panel_sizer.panel_rect, "Panel");

        if self.resizable {
            // Prepare the resizable panel to avoid frame latency in the resize
            self.prepare_resizable_panel(&mut panel_sizer, ui);
        }

        // NOTE(shark98): This must be **after** the resizable preparation, as the size
        // may change and round_ui() uses the size.
        panel_sizer.panel_rect = panel_sizer.panel_rect.round_ui();

        let get_ui_kind = || match side {
            Side::Vertical(v_side) => match v_side {
                VerticalSide::Left => UiKind::LeftPanel,
                VerticalSide::Right => UiKind::RightPanel,
            },
            Side::Horizontal(h_side) => match h_side {
                HorizontalSide::Top => UiKind::TopPanel,
                HorizontalSide::Bottom => UiKind::BottomPanel,
            },
        };

        let mut panel_ui = ui.new_child(
            UiBuilder::new()
                .id_salt(id)
                .ui_stack_info(UiStackInfo::new(get_ui_kind()))
                .max_rect(panel_sizer.panel_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        panel_ui.expand_to_include_rect(panel_sizer.panel_rect);
        panel_ui.set_clip_rect(panel_sizer.panel_rect); // If we overflow, don't do so visibly (#4475)

        let inner_response = panel_sizer.frame.show(&mut panel_ui, |ui| {
            match side {
                Side::Vertical(_) => {
                    ui.set_min_height(ui.max_rect().height()); // Make sure the frame fills the full height
                    ui.set_min_width(
                        (size_range.min - panel_sizer.frame.inner_margin.sum().x).at_least(0.0),
                    );
                }
                Side::Horizontal(_) => {
                    ui.set_min_width(ui.max_rect().width()); // Make the frame fill full width
                    ui.set_min_height(
                        (size_range.min - panel_sizer.frame.inner_margin.sum().y).at_least(0.0),
                    );
                }
            }

            add_contents(ui)
        });

        let rect = inner_response.response.rect;

        {
            let mut cursor = ui.cursor();
            match side {
                Side::Vertical(v_side) => match v_side {
                    VerticalSide::Left => cursor.min.x = rect.max.x,
                    VerticalSide::Right => cursor.max.x = rect.min.x,
                },
                Side::Horizontal(h_side) => match h_side {
                    HorizontalSide::Top => cursor.min.y = rect.max.y,
                    HorizontalSide::Bottom => cursor.max.y = rect.min.y,
                },
            };
            ui.set_cursor(cursor);
        }

        ui.expand_to_include_rect(rect);

        let mut resize_hover = false;
        let mut is_resizing = false;
        if resizable {
            // Now we do the actual resize interaction, on top of all the contents,
            // otherwise its input could be eaten by the contents, e.g. a
            // `ScrollArea` on either side of the panel boundary.
            (resize_hover, is_resizing) = self.resize_panel(&mut panel_sizer, ui);
        }

        if resize_hover || is_resizing {
            ui.ctx().set_cursor_icon(self.get_cursor_icon(&panel_sizer));
        }

        PanelState { rect }.store(ui.ctx(), id);

        {
            let stroke = if is_resizing {
                ui.style().visuals.widgets.active.fg_stroke // highly visible
            } else if resize_hover {
                ui.style().visuals.widgets.hovered.fg_stroke // highly visible
            } else if show_separator_line {
                // TODO(emilk): distinguish resizable from non-resizable
                ui.style().visuals.widgets.noninteractive.bg_stroke // dim
            } else {
                Stroke::NONE
            };
            // TODO(emilk): draw line on top of all panels in this ui when https://github.com/emilk/egui/issues/1516 is done
            let resize_axe = side.opposite().side_axe(rect);
            let resize_axe = resize_axe + 0.5 * side.sign() * stroke.width;
            match side {
                Side::Vertical(_) => {
                    ui.painter()
                        .vline(resize_axe, panel_sizer.panel_rect.y_range(), stroke);
                }
                Side::Horizontal(_) => {
                    ui.painter()
                        .hline(panel_sizer.panel_rect.x_range(), resize_axe, stroke);
                }
            }
        }

        inner_response
    }

    /// Show the panel at the top level.
    fn show_dyn<'c, R>(
        self,
        ctx: &Context,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let side = self.side;
        let available_rect = ctx.available_rect();
        let mut panel_ui = Ui::new(
            ctx.clone(),
            self.id,
            UiBuilder::new()
                .layer_id(LayerId::background())
                .max_rect(available_rect),
        );
        panel_ui.set_clip_rect(ctx.screen_rect());

        let inner_response = self.show_inside_dyn(&mut panel_ui, add_contents);
        let rect = inner_response.response.rect;

        match side {
            Side::Vertical(v_side) => match v_side {
                VerticalSide::Left => ctx.pass_state_mut(|state| {
                    state.allocate_left_panel(Rect::from_min_max(available_rect.min, rect.max));
                }),
                VerticalSide::Right => ctx.pass_state_mut(|state| {
                    state.allocate_right_panel(Rect::from_min_max(rect.min, available_rect.max));
                }),
            },
            Side::Horizontal(h_side) => match h_side {
                HorizontalSide::Top => {
                    ctx.pass_state_mut(|state| {
                        state.allocate_top_panel(Rect::from_min_max(available_rect.min, rect.max));
                    });
                }
                HorizontalSide::Bottom => {
                    ctx.pass_state_mut(|state| {
                        state.allocate_bottom_panel(Rect::from_min_max(
                            rect.min,
                            available_rect.max,
                        ));
                    });
                }
            },
        }
        inner_response
    }

    fn prepare_resizable_panel(&self, panel_sizer: &mut PanelSizer, ui: &mut Ui) {
        let resize_id = self.id.with("__resize");
        let resize_response = ui.ctx().read_response(resize_id);

        if resize_response.is_some() {
            let resize_response = resize_response.unwrap();

            // NOTE(sharky98): The original code was initializing to
            // false first, but it doesn't seem necessary.
            let is_resizing = resize_response.dragged();
            let pointer = resize_response.interact_pointer_pos();
            panel_sizer.prepare_resizing_response(is_resizing, pointer);
        }
    }

    fn resize_panel(&self, panel_sizer: &mut PanelSizer, ui: &mut Ui) -> (bool, bool) {
        let (resize_x, resize_y, amnt): (impl Into<Rangef>, impl Into<Rangef>, Vec2) =
            match self.side {
                Side::Vertical(_) => {
                    let resize_x = self.side.opposite().side_axe(panel_sizer.panel_rect);
                    let resize_y = panel_sizer.panel_rect.y_range();
                    (
                        resize_x..=resize_x,
                        resize_y,
                        vec2(ui.style().interaction.resize_grab_radius_side, 0.0),
                    )
                }
                Side::Horizontal(_) => {
                    let resize_x = panel_sizer.panel_rect.x_range();
                    let resize_y = self.side.opposite().side_axe(panel_sizer.panel_rect);
                    (
                        resize_x,
                        resize_y..=resize_y,
                        vec2(0.0, ui.style().interaction.resize_grab_radius_side),
                    )
                }
            };

        let resize_id = self.id.with("__resize");
        let resize_rect = Rect::from_x_y_ranges(resize_x, resize_y).expand2(amnt);
        let resize_response = ui.interact(resize_rect, resize_id, Sense::drag());

        (resize_response.hovered(), resize_response.dragged())
    }

    fn get_cursor_icon(&self, panel_sizer: &PanelSizer) -> CursorIcon {
        if panel_sizer.size <= self.size_range.min {
            match self.side {
                Side::Vertical(v_side) => match v_side {
                    VerticalSide::Left => CursorIcon::ResizeEast,
                    VerticalSide::Right => CursorIcon::ResizeWest,
                },
                Side::Horizontal(h_side) => match h_side {
                    HorizontalSide::Top => CursorIcon::ResizeSouth,
                    HorizontalSide::Bottom => CursorIcon::ResizeNorth,
                },
            }
        } else if panel_sizer.size < self.size_range.max {
            match self.side {
                Side::Vertical(_) => CursorIcon::ResizeHorizontal,
                Side::Horizontal(_) => CursorIcon::ResizeVertical,
            }
        } else {
            match self.side {
                Side::Vertical(v_side) => match v_side {
                    VerticalSide::Left => CursorIcon::ResizeWest,
                    VerticalSide::Right => CursorIcon::ResizeEast,
                },
                Side::Horizontal(h_side) => match h_side {
                    HorizontalSide::Top => CursorIcon::ResizeNorth,
                    HorizontalSide::Bottom => CursorIcon::ResizeSouth,
                },
            }
        }
    }

    /// Get the real or fake panel to animate if `is_expanded` is `true`.
    fn get_animated_panel(self, ctx: &Context, is_expanded: bool) -> Option<Self> {
        let how_expanded = animate_expansion(ctx, self.id.with("animation"), is_expanded);

        if 0.0 == how_expanded {
            None
        } else if how_expanded < 1.0 {
            // Show a fake panel in this in-between animation state:
            // TODO(emilk): move the panel out-of-screen instead of changing its width.
            // Then we can actually paint it as it animates.
            let expanded_size = Self::get_animated_size(ctx, &self);
            let fake_size = how_expanded * expanded_size;
            Some(
                Self {
                    id: self.id.with("animating_panel"),
                    ..self
                }
                    .resizable(false)
                    .exact_size(fake_size),
            )
        } else {
            // Show the real panel:
            Some(self)
        }
    }

    /// Get either the collapsed or expended panel to animate.
    fn get_animated_between_panel(
        ctx: &Context,
        is_expanded: bool,
        collapsed_panel: Self,
        expanded_panel: Self,
    ) -> Self {
        let how_expanded = animate_expansion(ctx, expanded_panel.id.with("animation"), is_expanded);

        if 0.0 == how_expanded {
            collapsed_panel
        } else if how_expanded < 1.0 {
            let collapsed_size = Self::get_animated_size(ctx, &collapsed_panel);
            let expanded_size = Self::get_animated_size(ctx, &expanded_panel);

            let fake_size = lerp(collapsed_size..=expanded_size, how_expanded);

            Self {
                id: expanded_panel.id.with("animating_panel"),
                ..expanded_panel
            }
                .resizable(false)
                .exact_size(fake_size)
        } else {
            expanded_panel
        }
    }

    fn get_animated_size(ctx: &Context, panel: &Panel) -> f32 {
        let get_rect_state_size = |state: PanelState| match panel.side {
            Side::Vertical(_) => state.rect.width(),
            Side::Horizontal(_) => state.rect.height(),
        };

        let get_spacing_size = || match panel.side {
            Side::Vertical(_) => ctx.style().spacing.interact_size.x,
            Side::Horizontal(_) => ctx.style().spacing.interact_size.y,
        };

        PanelState::load(ctx, panel.id)
            .map(get_rect_state_size)
            .or(panel.default_size)
            .unwrap_or(get_spacing_size())
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
/// NOTE: Any [`crate::Window`]s and [`crate::Area`]s will cover the top-level [`CentralPanel`].
///
/// See the [module level docs](crate::containers::panel) for more details.
///
/// ```
/// # egui::__run_test_ctx(|ctx| {
/// egui::Panel::top("my_panel").show(ctx, |ui| {
///    ui.label("Hello World! From `Panel`, that must be before `CentralPanel`!");
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
    #[inline]
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
        let mut panel_ui = ui.new_child(
            UiBuilder::new()
                .ui_stack_info(UiStackInfo::new(UiKind::CentralPanel))
                .max_rect(panel_rect)
                .layout(Layout::top_down(Align::Min)),
        );
        panel_ui.set_clip_rect(panel_rect); // If we overflow, don't do so visibly (#4475)

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
        let id = Id::new((ctx.viewport_id(), "central_panel"));

        let mut panel_ui = Ui::new(
            ctx.clone(),
            id,
            UiBuilder::new()
                .layer_id(LayerId::background())
                .max_rect(ctx.available_rect().round_ui()),
        );
        panel_ui.set_clip_rect(ctx.screen_rect());

        let inner_response = self.show_inside_dyn(&mut panel_ui, add_contents);

        // Only inform ctx about what we actually used, so we can shrink the native window to fit.
        ctx.pass_state_mut(|state| state.allocate_central_panel(inner_response.response.rect));

        inner_response
    }
}

fn clamp_to_range(x: f32, range: Rangef) -> f32 {
    let range = range.as_positive();
    x.clamp(range.min, range.max)
}
