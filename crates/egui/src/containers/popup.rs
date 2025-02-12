use crate::{
    Area, AreaState, Context, Frame, Id, InnerResponse, Key, LayerId, Order, Response, Sense, Ui,
    UiKind,
};
use emath::{vec2, Align, Align2, Pos2, Rect, Vec2};
use std::iter::once;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupAnchor {
    Rect(Rect),
    Pointer,
    Position(Pos2),
}

impl From<Rect> for PopupAnchor {
    fn from(rect: Rect) -> Self {
        Self::Rect(rect)
    }
}

impl From<Pos2> for PopupAnchor {
    fn from(pos: Pos2) -> Self {
        Self::Position(pos)
    }
}

impl PopupAnchor {
    pub fn rect(self, ctx: &Context) -> Option<Rect> {
        match self {
            Self::Rect(rect) => Some(rect),
            Self::Pointer => ctx.pointer_hover_pos().map(Rect::from_pos),
            Self::Position(pos) => Some(Rect::from_pos(pos)),
        }
    }
}

/// Determines popup's close behavior
#[derive(Clone, Copy)]
pub enum PopupCloseBehavior {
    /// Popup will be closed on click anywhere, inside or outside the popup.
    ///
    /// It is used in [`crate::ComboBox`].
    CloseOnClick,

    /// Popup will be closed if the click happened somewhere else
    /// but in the popup's body
    CloseOnClickOutside,

