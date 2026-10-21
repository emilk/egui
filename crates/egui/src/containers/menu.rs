//! Popup menus, context menus and menu bars.
//!
//! Show menus via
//! - [`Popup::menu`] and [`Popup::context_menu`]
//! - [`Ui::menu_button`], [`MenuButton`] and [`SubMenuButton`]
//! - [`MenuBar`]
//! - [`Response::context_menu`]
//!
//! See [`MenuBar`] for an example.

use crate::style::StyleModifier;
use crate::{
    Atom, AtomKind, AtomPaintArgs, Button, Color32, Context, FocusDirection, Frame, Id,
    InnerResponse, IntoAtoms, IntoSizedResult, Layout, PointerButton, Popup, PopupCloseBehavior,
    PopupKind, Response, SetOpenCommand, SizedAtomKind, Style, Ui, UiBuilder, UiKind, UiStack,
    UiStackInfo, Widget as _,
};
use alloc::sync::Arc;
use emath::{Align, Rect, RectAlign, Vec2, vec2};
use epaint::{Shape, Stroke};

/// Apply a menu style to the [`Style`].
///
/// Mainly removes the background stroke and the inactive background fill.
pub fn menu_style(style: &mut Style) {
    style.spacing.button_padding = vec2(2.0, 0.0);
    style.visuals.widgets.active.bg_stroke = Stroke::NONE;
    style.visuals.widgets.open.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
}

/// Find the root [`UiStack`] of the menu.
pub fn find_menu_root(ui: &Ui) -> &UiStack {
    ui.stack()
        .iter()
        .find(|stack| {
            stack.is_root_ui()
                || [Some(UiKind::Popup), Some(UiKind::Menu)].contains(&stack.kind())
                || stack.info.tags.contains(MenuConfig::MENU_CONFIG_TAG)
        })
        .expect("We should always find the root")
}

/// Find the [`UiStack`] of the [`MenuBar`] this [`Ui`] is directly part of, if any.
///
/// Returns `None` if the closest menu root is not a menu bar (e.g. if we are in a popup menu).
fn find_menu_bar_root(ui: &Ui) -> Option<&UiStack> {
    let root = find_menu_root(ui);
    root.info
        .tags
        .get_downcast::<MenuConfig>(MenuConfig::MENU_CONFIG_TAG)
        .is_some_and(|config| config.bar)
        .then_some(root)
}

/// Is this Ui part of a menu?
///
/// Returns `false` if this is a menu bar.
/// Should be used to determine if we should show a menu button or submenu button.
pub fn is_in_menu(ui: &Ui) -> bool {
    for stack in ui.stack().iter() {
        if let Some(config) = stack
            .info
            .tags
            .get_downcast::<MenuConfig>(MenuConfig::MENU_CONFIG_TAG)
        {
            return !config.bar;
        }
        if [Some(UiKind::Popup), Some(UiKind::Menu)].contains(&stack.kind()) {
            return true;
        }
    }
    false
}

/// Configuration and style for menus.
#[derive(Clone, Debug)]
pub struct MenuConfig {
    /// Is this a menu bar?
    bar: bool,

    /// If the user clicks, should we close the menu?
    pub close_behavior: PopupCloseBehavior,

    /// Override the menu style.
    ///
    /// Default is [`menu_style`].
    pub style: StyleModifier,

    /// The visual gap between the frame of a menu and the frames of the submenus it opens.
    ///
    /// `0.0` places submenus flush against their parent menu.
    ///
    /// Default is `2.0`.
    pub submenu_gap: f32,
}

impl Default for MenuConfig {
    fn default() -> Self {
        Self {
            close_behavior: PopupCloseBehavior::default(),
            bar: false,
            style: menu_style.into(),
            submenu_gap: 2.0,
        }
    }
}

impl MenuConfig {
    /// The tag used to store the menu config in the [`UiStack`].
    pub const MENU_CONFIG_TAG: &'static str = "egui_menu_config";

    pub fn new() -> Self {
        Self::default()
    }

    /// If the user clicks, should we close the menu?
    #[inline]
    pub fn close_behavior(mut self, close_behavior: PopupCloseBehavior) -> Self {
        self.close_behavior = close_behavior;
        self
    }

