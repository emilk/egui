use crate::{
    Area, AreaState, Context, Frame, Id, InnerResponse, Key, LayerId, Layout, Order, Response,
    Sense, Ui, UiKind,
};
use emath::{vec2, Align, Pos2, Rect, RectAlign, Vec2};
use std::iter::once;

/// What should we anchor the popup to?
/// The final position for the popup will be calculated based on [`RectAlign`]
/// and can be customized with [`Popup::align`] and [`Popup::align_alternatives`].
/// [`PopupAnchor`] is the parent rect of [`RectAlign`].
///
/// For [`PopupAnchor::Pointer`], [`PopupAnchor::PointerFixed`] and [`PopupAnchor::Position`],
/// the rect will be derived via [`Rect::from_pos`] (so a zero-sized rect at the given position).
///
/// The rect should be in global coordinates. `PopupAnchor::from(&response)` will automatically
/// do this conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupAnchor {
    /// Show the popup relative to some parent [`Rect`].
    ParentRect(Rect),

    /// Show the popup relative to the mouse pointer.
    Pointer,

    /// Remember the mouse position and show the popup relative to that (like a context menu).
    PointerFixed,

    /// Show the popup relative to a specific position.
    Position(Pos2),
}

impl From<Rect> for PopupAnchor {
    fn from(rect: Rect) -> Self {
        Self::ParentRect(rect)
    }
}

impl From<Pos2> for PopupAnchor {
    fn from(pos: Pos2) -> Self {
        Self::Position(pos)
    }
}

impl From<&Response> for PopupAnchor {
    fn from(response: &Response) -> Self {
        let mut widget_rect = response.rect;
        if let Some(to_global) = response.ctx.layer_transform_to_global(response.layer_id) {
            widget_rect = to_global * widget_rect;
        }
        Self::ParentRect(widget_rect)
    }
}

impl PopupAnchor {
    /// Get the rect the popup should be shown relative to.
    /// Returns `Rect::from_pos` for [`PopupAnchor::Pointer`], [`PopupAnchor::PointerFixed`]
    /// and [`PopupAnchor::Position`] (so the rect will be zero-sized).
    pub fn rect(self, popup_id: Id, ctx: &Context) -> Option<Rect> {
        match self {
            Self::ParentRect(rect) => Some(rect),
            Self::Pointer => ctx.pointer_hover_pos().map(Rect::from_pos),
            Self::PointerFixed => ctx
                .memory(|mem| mem.popup_position(popup_id))
                .map(Rect::from_pos),
            Self::Position(pos) => Some(Rect::from_pos(pos)),
        }
    }
}

/// Determines popup's close behavior
#[derive(Clone, Copy, PartialEq, Eq)]
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
pub enum SetOpenCommand {
    /// Set the open state to the given value
    Bool(bool),

    /// Toggle the open state
    Toggle,
}

impl From<bool> for SetOpenCommand {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

/// How do we determine if the popup should be open or closed
enum OpenKind<'a> {
    /// Always open
    Open,

    /// Always closed
    Closed,

    /// Open if the bool is true
    Bool(&'a mut bool, PopupCloseBehavior),

    /// Store the open state via [`crate::Memory`]
    Memory {
        set: Option<SetOpenCommand>,
        close_behavior: PopupCloseBehavior,
    },
}

impl<'a> OpenKind<'a> {
    /// Returns `true` if the popup should be open
    fn is_open(&self, id: Id, ctx: &Context) -> bool {
        match self {
            OpenKind::Open => true,
            OpenKind::Closed => false,
            OpenKind::Bool(open, _) => **open,
            OpenKind::Memory { .. } => ctx.memory(|mem| mem.is_popup_open(id)),
        }
    }
}

/// Is the popup a popup, tooltip or menu?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupKind {
    Popup,
    Tooltip,
    Menu,
}

pub struct Popup<'a> {
    id: Id,
    ctx: Context,
    anchor: PopupAnchor,
    rect_align: RectAlign,
    alternative_aligns: Option<&'a [RectAlign]>,
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
    layout: Layout,
    frame: Option<Frame>,
}

impl<'a> Popup<'a> {
    /// Create a new popup
    pub fn new(id: Id, ctx: Context, anchor: impl Into<PopupAnchor>, layer_id: LayerId) -> Self {
        Self {
            id,
            ctx,
            anchor: anchor.into(),
            open_kind: OpenKind::Open,
            kind: PopupKind::Popup,
            layer_id,
            rect_align: RectAlign::BOTTOM_START,
            alternative_aligns: None,
            gap: 0.0,
            widget_clicked_elsewhere: false,
            width: None,
            sense: Sense::click(),
            layout: Layout::default(),
            frame: None,
        }
    }