    /// Clicks will be ignored. Popup might be closed manually by calling [`crate::Memory::close_popup`]
    /// or by pressing the escape button
    IgnoreClicks,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetOpen {
    DoNothing,
    Bool(bool),
    Toggle,
}

impl From<Option<bool>> for SetOpen {
    fn from(opt: Option<bool>) -> Self {
        match opt {
            Some(true) => Self::Bool(true),
            Some(false) => Self::Bool(false),
            None => Self::DoNothing,
        }
    }
}

impl From<bool> for SetOpen {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

enum OpenKind<'a> {
    Open,
    Closed,
    // TODO: Do we need this? Without we could get rid of the lifetime
    Bool(&'a mut bool, PopupCloseBehavior),
    Memory {
        set: SetOpen,
        close_behavior: PopupCloseBehavior,
    },
}

impl<'a> OpenKind<'a> {
    fn is_open(&self, id: Id, ctx: &Context) -> bool {
        match self {
            OpenKind::Open => true,
            OpenKind::Closed => false,
            OpenKind::Bool(open, _) => **open,
            OpenKind::Memory { .. } => ctx.memory(|mem| mem.is_popup_open(id)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupKind {
    Popup,
    Tooltip,
    Menu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    // TODO: Should we also support Center?
    Left,
    Right,
    Top,
    Bottom,
}

/// Similar to [`Align2`] but for aligning something to the outside of some rect.
/// ```text
///              ┌───────────┐  ┌────────┐  ┌─────────┐              
///              │ TOP_START │  │  TOP   │  │ TOP_END │              
///              └───────────┘  └────────┘  └─────────┘               
/// ┌──────────┐ ┌────────────────────────────────────┐ ┌───────────┐
/// │LEFT_START│ │                                    │ │RIGHT_START│
/// └──────────┘ │                                    │ └───────────┘
/// ┌──────────┐ │                                    │ ┌───────────┐
/// │   LEFT   │ │             some_rect              │ │   RIGHT   │
/// └──────────┘ │                                    │ └───────────┘
/// ┌──────────┐ │                                    │ ┌───────────┐
/// │ LEFT_END │ │                                    │ │ RIGHT_END │
/// └──────────┘ └────────────────────────────────────┘ └───────────┘
///              ┌────────────┐  ┌──────┐  ┌──────────┐              
///              │BOTTOM_START│  │BOTTOM│  │BOTTOM_END│              
///              └────────────┘  └──────┘  └──────────┘              
/// ```
// TODO: Find a better name for Position and PositionAlign
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PositionAlign(pub Position, pub Align);

impl Default for PositionAlign {
    fn default() -> Self {
        Self::BOTTOM_START
    }
}

impl PositionAlign {
    pub const TOP_START: Self = Self(Position::Top, Align::Min);
    pub const TOP: Self = Self(Position::Top, Align::Center);
    pub const TOP_END: Self = Self(Position::Top, Align::Max);
    pub const RIGHT_START: Self = Self(Position::Right, Align::Min);
    pub const RIGHT: Self = Self(Position::Right, Align::Center);
    pub const RIGHT_END: Self = Self(Position::Right, Align::Max);
    pub const BOTTOM_START: Self = Self(Position::Bottom, Align::Min);
    pub const BOTTOM: Self = Self(Position::Bottom, Align::Center);
    pub const BOTTOM_END: Self = Self(Position::Bottom, Align::Max);
    pub const LEFT_START: Self = Self(Position::Left, Align::Min);
    pub const LEFT: Self = Self(Position::Left, Align::Center);
    pub const LEFT_END: Self = Self(Position::Left, Align::Max);

    pub const ALL: [Self; 12] = [
        Self::TOP_START,
        Self::TOP_END,
        Self::RIGHT_START,
        Self::RIGHT_END,
        Self::BOTTOM_END,
        Self::BOTTOM_START,
        Self::LEFT_END,
        Self::LEFT_START,
        // These come last on purpose, we want to prefer the corner ones
        Self::TOP,
        Self::RIGHT,
        Self::BOTTOM,
        Self::LEFT,
    ];

    pub fn align_rect(&self, rect: Rect, gap: f32, size: Vec2) -> Rect {
        let (pivot, anchor) = self.pivot_anchor(rect, gap);
        pivot.anchor_size(anchor, size)
    }

    pub fn pivot_anchor(&self, rect: Rect, gap: f32) -> (Align2, Pos2) {
        (self.pivot_align(), self.anchor(rect, gap))
    }

    pub fn pivot_align(&self) -> Align2 {
        match *self {
            Self::TOP => Align2::CENTER_BOTTOM,
            Self::RIGHT => Align2::LEFT_CENTER,
            Self::BOTTOM => Align2::CENTER_TOP,
            Self::LEFT => Align2::RIGHT_CENTER,
            Self::TOP_START | Self::RIGHT_END => Align2::LEFT_BOTTOM,
            Self::TOP_END | Self::LEFT_END => Align2::RIGHT_BOTTOM,
            Self::RIGHT_START | Self::BOTTOM_START => Align2::LEFT_TOP,
            Self::LEFT_START | Self::BOTTOM_END => Align2::RIGHT_TOP,
        }
    }

    pub fn anchor(&self, rect: Rect, gap: f32) -> Pos2 {
        let mut anchor = match *self {
            Self::TOP => rect.center_top(),
            Self::RIGHT => rect.right_center(),
            Self::BOTTOM => rect.center_bottom(),
            Self::LEFT => rect.left_center(),
            Self::TOP_START | Self::LEFT_START => rect.left_top(),
            Self::TOP_END | Self::RIGHT_START => rect.right_top(),
            Self::RIGHT_END | Self::BOTTOM_END => rect.right_bottom(),
            Self::BOTTOM_START | Self::LEFT_END => rect.left_bottom(),
        };
        match self.0 {
            Position::Top => anchor.y -= gap,
            Position::Right => anchor.x += gap,
            Position::Bottom => anchor.y += gap,
            Position::Left => anchor.x -= gap,
        }
        anchor
    }

    fn alternatives(&self) -> [Self; 4] {
        let Self(pos, align) = *self;
        let mirrored_pos = match pos {
            Position::Top => Position::Bottom,
            Position::Right => Position::Left,
            Position::Bottom => Position::Top,
            Position::Left => Position::Right,
        };
        let mirrored_align = match align {
            Align::Min => Align::Max,
            Align::Center => Align::Center,
            Align::Max => Align::Min,
        };
        [
            Self(mirrored_pos, align),
            Self(pos, mirrored_align),
            Self(mirrored_pos, mirrored_align),
            Self(pos, Align::Center),
        ]
    }

    /// Look for the [`PositionAlign`] that fits best in the available space.
    /// Starts with `self` and `self.alternatives()`, then tries all other positions.
    pub fn find_best_align(
        mut values_to_try: impl Iterator<Item = Self>,
        available_space: Rect,
        widget_rect: Rect,
        gap: f32,
        size: Vec2,
    ) -> Self {
        let area = size.x * size.y;

        let blocked_area = |pos: Self| {
            let rect = pos.align_rect(widget_rect, gap, size);
            area - available_space.intersect(rect).area()
        };

        let first = values_to_try.next().unwrap_or_default();

        if blocked_area(first) == 0.0 {
            return first;
        }

        let mut best_area = blocked_area(first);
        let mut best = first;

        for align in values_to_try {
            let blocked = blocked_area(align);
            if blocked == 0.0 {
                return align;
            }
            if blocked < best_area {
                best = align;
                best_area = blocked;
            }
        }

        best
    }
}

pub struct Popup<'a> {
    id: Id,
    pub anchor: PopupAnchor,
    position_align: PositionAlign,
    alternative_aligns: Option<&'a [PositionAlign]>,
    /// If multiple popups are shown with the same widget id, they will be laid out so they don't overlap.
    layer_id: LayerId,
    open_kind: OpenKind<'a>,
    kind: PopupKind,
    /// Gap between the anchor and the popup
    gap: f32,
    /// Used later depending on close behavior
    widget_clicked_elsewhere: bool,
    /// Default width passed to the Area
    width: Option<f32>,
    sense: Sense,
}

impl<'a> Popup<'a> {
    pub fn new(id: Id, anchor: impl Into<PopupAnchor>, layer_id: LayerId) -> Self {
        Self {
            id,
            position_align: PositionAlign::BOTTOM_START,
            alternative_aligns: None,
            anchor: anchor.into(),
            open_kind: OpenKind::Open,
            kind: PopupKind::Popup,
            layer_id,
            gap: 0.0,
            widget_clicked_elsewhere: false,
            width: None,
            sense: Sense::click(),
        }
    }

    pub fn kind(mut self, kind: PopupKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the position and alignment of the popup relative to the anchor.
    /// This is the default position, and will be used if it fits.
    /// See [`Self::position_alternatives`] for more on this.
    pub fn position(mut self, position_align: PositionAlign) -> Self {
        self.position_align = position_align;
        self
    }

    /// Set alternative positions to try if the default one doesn't fit. Set to an empty slice to
    /// always use the position you set with [`Self::position`].
    /// By default, this will try the mirrored position and alignment, and then every other position
    pub fn position_alternatives(mut self, alternatives: &'a [PositionAlign]) -> Self {
        self.alternative_aligns = Some(alternatives);
        self
    }

    pub fn from_response(response: &Response) -> Self {
        // Transform layer coords to global coords:
        let mut widget_rect = response.rect;
        if let Some(to_global) = response.ctx.layer_transform_to_global(response.layer_id) {
            widget_rect = to_global * widget_rect;
        }
        Self {
            id: response.id.with("popup"),
            anchor: PopupAnchor::Rect(widget_rect),
            open_kind: OpenKind::Open,
            kind: PopupKind::Popup,
            layer_id: response.layer_id,
            position_align: PositionAlign::BOTTOM_START,
            alternative_aligns: None,
            gap: 0.0,
            widget_clicked_elsewhere: response.clicked_elsewhere(),
            width: Some(widget_rect.width()),
            sense: Sense::click(),
        }
    }

    pub fn menu(response: &Response) -> Self {
        Self::from_response(response).open_memory(
            if response.clicked() {
                SetOpen::Toggle
            } else {
                SetOpen::DoNothing
            },
            PopupCloseBehavior::CloseOnClick,
        )
    }

    pub fn context_menu(response: &Response) -> Self {
        Self::from_response(response)
            .open_memory(
                response.secondary_clicked().then_some(true),
                PopupCloseBehavior::CloseOnClick,
            )
            .at_pointer()
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open_kind = if open {
            OpenKind::Open
        } else {
            OpenKind::Closed
        };
        self
    }

    pub fn open_memory(
        mut self,
        set_state: impl Into<SetOpen>,
        close_behavior: PopupCloseBehavior,
    ) -> Self {
        self.open_kind = OpenKind::Memory {
            set: set_state.into(),
            close_behavior,
        };
        self
    }

    pub fn open_bool(mut self, open: &'a mut bool, close_behavior: PopupCloseBehavior) -> Self {
        self.open_kind = OpenKind::Bool(open, close_behavior);
        self
    }

    pub fn close_behavior(mut self, close_behavior: PopupCloseBehavior) -> Self {
        match &mut self.open_kind {
            OpenKind::Memory {
                close_behavior: behavior,
                ..
            }
            | OpenKind::Bool(_, behavior) => {
                *behavior = close_behavior;
            }
            _ => {}
        }
        self
    }

    pub fn at_pointer(mut self) -> Self {
        self.anchor = PopupAnchor::Pointer;
        self
    }

    pub fn at_position(mut self, position: Pos2) -> Self {
        self.anchor = PopupAnchor::Position(position);
        self
    }

    pub fn anchor(mut self, anchor: impl Into<PopupAnchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// The width that will be passed to [`Area::default_width`].
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    pub fn is_open(&self, ctx: &Context) -> bool {
        match &self.open_kind {
            OpenKind::Open => true,
            OpenKind::Closed => false,
            OpenKind::Bool(open, _) => **open,
            OpenKind::Memory { .. } => ctx.memory(|mem| mem.is_popup_open(self.id)),
        }
    }

    pub fn get_best_align(&self, ctx: &Context) -> PositionAlign {
        let expected_tooltip_size = AreaState::load(ctx, self.id)
            .and_then(|area| area.size)
            .unwrap_or(vec2(self.width.unwrap_or(0.0), 0.0));

        let Some(anchor_rect) = self.anchor.rect(ctx) else {
            return self.position_align;
        };

        PositionAlign::find_best_align(
            #[allow(clippy::iter_on_empty_collections)]
            once(self.position_align).chain(
                self.alternative_aligns
                    // Need the empty slice so the iters have the same type so we can unwrap_or
                    .map(|a| a.iter().copied().chain([].iter().copied()))
                    .unwrap_or(
                        self.position_align
                            .alternatives()
                            .iter()
                            .copied()
                            .chain(PositionAlign::ALL.iter().copied()),
                    ),
            ),
            ctx.screen_rect(),
            anchor_rect,
            self.gap,
            expected_tooltip_size,
        )
    }

    /// Returns `None` if the popup is not open or anchor is `PopupAnchor::Pointer` and there is
    /// no pointer.
    pub fn show<R>(
        self,
        ctx: &Context,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let best_align = self.get_best_align(ctx);

        let Popup {
            id,
            anchor,
            open_kind,
            kind,
            layer_id,
            position_align: _,
            alternative_aligns: _,
            gap,
            widget_clicked_elsewhere,
            width,
            sense,
        } = self;

        if let OpenKind::Memory { set, .. } = open_kind {
            ctx.memory_mut(|mem| match set {
                SetOpen::DoNothing => {}
                SetOpen::Bool(open) => {
                    if open {
                        mem.open_popup(id);
                    } else {
                        mem.close_popup();
                    }
                }
                SetOpen::Toggle => {
                    mem.toggle_popup(id);
                }
            });
        }

        if !open_kind.is_open(id, ctx) {
            return None;
        }

        let (ui_kind, order) = match kind {
            PopupKind::Popup => (UiKind::Popup, Order::Foreground),
            PopupKind::Tooltip => (UiKind::Tooltip, Order::Tooltip),
            PopupKind::Menu => (UiKind::Menu, Order::Foreground),
        };

        if kind == PopupKind::Popup {
            ctx.pass_state_mut(|fs| {
                fs.layers
                    .entry(layer_id)
                    .or_default()
                    .open_popups
                    .insert(id)
            });
        }

        let anchor_rect = anchor.rect(ctx)?;

        let (pivot, anchor) = best_align.pivot_anchor(anchor_rect, gap);

        let mut area = Area::new(id)
            .order(order)
            .kind(ui_kind)
            .pivot(pivot)
            .fixed_pos(anchor)
            .sense(sense);

        if let Some(width) = width {
            area = area.default_width(width);
        }

        let response = area.show(ctx, |ui| Frame::popup(&ctx.style()).show(ui, content).inner);

        let should_close = |close_behavior| {
            let should_close = match close_behavior {
                PopupCloseBehavior::CloseOnClick => widget_clicked_elsewhere,
                PopupCloseBehavior::CloseOnClickOutside => {
                    widget_clicked_elsewhere && response.response.clicked_elsewhere()
                }
                PopupCloseBehavior::IgnoreClicks => false,
            };

            should_close || ctx.input(|i| i.key_pressed(Key::Escape))
        };

        match open_kind {
            OpenKind::Open | OpenKind::Closed => {}
            OpenKind::Bool(open, close_behavior) => {
                if should_close(close_behavior) {
                    *open = false;
                }
            }
            OpenKind::Memory { close_behavior, .. } => {
                if should_close(close_behavior) {
                    ctx.memory_mut(|mem| mem.close_popup());
                }
            }
        }

        Some(response)
    }
}
