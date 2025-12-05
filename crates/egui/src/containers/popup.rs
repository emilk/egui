#![expect(deprecated)] // This is a new, safe wrapper around the old `Memory::popup` API.

use std::iter::once;

use emath::{Align, Pos2, Rect, RectAlign, Vec2, vec2};

use crate::{
    Area, AreaState, Context, Frame, Id, InnerResponse, Key, LayerId, Layout, Order, Response,
    Sense, Ui, UiKind, UiStackInfo,
    containers::menu::{MenuConfig, MenuState, menu_style},
    style::StyleModifier,
};

/// What should we anchor the popup to?
///
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
        // We use interact_rect so we don't show the popup relative to some clipped point
        let mut widget_rect = response.interact_rect;
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
            Self::PointerFixed => Popup::position_of_id(ctx, popup_id).map(Rect::from_pos),
            Self::Position(pos) => Some(Rect::from_pos(pos)),
        }
    }
}

/// Determines popup's close behavior
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum PopupCloseBehavior {
    /// Popup will be closed on click anywhere, inside or outside the popup.
    ///
    /// It is used in [`crate::ComboBox`] and in [`crate::containers::menu`]s.
    #[default]
    CloseOnClick,

    /// Popup will be closed if the click happened somewhere else
    /// but in the popup's body
    CloseOnClickOutside,

    /// Clicks will be ignored. Popup might be closed manually by calling [`crate::Memory::close_all_popups`]
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
    Bool(&'a mut bool),

    /// Store the open state via [`crate::Memory`]
    Memory { set: Option<SetOpenCommand> },
}

impl OpenKind<'_> {
    /// Returns `true` if the popup should be open
    fn is_open(&self, popup_id: Id, ctx: &Context) -> bool {
        match self {
            OpenKind::Open => true,
            OpenKind::Closed => false,
            OpenKind::Bool(open) => **open,
            OpenKind::Memory { .. } => Popup::is_id_open(ctx, popup_id),
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

impl PopupKind {
    /// Returns the order to be used with this kind.
    pub fn order(self) -> Order {
        match self {
            Self::Tooltip => Order::Tooltip,
            Self::Menu | Self::Popup => Order::Foreground,
        }
    }
}

impl From<PopupKind> for UiKind {
    fn from(kind: PopupKind) -> Self {
        match kind {
            PopupKind::Popup => Self::Popup,
            PopupKind::Tooltip => Self::Tooltip,
            PopupKind::Menu => Self::Menu,
        }
    }
}

/// A popup container.
#[must_use = "Call `.show()` to actually display the popup"]
pub struct Popup<'a> {
    id: Id,
    ctx: Context,
    anchor: PopupAnchor,
    rect_align: RectAlign,
    alternative_aligns: Option<&'a [RectAlign]>,
    layer_id: LayerId,
    open_kind: OpenKind<'a>,
    close_behavior: PopupCloseBehavior,
    info: Option<UiStackInfo>,
    kind: PopupKind,

    /// Gap between the anchor and the popup
    gap: f32,

    /// Default width passed to the Area
    width: Option<f32>,
    sense: Sense,
    layout: Layout,
    frame: Option<Frame>,
    style: StyleModifier,
}

impl<'a> Popup<'a> {
    /// Create a new popup
    pub fn new(id: Id, ctx: Context, anchor: impl Into<PopupAnchor>, layer_id: LayerId) -> Self {
        Self {
            id,
            ctx,
            anchor: anchor.into(),
            open_kind: OpenKind::Open,
            close_behavior: PopupCloseBehavior::default(),
            info: None,
            kind: PopupKind::Popup,
            layer_id,
            rect_align: RectAlign::BOTTOM_START,
            alternative_aligns: None,
            gap: 0.0,
            width: None,
            sense: Sense::click(),
            layout: Layout::default(),
            frame: None,
            style: StyleModifier::default(),
        }
    }

    /// Show a popup relative to some widget.
    /// The popup will be always open.
    ///
    /// See [`Self::menu`] and [`Self::context_menu`] for common use cases.
    pub fn from_response(response: &Response) -> Self {
        Self::new(
            Self::default_response_id(response),
            response.ctx.clone(),
            response,
            response.layer_id,
        )
    }

    /// Show a popup relative to some widget,
    /// toggling the open state based on the widget's click state.
    ///
    /// See [`Self::menu`] and [`Self::context_menu`] for common use cases.
    pub fn from_toggle_button_response(button_response: &Response) -> Self {
        Self::from_response(button_response)
            .open_memory(button_response.clicked().then_some(SetOpenCommand::Toggle))
    }

    /// Show a popup when the widget was clicked.
    /// Sets the layout to `Layout::top_down_justified(Align::Min)`.
    pub fn menu(button_response: &Response) -> Self {
        Self::from_toggle_button_response(button_response)
            .kind(PopupKind::Menu)
            .layout(Layout::top_down_justified(Align::Min))
            .style(menu_style)
            .gap(0.0)
    }

    /// Show a context menu when the widget was secondary clicked.
    /// Sets the layout to `Layout::top_down_justified(Align::Min)`.
    /// In contrast to [`Self::menu`], this will open at the pointer position.
    pub fn context_menu(response: &Response) -> Self {
        Self::menu(response)
            .open_memory(if response.secondary_clicked() {
                Some(SetOpenCommand::Bool(true))
            } else if response.clicked() {
                // Explicitly close the menu if the widget was clicked
                // Without this, the context menu would stay open if the user clicks the widget
                Some(SetOpenCommand::Bool(false))
            } else {
                None
            })
            .at_pointer_fixed()
    }

    /// Set the kind of the popup. Used for [`Area::kind`] and [`Area::order`].
    #[inline]
    pub fn kind(mut self, kind: PopupKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the [`UiStackInfo`] of the popup's [`Ui`].
    #[inline]
    pub fn info(mut self, info: UiStackInfo) -> Self {
        self.info = Some(info);
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
    pub fn open_memory(mut self, set_state: impl Into<Option<SetOpenCommand>>) -> Self {
        self.open_kind = OpenKind::Memory {
            set: set_state.into(),
        };
        self
    }

    /// Store the open state via a mutable bool.
    #[inline]
    pub fn open_bool(mut self, open: &'a mut bool) -> Self {
        self.open_kind = OpenKind::Bool(open);
        self
    }

    /// Set the close behavior of the popup.
    ///
    /// This will do nothing if [`Popup::open`] was called.
    #[inline]
    pub fn close_behavior(mut self, close_behavior: PopupCloseBehavior) -> Self {
        self.close_behavior = close_behavior;
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

    /// Set the frame of the popup.
    #[inline]
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
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

    /// Set the style for the popup contents.
    ///
    /// Default:
    /// - is [`menu_style`] for [`Self::menu`] and [`Self::context_menu`]
    /// - is [`None`] otherwise
    #[inline]
    pub fn style(mut self, style: impl Into<StyleModifier>) -> Self {
        self.style = style.into();
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
            OpenKind::Bool(open) => **open,
            OpenKind::Memory { .. } => Self::is_id_open(&self.ctx, self.id),
        }
    }

    /// Get the expected size of the popup.
    pub fn get_expected_size(&self) -> Option<Vec2> {
        AreaState::load(&self.ctx, self.id)?.size
    }

    /// Calculate the best alignment for the popup, based on the last size and screen rect.
    pub fn get_best_align(&self) -> RectAlign {
        let expected_popup_size = self
            .get_expected_size()
            .unwrap_or_else(|| vec2(self.width.unwrap_or(0.0), 0.0));

        let Some(anchor_rect) = self.anchor.rect(self.id, &self.ctx) else {
            return self.rect_align;
        };

        RectAlign::find_best_align(
            #[expect(clippy::iter_on_empty_collections)]
            #[expect(clippy::or_fun_call)]
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
            self.ctx.content_rect(),
            anchor_rect,
            self.gap,
            expected_popup_size,
        )
        .unwrap_or_default()
    }

    /// Show the popup.
    ///
    /// Returns `None` if the popup is not open or anchor is `PopupAnchor::Pointer` and there is
    /// no pointer.
    pub fn show<R>(self, content: impl FnOnce(&mut Ui) -> R) -> Option<InnerResponse<R>> {
        let id = self.id;
        // When the popup was just opened with a click we don't want to immediately close it based
        // on the `PopupCloseBehavior`, so we need to remember if the popup was already open on
        // last frame. A convenient way to check this is to see if we have a response for the `Area`
        // from last frame:
        let was_open_last_frame = self.ctx.read_response(id).is_some();

        let hover_pos = self.ctx.pointer_hover_pos();
        if let OpenKind::Memory { set } = self.open_kind {
            match set {
                Some(SetOpenCommand::Bool(open)) => {
                    if open {
                        match self.anchor {
                            PopupAnchor::PointerFixed => {
                                self.ctx.memory_mut(|mem| mem.open_popup_at(id, hover_pos));
                            }
                            _ => Popup::open_id(&self.ctx, id),
                        }
                    } else {
                        Self::close_id(&self.ctx, id);
                    }
                }
                Some(SetOpenCommand::Toggle) => {
                    Self::toggle_id(&self.ctx, id);
                }
                None => {
                    self.ctx.memory_mut(|mem| mem.keep_popup_open(id));
                }
            }
        }

        if !self.open_kind.is_open(self.id, &self.ctx) {
            return None;
        }

        let best_align = self.get_best_align();

        let Popup {
            id,
            ctx,
            anchor,
            open_kind,
            close_behavior,
            kind,
            info,
            layer_id,
            rect_align: _,
            alternative_aligns: _,
            gap,
            width,
            sense,
            layout,
            frame,
            style,
        } = self;

        if kind != PopupKind::Tooltip {
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
            .order(kind.order())
            .pivot(pivot)
            .fixed_pos(anchor)
            .sense(sense)
            .layout(layout)
            .info(info.unwrap_or_else(|| {
                UiStackInfo::new(kind.into()).with_tag_value(
                    MenuConfig::MENU_CONFIG_TAG,
                    MenuConfig::new()
                        .close_behavior(close_behavior)
                        .style(style.clone()),
                )
            }));

        if let Some(width) = width {
            area = area.default_width(width);
        }

        let mut response = area.show(&ctx, |ui| {
            style.apply(ui.style_mut());
            let frame = frame.unwrap_or_else(|| Frame::popup(ui.style()));
            frame.show(ui, content).inner
        });

        // If the popup was just opened with a click, we don't want to immediately close it again.
        let close_click = was_open_last_frame && ctx.input(|i| i.pointer.any_click());

        let closed_by_click = match close_behavior {
            PopupCloseBehavior::CloseOnClick => close_click,
            PopupCloseBehavior::CloseOnClickOutside => {
                close_click && response.response.clicked_elsewhere()
            }
            PopupCloseBehavior::IgnoreClicks => false,
        };

        // Mark the menu as shown, so the sub menu open state is not reset
        MenuState::mark_shown(&ctx, id);

        // If a submenu is open, the CloseBehavior is handled there
        let is_any_submenu_open = !MenuState::is_deepest_open_sub_menu(&response.response.ctx, id);

        let should_close = (!is_any_submenu_open && closed_by_click)
            || ctx.input(|i| i.key_pressed(Key::Escape))
            || response.response.should_close();

        if should_close {
            response.response.set_close();
        }

        match open_kind {
            OpenKind::Open | OpenKind::Closed => {}
            OpenKind::Bool(open) => {
                if should_close {
                    *open = false;
                }
            }
            OpenKind::Memory { .. } => {
                if should_close {
                    ctx.memory_mut(|mem| mem.close_popup(id));
                }
            }
        }

        Some(response)
    }
}

/// ## Static methods
impl Popup<'_> {
    /// The default ID when constructing a popup from the [`Response`] of e.g. a button.
    pub fn default_response_id(response: &Response) -> Id {
        response.id.with("popup")
    }

    /// Is the given popup open?
    ///
    /// This assumes the use of either:
    /// * [`Self::open_memory`]
    /// * [`Self::from_toggle_button_response`]
    /// * [`Self::menu`]
    /// * [`Self::context_menu`]
    ///
    /// The popup id should be the same as either you set with [`Self::id`] or the
    /// default one from [`Self::default_response_id`].
    pub fn is_id_open(ctx: &Context, popup_id: Id) -> bool {
        ctx.memory(|mem| mem.is_popup_open(popup_id))
    }

    /// Is any popup open?
    ///
    /// This assumes the egui memory is being used to track the open state of popups.
    pub fn is_any_open(ctx: &Context) -> bool {
        ctx.memory(|mem| mem.any_popup_open())
    }

    /// Open the given popup and close all others.
    ///
    /// If you are NOT using [`Popup::show`], you must
    /// also call [`crate::Memory::keep_popup_open`] as long as
    /// you're showing the popup.
    pub fn open_id(ctx: &Context, popup_id: Id) {
        ctx.memory_mut(|mem| mem.open_popup(popup_id));
    }

    /// Toggle the given popup between closed and open.
    ///
    /// Note: At most, only one popup can be open at a time.
    pub fn toggle_id(ctx: &Context, popup_id: Id) {
        ctx.memory_mut(|mem| mem.toggle_popup(popup_id));
    }

    /// Close all currently open popups.
    pub fn close_all(ctx: &Context) {
        ctx.memory_mut(|mem| mem.close_all_popups());
    }

    /// Close the given popup, if it is open.
    ///
    /// See also [`Self::close_all`] if you want to close any / all currently open popups.
    pub fn close_id(ctx: &Context, popup_id: Id) {
        ctx.memory_mut(|mem| mem.close_popup(popup_id));
    }

    /// Get the position for this popup, if it is open.
    pub fn position_of_id(ctx: &Context, popup_id: Id) -> Option<Pos2> {
        ctx.memory(|mem| mem.popup_position(popup_id))
    }
}