    /// Set the kind of the popup. Used for [`Area::kind`] and [`Area::order`].
    #[inline]
    pub fn kind(mut self, kind: PopupKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the [`RectAlign`] of the popup relative to the [`PopupAnchor`].
    /// This is the default position, and will be used if it fits.
    /// See [`Self::align_alternatives`] for more on this.
    #[inline]
    pub fn align(mut self, position_align: RectAlign) -> Self {
        self.rect_align = position_align;
        self
    }

    /// Set alternative positions to try if the default one doesn't fit. Set to an empty slice to
    /// always use the position you set with [`Self::align`].
    /// By default, this will try [`RectAlign::symmetries`] and then [`RectAlign::MENU_ALIGNS`].
    #[inline]
    pub fn align_alternatives(mut self, alternatives: &'a [RectAlign]) -> Self {
        self.alternative_aligns = Some(alternatives);
        self
    }

    /// Show a popup relative to some widget.
    /// The popup will be always open.
    ///
    /// See [`Self::menu`] and [`Self::context_menu`] for common use cases.
    pub fn from_response(response: &Response) -> Self {
        let mut popup = Self::new(
            response.id.with("popup"),
            response.ctx.clone(),
            response,
            response.layer_id,
        );
        popup.widget_clicked_elsewhere = response.clicked_elsewhere();
        popup
    }

    /// Show a popup when the widget was clicked.
    /// Sets the layout to `Layout::top_down_justified(Align::Min)`.
    pub fn menu(response: &Response) -> Self {
        Self::from_response(response)
            .open_memory(
                if response.clicked() {
                    Some(SetOpenCommand::Toggle)
                } else {
                    None
                },
                PopupCloseBehavior::CloseOnClick,
            )
            .layout(Layout::top_down_justified(Align::Min))
    }

    /// Show a context menu when the widget was secondary clicked.
    /// Sets the layout to `Layout::top_down_justified(Align::Min)`.
    /// In contrast to [`Self::menu`], this will open at the pointer position.
    pub fn context_menu(response: &Response) -> Self {
        Self::from_response(response)
            .open_memory(
                response
                    .secondary_clicked()
                    .then_some(SetOpenCommand::Bool(true)),
                PopupCloseBehavior::CloseOnClick,
            )
            .layout(Layout::top_down_justified(Align::Min))
            .at_pointer_fixed()
    }

    /// Force the popup to be open or closed.
    #[inline]
    pub fn open(mut self, open: bool) -> Self {
        self.open_kind = if open {
            OpenKind::Open
        } else {
            OpenKind::Closed
        };
        self
    }

    /// Store the open state via [`crate::Memory`].
    /// You can set the state via the first [`SetOpenCommand`] param.
    #[inline]
    pub fn open_memory(
        mut self,
        set_state: impl Into<Option<SetOpenCommand>>,
        close_behavior: PopupCloseBehavior,
    ) -> Self {
        self.open_kind = OpenKind::Memory {
            set: set_state.into(),
            close_behavior,
        };
        self
    }

    /// Store the open state via a mutable bool.
    #[inline]
    pub fn open_bool(mut self, open: &'a mut bool, close_behavior: PopupCloseBehavior) -> Self {
        self.open_kind = OpenKind::Bool(open, close_behavior);
        self
    }

    /// Set the close behavior of the popup.
    ///
    /// This will do nothing if [`Popup::open`] was called.
    #[inline]
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

    /// Show the popup relative to the pointer.
    #[inline]
    pub fn at_pointer(mut self) -> Self {
        self.anchor = PopupAnchor::Pointer;
        self
    }

    /// Remember the pointer position at the time of opening the popup, and show the popup
    /// relative to that.
    #[inline]
    pub fn at_pointer_fixed(mut self) -> Self {
        self.anchor = PopupAnchor::PointerFixed;
        self
    }

    /// Show the popup relative to a specific position.
    #[inline]
    pub fn at_position(mut self, position: Pos2) -> Self {
        self.anchor = PopupAnchor::Position(position);
        self
    }

    /// Show the popup relative to the given [`PopupAnchor`].
    #[inline]
    pub fn anchor(mut self, anchor: impl Into<PopupAnchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Set the gap between the anchor and the popup.
    #[inline]
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Set the sense of the popup.
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Set the layout of the popup.
    #[inline]
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    /// The width that will be passed to [`Area::default_width`].
    #[inline]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the id of the Area.
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    /// Get the [`Context`]
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    /// Return the [`PopupAnchor`] of the popup.
    pub fn get_anchor(&self) -> PopupAnchor {
        self.anchor
    }

    /// Return the anchor rect of the popup.
    ///
    /// Returns `None` if the anchor is [`PopupAnchor::Pointer`] and there is no pointer.
    pub fn get_anchor_rect(&self) -> Option<Rect> {
        self.anchor.rect(self.id, &self.ctx)
    }

    /// Get the expected rect the popup will be shown in.
    ///
    /// Returns `None` if the popup wasn't shown before or anchor is `PopupAnchor::Pointer` and
    /// there is no pointer.
    pub fn get_popup_rect(&self) -> Option<Rect> {
        let size = self.get_expected_size();
        if let Some(size) = size {
            self.get_anchor_rect()
                .map(|anchor| self.get_best_align().align_rect(&anchor, size, self.gap))
        } else {
            None
        }
    }

    /// Get the id of the popup.
    pub fn get_id(&self) -> Id {
        self.id
    }

    /// Is the popup open?
    pub fn is_open(&self) -> bool {
        match &self.open_kind {
            OpenKind::Open => true,
            OpenKind::Closed => false,
            OpenKind::Bool(open, _) => **open,
            OpenKind::Memory { .. } => self.ctx.memory(|mem| mem.is_popup_open(self.id)),
        }
    }

    pub fn get_expected_size(&self) -> Option<Vec2> {
        AreaState::load(&self.ctx, self.id).and_then(|area| area.size)
    }

    /// Calculate the best alignment for the popup, based on the last size and screen rect.
    pub fn get_best_align(&self) -> RectAlign {
        let expected_popup_size = self
            .get_expected_size()
            .unwrap_or(vec2(self.width.unwrap_or(0.0), 0.0));

        let Some(anchor_rect) = self.anchor.rect(self.id, &self.ctx) else {
            return self.rect_align;
        };

        RectAlign::find_best_align(
            #[allow(clippy::iter_on_empty_collections)]
            once(self.rect_align).chain(
                self.alternative_aligns
                    // Need the empty slice so the iters have the same type so we can unwrap_or
                    .map(|a| a.iter().copied().chain([].iter().copied()))
                    .unwrap_or(
                        self.rect_align
                            .symmetries()
                            .iter()
                            .copied()
                            .chain(RectAlign::MENU_ALIGNS.iter().copied()),
                    ),
            ),
            self.ctx.screen_rect(),
            anchor_rect,
            self.gap,
            expected_popup_size,
        )
    }

    /// Show the popup.
    /// Returns `None` if the popup is not open or anchor is `PopupAnchor::Pointer` and there is
    /// no pointer.
    pub fn show<R>(self, content: impl FnOnce(&mut Ui) -> R) -> Option<InnerResponse<R>> {
        let best_align = self.get_best_align();

        let Popup {
            id,
            ctx,
            anchor,
            open_kind,
            kind,
            layer_id,
            rect_align: _,
            alternative_aligns: _,
            gap,
            widget_clicked_elsewhere,
            width,
            sense,
            layout,
            frame,
        } = self;

        let hover_pos = ctx.pointer_hover_pos();
        if let OpenKind::Memory { set, .. } = open_kind {
            ctx.memory_mut(|mem| match set {
                Some(SetOpenCommand::Bool(open)) => {
                    if open {
                        match self.anchor {
                            PopupAnchor::PointerFixed => {
                                mem.open_popup_at(id, hover_pos);
                            }
                            _ => mem.open_popup(id),
                        }
                    } else {
                        mem.close_popup();
                    }
                }
                Some(SetOpenCommand::Toggle) => {
                    mem.toggle_popup(id);
                }
                None => {}
            });
        }

        if !open_kind.is_open(id, &ctx) {
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

        let anchor_rect = anchor.rect(id, &ctx)?;

        let (pivot, anchor) = best_align.pivot_pos(&anchor_rect, gap);

        let mut area = Area::new(id)
            .order(order)
            .kind(ui_kind)
            .pivot(pivot)
            .fixed_pos(anchor)
            .sense(sense)
            .layout(layout);

        if let Some(width) = width {
            area = area.default_width(width);
        }

        let frame = frame.unwrap_or_else(|| Frame::popup(&ctx.style()));

        let response = area.show(&ctx, |ui| frame.show(ui, content).inner);

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