    /// Override the menu style.
    ///
    /// Default is [`menu_style`].
    #[inline]
    pub fn style(mut self, style: impl Into<StyleModifier>) -> Self {
        self.style = style.into();
        self
    }

    /// The visual gap between the frame of a menu and the frames of the submenus it opens.
    ///
    /// `0.0` places submenus flush against their parent menu.
    ///
    /// Default is `2.0`.
    #[inline]
    pub fn submenu_gap(mut self, submenu_gap: f32) -> Self {
        self.submenu_gap = submenu_gap;
        self
    }

    fn from_stack(stack: &UiStack) -> Self {
        stack
            .info
            .tags
            .get_downcast(Self::MENU_CONFIG_TAG)
            .cloned()
            .unwrap_or_default()
    }

    /// Find the config for the current menu.
    ///
    /// Returns the default config if no config is found.
    pub fn find(ui: &Ui) -> Self {
        find_menu_root(ui)
            .info
            .tags
            .get_downcast(Self::MENU_CONFIG_TAG)
            .cloned()
            .unwrap_or_default()
    }
}

/// Holds the state of the menu.
#[derive(Clone)]
pub struct MenuState {
    /// The currently open sub menu in this menu.
    pub open_item: Option<Id>,
    last_visible_pass: u64,
    focus_first: bool,

    /// A stationary pointer should not close a submenu opened with the keyboard.
    ignore_hover: bool,

    /// The widget that arrow keys moved focus to in the last pass.
    arrow_focus: Option<Id>,
}

impl MenuState {
    pub const ID: &'static str = "menu_state";

    /// Find the root of the menu and get the state
    pub fn from_ui<R>(ui: &Ui, f: impl FnOnce(&mut Self, &UiStack) -> R) -> R {
        let stack = find_menu_root(ui);
        Self::from_id(ui.ctx(), stack.unique_id, |state| f(state, stack))
    }

    /// Get the state via the menus root [`Ui`] id
    pub fn from_id<R>(ctx: &Context, id: Id, f: impl FnOnce(&mut Self) -> R) -> R {
        let pass_nr = ctx.cumulative_pass_nr();
        ctx.data_mut(|data| {
            let state_id = id.with(Self::ID);
            let mut state = data.get_temp(state_id).unwrap_or(Self {
                open_item: None,
                last_visible_pass: pass_nr,
                focus_first: false,
                ignore_hover: false,
                arrow_focus: None,
            });
            // If the menu was closed for at least a frame, reset the open item
            if state.last_visible_pass + 1 < pass_nr {
                state.open_item = None;
                state.focus_first = false;
                state.ignore_hover = false;
                state.arrow_focus = None;
            }
            if let Some(item) = state.open_item
                && data
                    .get_temp(item.with(Self::ID))
                    .is_none_or(|item: Self| item.last_visible_pass + 1 < pass_nr)
            {
                // If the open item wasn't shown for at least a frame, reset the open item
                state.open_item = None;
            }
            let r = f(&mut state);
            data.insert_temp(state_id, state);
            r
        })
    }

    pub fn mark_shown(ctx: &Context, id: Id) {
        let pass_nr = ctx.cumulative_pass_nr();
        Self::from_id(ctx, id, |state| {
            state.last_visible_pass = pass_nr;
        });
    }

    /// Navigate within this menu after all of its entries have registered focus interest.
    pub(crate) fn handle_keyboard(ui: &Ui, anchor_widget: Option<Id>) {
        let menu_id = ui.layer_id().id;
        let (allows_interaction, direction) =
            ui.memory(|mem| (mem.allows_interaction(ui.layer_id()), mem.focus_direction()));
        let active = !ui.is_sizing_pass() && allows_interaction;
        let focus_first = Self::from_id(ui.ctx(), menu_id, |state| {
            state.arrow_focus = None;
            active && core::mem::take(&mut state.focus_first)
        });
        if !active || (!focus_first && !direction.is_cardinal()) {
            return;
        }
        let is_submenu = anchor_widget.is_some_and(|id| SubMenu::id_from_widget_id(id) == menu_id);
        let focused = ui.memory(|mem| mem.focused());
        let entries: Vec<Id> = ui.ctx().viewport(|viewport| {
            viewport
                .this_pass
                .widgets
                .focusable_widgets_in_layer(ui.layer_id())
                .collect()
        });
        let focus_first = focus_first && focused == anchor_widget;
        let index = entries.iter().position(|id| Some(*id) == focused);
        let entering = focused == anchor_widget && !is_submenu;
        let from_empty = focused.is_none() && Self::is_deepest_open_sub_menu(ui.ctx(), menu_id);
        if index.is_none() && !entering && !from_empty && !focus_first {
            return;
        }
        let next = if focus_first {
            entries.first().copied()
        } else {
            match direction {
                FocusDirection::Down => index
                    .and_then(|index| entries.get(index + 1))
                    .or_else(|| entries.first())
                    .copied(),
                FocusDirection::Up => index
                    .and_then(|index| index.checked_sub(1))
                    .and_then(|index| entries.get(index))
                    .or_else(|| entries.last())
                    .copied(),
                // SubMenu handles the key towards its submenu on its own button before we get here.
                direction if direction == submenu_close_direction(ui) => anchor_widget,
                _ => None,
            }
        };
        Self::from_id(ui.ctx(), menu_id, |state| state.arrow_focus = next);
        ui.memory_mut(|mem| {
            mem.move_focus(FocusDirection::None);
            if let Some(next) = next {
                mem.request_focus_next_frame(next);
            }
        });
        ui.request_repaint();
    }

    /// Is the menu with this id the deepest sub menu? (-> no child sub menu is open)
    ///
    /// Note: This only returns correct results if called after the menu contents were shown.
    pub fn is_deepest_open_sub_menu(ctx: &Context, id: Id) -> bool {
        let pass_nr = ctx.cumulative_pass_nr();
        let open_item = Self::from_id(ctx, id, |state| state.open_item);
        // If we have some open item, check if that was actually shown this frame
        open_item.is_none_or(|submenu_id| {
            Self::from_id(ctx, submenu_id, |state| state.last_visible_pass != pass_nr)
        })
    }
}

/// Horizontal menu bar where you can add [`MenuButton`]s.
///
/// The menu bar goes well in a [`crate::Panel::top`],
/// but can also be placed in a [`crate::Window`].
/// In the latter case you may want to wrap it in [`Frame`].
///
/// ### Example:
/// ```
/// # egui::__run_test_ui(|ui| {
/// egui::MenuBar::new().ui(ui, |ui| {
///     ui.menu_button("File", |ui| {
///         if ui.button("Quit").clicked() {
///             ui.send_viewport_cmd(egui::ViewportCommand::Close);
///         }
///     });
/// });
/// # });
/// ```
#[derive(Clone, Debug)]
pub struct MenuBar {
    config: MenuConfig,
    style: StyleModifier,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self {
            config: MenuConfig::default(),
            style: menu_style.into(),
        }
    }
}

impl MenuBar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the style for buttons in the menu bar.
    ///
    /// Doesn't affect the style of submenus, use [`MenuConfig::style`] for that.
    /// Default is [`menu_style`].
    #[inline]
    pub fn style(mut self, style: impl Into<StyleModifier>) -> Self {
        self.style = style.into();
        self
    }

    /// Set the config for submenus.
    ///
    /// Note: The config will only be passed when using [`MenuButton`], not via [`Popup::menu`].
    #[inline]
    pub fn config(mut self, config: MenuConfig) -> Self {
        self.config = config;
        self
    }

    /// Show the menu bar.
    #[inline]
    pub fn ui<R>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let Self { mut config, style } = self;
        config.bar = true;
        // TODO(lucasmerlin): It'd be nice if we had a ui.horizontal_builder or something
        // So we don't need the nested scope here
        ui.horizontal(|ui| {
            ui.scope_builder(
                UiBuilder::new()
                    .layout(Layout::left_to_right(Align::Center))
                    .ui_stack_info(
                        UiStackInfo::new(UiKind::Menu)
                            .with_tag_value(MenuConfig::MENU_CONFIG_TAG, config),
                    ),
                |ui| {
                    style.apply(ui.style_mut());

                    // Take full width and fixed height:
                    let height = ui.spacing().interact_size.y;
                    ui.set_min_size(vec2(ui.available_width(), height));

                    content(ui)
                },
            )
            .inner
        })
    }
}

/// A thin wrapper around a [`Button`] that shows a [`Popup::menu`] when clicked.
///
/// The only thing this does is search for the current menu config (if set via [`MenuBar`]).
/// If your menu button is not in a [`MenuBar`] it's fine to use [`Ui::button`] and [`Popup::menu`]
/// directly.
pub struct MenuButton<'a> {
    pub button: Button<'a>,
    pub config: Option<MenuConfig>,
}

impl<'a> MenuButton<'a> {
    /// Salt for the id (relative to the [`MenuBar`]) where we store the popup id of
    /// the currently open menu in that bar.
    const OPEN_MENU_ID_SALT: &'static str = "egui_menu_bar_open_menu";

    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        Self::from_button(Button::new(atoms.into_atoms()))
    }

    /// Set the config for the menu.
    #[inline]
    pub fn config(mut self, config: MenuConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Create a new menu button from a [`Button`].
    #[inline]
    pub fn from_button(button: Button<'a>) -> Self {
        Self {
            button,
            config: None,
        }
    }

    /// Show the menu button.
    pub fn ui<R>(
        self,
        ui: &mut Ui,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> (Response, Option<InnerResponse<R>>) {
        let response = self.button.ui(ui);
        let mut config = self.config.unwrap_or_else(|| MenuConfig::find(ui));
        config.bar = false;

        let mut menu = Popup::menu(&response);
        let popup_id = menu.get_id();

        // If we are part of a menu bar, we remember which of its menus is open, so that
        // hovering another menu button in the same bar can switch to that menu without
        // requiring a click. This is the typical behavior of menu bars at the top of a window.
        let bar_open_menu_id =
            find_menu_bar_root(ui).map(|bar| bar.unique_id.with(Self::OPEN_MENU_ID_SALT));
        if let Some(bar_open_menu_id) = bar_open_menu_id
            && response.hovered()
            && !response.clicked()
        {
            let open_menu_in_bar = ui.data(|d| d.get_temp::<Id>(bar_open_menu_id));
            if let Some(open_menu_in_bar) = open_menu_in_bar
                && open_menu_in_bar != popup_id
                && Popup::is_id_open(ui.ctx(), open_menu_in_bar)
            {
                menu = menu.open_memory(Some(SetOpenCommand::Bool(true)));
            }
        }

        let inner = menu
            .close_behavior(config.close_behavior)
            .style(config.style.clone())
            .info(
                UiStackInfo::new(UiKind::Menu).with_tag_value(MenuConfig::MENU_CONFIG_TAG, config),
            )
            .show(content);

        if let Some(bar_open_menu_id) = bar_open_menu_id
            && inner.is_some()
        {
            ui.data_mut(|d| d.insert_temp(bar_open_menu_id, popup_id));
        }

        (response, inner)
    }
}

/// A submenu button that shows a [`SubMenu`] if a [`Button`] is hovered.
pub struct SubMenuButton<'a> {
    pub button: Button<'a>,
    pub sub_menu: SubMenu,
}

impl<'a> SubMenuButton<'a> {
    /// The submenu arrow triangle shape, pointing right.
    pub fn arrow_shape(rect: Rect, color: impl Into<Color32>) -> Shape {
        Self::arrow_shape_towards_submenu(rect, color, false)
    }

    /// The submenu arrow triangle shape, pointing left when `right_to_left`.
    fn arrow_shape_towards_submenu(
        rect: Rect,
        color: impl Into<Color32>,
        right_to_left: bool,
    ) -> Shape {
        let rect = Rect::from_center_size(
            rect.center(),
            vec2(rect.width() * 0.55, rect.height() * 0.35),
        );
        let rotation = if right_to_left {
            core::f32::consts::TAU / 4.0
        } else {
            -core::f32::consts::TAU / 4.0
        };
        Shape::rotated_triangle(rect, rotation, color)
    }

    /// An [`Atom`] painting the [`Self::arrow_shape`].
    ///
    /// With `None` the arrow follows the buttons text color, `Some(color)` overrides it.
    pub fn arrow_atom(color: Option<Color32>) -> Atom<'static> {
        // A closure, so the size can be based on the `Ui`s spacing.
        AtomKind::closure(move |ui, _args| {
            let size = Vec2::splat(ui.spacing().icon_width);
            IntoSizedResult {
                intrinsic_size: size,
                sized: SizedAtomKind::Paint {
                    paint: Arc::new(move |ui: &Ui, args: AtomPaintArgs| {
                        let color = color.unwrap_or(args.fallback_text_color);
                        ui.painter().add(Self::arrow_shape_towards_submenu(
                            args.rect,
                            color,
                            ui.style().right_to_left,
                        ));
                    }),
                    size,
                },
            }
        })
        .into()
    }

    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        Self::from_button(Button::new(atoms.into_atoms()).right_text(Self::arrow_atom(None)))
    }

    /// Create a new submenu button from a [`Button`].
    ///
    /// Use [`Button::right_text`] and [`SubMenuButton::arrow_atom`] to add the default right
    /// arrow.
    pub fn from_button(button: Button<'a>) -> Self {
        Self {
            button,
            sub_menu: SubMenu::default(),
        }
    }

    /// Set the config for the submenu.
    ///
    /// The close behavior will not affect the current button, but the buttons in the submenu.
    #[inline]
    pub fn config(mut self, config: MenuConfig) -> Self {
        self.sub_menu.config = Some(config);
        self
    }

    /// Show the submenu button.
    pub fn ui<R>(
        self,
        ui: &mut Ui,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> (Response, Option<InnerResponse<R>>) {
        let my_id = ui.next_auto_id();
        let open = MenuState::from_ui(ui, |state, _| {
            state.open_item == Some(SubMenu::id_from_widget_id(my_id))
        });
        let inactive = ui.style().visuals.widgets.inactive;
        // TODO(lucasmerlin) add `open` function to `Button`
        if open {
            ui.style_mut().visuals.widgets.inactive = ui.style().visuals.widgets.open;
        }
        // Mirrored in right-to-left, so the arrow sits on the side the submenu opens on.
        let response = self.button.direction(ui.style().chrome_direction()).ui(ui);
        ui.style_mut().visuals.widgets.inactive = inactive;

        let popup_response = self.sub_menu.show(ui, &response, content);

        (response, popup_response)
    }
}

/// The arrow key that moves focus from a submenu back to its button.
///
/// The opposite key opens the submenu.
fn submenu_close_direction(ui: &Ui) -> FocusDirection {
    if ui.style().right_to_left {
        FocusDirection::Right
    } else {
        FocusDirection::Left
    }
}

/// Show a submenu in a menu.
///
/// Useful if you want to make custom menu buttons.
/// Usually, just use [`MenuButton`] or [`SubMenuButton`] instead.
#[derive(Clone, Debug, Default)]
pub struct SubMenu {
    config: Option<MenuConfig>,
}

impl SubMenu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the config for the submenu.
    ///
    /// The close behavior will not affect the current button, but the buttons in the submenu.
    #[inline]
    pub fn config(mut self, config: MenuConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Get the id for the submenu from the widget/response id.
    pub fn id_from_widget_id(widget_id: Id) -> Id {
        widget_id.with("submenu")
    }

    /// Show the submenu.
    ///
    /// This does some heuristics to check if the `button_response` was the last thing in the
    /// menu that was hovered/clicked, and if so, shows the submenu.
    pub fn show<R>(
        self,
        ui: &Ui,
        button_response: &Response,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        // This frame is only used to measure the offset we should set for the popup, so that
        // contents align
        let measurement_frame = Frame::menu(ui.style());

        let id = Self::id_from_widget_id(button_response.id);

        // Get the state from the parent menu
        let pass_nr = ui.ctx().cumulative_pass_nr();
        let (open_item, menu_id, parent_config, parent_arrow_focus) =
            MenuState::from_ui(ui, |state, stack| {
                state.last_visible_pass = pass_nr;
                (
                    state.open_item,
                    stack.unique_id,
                    MenuConfig::from_stack(stack),
                    state.arrow_focus,
                )
            });

        let mut menu_config = self.config.unwrap_or_else(|| parent_config.clone());
        menu_config.bar = false;

        #[expect(clippy::unwrap_used)] // Since we are a child of that ui, this should always exist
        let menu_root_response = ui.ctx().read_response(menu_id).unwrap();

        let hover_pos = ui.ctx().pointer_hover_pos();

        // We don't care if the user is hovering over the border
        let menu_rect = menu_root_response.rect - measurement_frame.total_margin();
        let is_hovering_menu = hover_pos.is_some_and(|pos| {
            ui.ctx().layer_id_at(pos) == Some(menu_root_response.layer_id)
                && menu_rect.contains(pos)
        });

        let is_any_open = open_item.is_some();
        let mut is_open = open_item == Some(id);
        let was_open = is_open;
        let mut set_open = None;

        // We expand the button rect so there is no empty space where no menu is shown
        // TODO(lucasmerlin): Instead, maybe make item_spacing.y 0.0?
        let button_rect = button_response
            .rect
            .expand2(ui.style().spacing.item_spacing / 2.0);

        // In theory some other widget could cover the button and this check would still pass
        // But since we check if no other menu is open, nothing should be able to cover the button
        let is_hovered = hover_pos.is_some_and(|pos| button_rect.contains(pos));

        // `clicked` includes keyboard and accessibility click actions.
        // Enter/Space on the focused button enters the submenu, an accessibility click toggles it,
        // and pointer clicks keep it open (for touch and pointer interactions).
        let clicked = button_response.clicked();
        let clicked_by_pointer = button_response.clicked_by(PointerButton::Primary);
        let clicked_by_keyboard_or_access = clicked && !clicked_by_pointer;
        let focused = button_response.enabled() && button_response.has_focus();
        let open_direction = if ui.style().right_to_left {
            FocusDirection::Left
        } else {
            FocusDirection::Right
        };
        let arrow_open = focused && ui.memory(|mem| mem.focus_direction() == open_direction);
        if arrow_open {
            ui.memory_mut(|mem| mem.move_focus(FocusDirection::None));
        }
        let enter = arrow_open || (focused && clicked_by_keyboard_or_access);
        // Only arrow keys open on focus, so Tab can still move past the submenu.
        let focus_open = focused && parent_arrow_focus == Some(button_response.id);
        let keyboard_open = enter || focus_open || (!was_open && clicked_by_keyboard_or_access);
        let pointer_changed =
            ui.input(|i| i.pointer.delta() != Vec2::ZERO || i.pointer.any_click());
        let (ignore_hover, left_submenu) = MenuState::from_id(ui.ctx(), id, |state| {
            state.focus_first |= enter;
            if keyboard_open {
                // Keep the entry request through the popup's initial sizing pass.
                state.last_visible_pass = pass_nr;
                state.ignore_hover = true;
            } else if !was_open || pointer_changed {
                state.ignore_hover = false;
            }
            // Left (Right in right-to-left) inside the submenu moves focus back to our button.
            let left_submenu = state.arrow_focus == Some(button_response.id);
            if left_submenu {
                state.arrow_focus = None;
            }
            (state.ignore_hover, left_submenu)
        });

        let focus_moved_away =
            left_submenu || parent_arrow_focus.is_some_and(|focus| focus != button_response.id);
        if is_open && (focus_moved_away || (clicked_by_keyboard_or_access && !enter)) {
            set_open = Some(false);
            is_open = false;
        }

        // The clicked handler is there for accessibility (keyboard navigation)
        let should_open = button_response.enabled()
            && (enter || focus_open || (!was_open && clicked) || (is_hovered && !is_any_open));
        if should_open {
            set_open = Some(true);
            is_open = true;
            // Ensure that all other sub menus are closed when we open the menu
            MenuState::from_id(ui.ctx(), menu_id, |state| {
                state.open_item = None;
            });
        }

        // Half the margin brings the two menu frames edge-to-edge, then add the visual gap:
        let gap = measurement_frame.total_margin().sum().x * 0.5 + menu_config.submenu_gap;

        let mut response = button_response.clone();
        // Expand the button rect so that the button and the first item in the submenu are aligned
        let expand = Vec2::new(0.0, measurement_frame.total_margin().sum().y * 0.5);
        response.interact_rect = response.interact_rect.expand2(expand);

        let align = if ui.style().right_to_left {
            RectAlign::LEFT_START
        } else {
            RectAlign::RIGHT_START
        };

        let popup_response = Popup::from_response(&response)
            .id(id)
            .kind(PopupKind::Menu)
            .open(is_open)
            .align(align)
            .layout(Layout::top_down_justified(Align::Min))
            .gap(gap)
            .style(menu_config.style.clone())
            // The close behavior is handled by the menu (see below)
            .close_behavior(PopupCloseBehavior::IgnoreClicks)
            .info(
                UiStackInfo::new(UiKind::Menu)
                    .with_tag_value(MenuConfig::MENU_CONFIG_TAG, menu_config.clone()),
            )
            .show(|ui| {
                // Ensure our layer stays on top when the button is clicked
                if button_response.clicked() || button_response.is_pointer_button_down_on() {
                    ui.ctx().move_to_top(ui.layer_id());
                }
                content(ui)
            });

        if let Some(popup_response) = &popup_response {
            // If no child sub menu is open means we must be the deepest child sub menu.
            let is_deepest_submenu = MenuState::is_deepest_open_sub_menu(ui.ctx(), id);

            // If the user clicks and the cursor is not hovering over our menu rect, it's
            // safe to assume they clicked outside the menu, so we close everything.
            // If they were to hover some other parent submenu we wouldn't be open.
            // Only edge case is the user hovering this submenu's button, so we also check
            // if we clicked outside the parent menu (which we luckily have access to here).
            let clicked_outside = is_deepest_submenu
                && popup_response.response.clicked_elsewhere()
                && menu_root_response.clicked_elsewhere();

            // We never automatically close when a submenu button is clicked, (so menus work
            // on touch devices)
            // Luckily we will always be the deepest submenu when a submenu button is clicked,
            // so the following check is enough.
            let submenu_button_clicked = button_response.clicked();

            let clicked_inside = is_deepest_submenu
                && !submenu_button_clicked
                && response.ctx.input(|i| i.pointer.any_click())
                && hover_pos.is_some_and(|pos| popup_response.response.interact_rect.contains(pos));

            let click_close = match menu_config.close_behavior {
                PopupCloseBehavior::CloseOnClick => clicked_outside || clicked_inside,
                PopupCloseBehavior::CloseOnClickOutside => clicked_outside,
                PopupCloseBehavior::IgnoreClicks => false,
            };

            if click_close {
                set_open = Some(false);
                ui.close();
            }

            let is_moving_towards_rect = ui.input(|i| {
                i.pointer
                    .is_moving_towards_rect(&popup_response.response.rect)
            });
            if is_moving_towards_rect {
                // We need to repaint while this is true, so we can detect when
                // the pointer is no longer moving towards the rect
                ui.request_repaint();
            }
            let hovering_other_menu_entry = is_open
                && !ignore_hover
                && !is_hovered
                && !popup_response.response.contains_pointer()
                && !is_moving_towards_rect
                && is_hovering_menu;

            let close_called = popup_response.response.should_close();

            // Close the parent ui to e.g. close the popup from where the submenu was opened
            if close_called {
                ui.close();
            }

            if hovering_other_menu_entry {
                set_open = Some(false);
            }

            if ui.will_parent_close() {
                ui.data_mut(|data| data.remove_by_type::<MenuState>());
            }
        }

        if let Some(set_open) = set_open {
            MenuState::from_id(ui.ctx(), menu_id, |state| {
                state.open_item = set_open.then_some(id);
            });
        }

        popup_response
    }
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;

    use crate::{
        Context, Event, Id, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, UiBuilder,
        epaint::{ClippedShape, Shape},
        pos2, vec2,
    };

    /// A menu in the middle of the screen with one submenu.
    struct MenuTest {
        ctx: Context,
        menu_button: Cell<Rect>,
        submenu_button: Cell<(Id, Rect)>,
        submenu: Cell<Option<Rect>>,
    }

    impl MenuTest {
        fn new(right_to_left: bool) -> Self {
            let ctx = Context::default();
            ctx.all_styles_mut(|style| style.right_to_left = right_to_left);
            let test = Self {
                ctx,
                menu_button: Cell::new(Rect::NOTHING),
                submenu_button: Cell::new((Id::NULL, Rect::NOTHING)),
                submenu: Cell::new(None),
            };
            test.run(vec![]);
            let pos = test.menu_button.get().center();
            test.run(vec![Event::PointerMoved(pos), click(pos, true)]);
            test.run(vec![click(pos, false)]);
            test.run(vec![]);
            test
        }

        fn run(&self, events: Vec<Event>) -> Vec<ClippedShape> {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
                events,
                ..Default::default()
            };
            self.submenu.set(None);
            let mut output = self.ctx.run_ui(input, |ui| {
                let max_rect = Rect::from_min_size(pos2(350.0, 100.0), vec2(100.0, 100.0));
                ui.scope_builder(UiBuilder::new().max_rect(max_rect), |ui| {
                    let response = ui.menu_button("Menu", |ui| {
                        let response = ui.menu_button("Submenu", |ui| {
                            let _ = ui.button("Item");
                            self.submenu.set(Some(ui.min_rect()));
                        });
                        self.submenu_button
                            .set((response.response.id, response.response.rect));
                    });
                    self.menu_button.set(response.response.rect);
                });
            });
            output.textures_delta.clear();
            output.shapes
        }

        /// Which way the submenu arrow points (`1.0` right, `-1.0` left),
        /// and whether it is in the left half of the submenu button.
        fn submenu_arrow(&self) -> (f32, bool) {
            let button = self.submenu_button.get().1;
            let triangles: Vec<_> = self
                .run(vec![])
                .into_iter()
                .filter_map(|clipped| match clipped.shape {
                    Shape::Path(path)
                        if path.points.len() == 3
                            && path.points.iter().all(|p| button.contains(*p)) =>
                    {
                        Some(path.points)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(triangles.len(), 1, "Expected one arrow: {triangles:?}");
            let points = &triangles[0];
            let mean_x = points.iter().map(|p| p.x).sum::<f32>() / 3.0;
            let tip = points
                .iter()
                .max_by(|a, b| (a.x - mean_x).abs().total_cmp(&(b.x - mean_x).abs()))
                .expect("three points");
            ((tip.x - mean_x).signum(), mean_x < button.center().x)
        }

        fn press(&self, key: Key) {
            let event = |pressed| Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: Modifiers::NONE,
            };
            self.run(vec![event(true), event(false)]);
            for _ in 0..3 {
                self.run(vec![]);
            }
        }

        fn submenu_button_is_focused(&self) -> bool {
            self.ctx.memory(|mem| mem.focused()) == Some(self.submenu_button.get().0)
        }

        /// Hovers the submenu button, and returns its rect and the submenu rect.
        fn open_submenu_by_hover(&self) -> (Rect, Rect) {
            self.run(vec![Event::PointerMoved(
                self.submenu_button.get().1.center(),
            )]);
            for _ in 0..3 {
                self.run(vec![]);
            }
            let submenu = self.submenu.get().expect("The submenu didn't open");
            (self.submenu_button.get().1, submenu)
        }

        /// Opens the submenu with `open` from its focused button, then goes back with `back`.
        fn assert_submenu_keys(&self, open: Key, back: Key) {
            self.press(Key::ArrowDown);
            assert!(self.submenu_button_is_focused());
            self.press(open);
            assert!(
                self.submenu.get().is_some(),
                "{open:?} didn't open the submenu"
            );
            assert!(!self.submenu_button_is_focused());

            self.press(back);
            assert!(self.submenu_button_is_focused(), "{back:?} didn't go back");
            assert!(
                self.submenu.get().is_none(),
                "{back:?} didn't close the submenu"
            );
        }
    }

    fn click(pos: Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }

    #[test]
    fn submenu_opens_to_the_right() {
        let (button, submenu) = MenuTest::new(false).open_submenu_by_hover();
        assert!(submenu.left() > button.right(), "{button:?} {submenu:?}");
    }

    #[test]
    fn submenu_opens_to_the_left_right_to_left() {
        let (button, submenu) = MenuTest::new(true).open_submenu_by_hover();
        assert!(submenu.right() < button.left(), "{button:?} {submenu:?}");
    }

    #[test]
    fn submenu_arrow_at_the_right_pointing_right() {
        assert_eq!(MenuTest::new(false).submenu_arrow(), (1.0, false));
    }

    #[test]
    fn submenu_arrow_at_the_left_pointing_left_right_to_left() {
        assert_eq!(MenuTest::new(true).submenu_arrow(), (-1.0, true));
    }

    #[test]
    fn submenu_keys() {
        MenuTest::new(false).assert_submenu_keys(Key::ArrowRight, Key::ArrowLeft);
    }

    #[test]
    fn submenu_keys_right_to_left() {
        MenuTest::new(true).assert_submenu_keys(Key::ArrowLeft, Key::ArrowRight);
    }
}
