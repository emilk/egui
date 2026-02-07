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
    Button, Color32, Context, Frame, Id, InnerResponse, IntoAtoms, Layout, Popup,
    PopupCloseBehavior, Response, Style, Ui, UiBuilder, UiKind, UiStack, UiStackInfo, Widget as _,
};
use emath::{Align, RectAlign, Vec2, vec2};
use epaint::Stroke;

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
}

impl Default for MenuConfig {
    fn default() -> Self {
        Self {
            close_behavior: PopupCloseBehavior::default(),
            bar: false,
            style: menu_style.into(),
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
}

impl MenuState {
    pub const ID: &'static str = "menu_state";

    /// Find the root of the menu and get the state
    pub fn from_ui<R>(ui: &Ui, f: impl FnOnce(&mut Self, &UiStack) -> R) -> R {
        let stack = find_menu_root(ui);
        Self::from_id(ui.ctx(), stack.id, |state| f(state, stack))
    }

    /// Get the state via the menus root [`Ui`] id
    pub fn from_id<R>(ctx: &Context, id: Id, f: impl FnOnce(&mut Self) -> R) -> R {
        let pass_nr = ctx.cumulative_pass_nr();
        ctx.data_mut(|data| {
            let state_id = id.with(Self::ID);
            let mut state = data.get_temp(state_id).unwrap_or(Self {
                open_item: None,
                last_visible_pass: pass_nr,
            });
            // If the menu was closed for at least a frame, reset the open item
            if state.last_visible_pass + 1 < pass_nr {
                state.open_item = None;
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
/// The menu bar goes well in a [`crate::TopBottomPanel::top`],
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

#[deprecated = "Renamed to `egui::MenuBar`"]
pub type Bar = MenuBar;

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
        let inner = Popup::menu(&response)
            .close_behavior(config.close_behavior)
            .style(config.style.clone())
            .info(
                UiStackInfo::new(UiKind::Menu).with_tag_value(MenuConfig::MENU_CONFIG_TAG, config),
            )
            .show(content);
        (response, inner)
    }
}

/// A submenu button that shows a [`SubMenu`] if a [`Button`] is hovered.
pub struct SubMenuButton<'a> {
    pub button: Button<'a>,
    pub sub_menu: SubMenu,
}

impl<'a> SubMenuButton<'a> {
    /// The default right arrow symbol: `"⏵"`
    pub const RIGHT_ARROW: &'static str = "⏵";

    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        Self::from_button(Button::new(atoms.into_atoms()).right_text("⏵"))
    }

    /// Create a new submenu button from a [`Button`].
    ///
    /// Use [`Button::right_text`] and [`SubMenuButton::RIGHT_ARROW`] to add the default right
    /// arrow symbol.
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
        let response = self.button.ui(ui);
        ui.style_mut().visuals.widgets.inactive = inactive;

        let popup_response = self.sub_menu.show(ui, &response, content);

        (response, popup_response)
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
        let frame = Frame::menu(ui.style());

        let id = Self::id_from_widget_id(button_response.id);

        // Get the state from the parent menu
        let (open_item, menu_id, parent_config) = MenuState::from_ui(ui, |state, stack| {
            (state.open_item, stack.id, MenuConfig::from_stack(stack))
        });

        let mut menu_config = self.config.unwrap_or_else(|| parent_config.clone());
        menu_config.bar = false;

        #[expect(clippy::unwrap_used)] // Since we are a child of that ui, this should always exist
        let menu_root_response = ui.ctx().read_response(menu_id).unwrap();

        let hover_pos = ui.ctx().pointer_hover_pos();

        // We don't care if the user is hovering over the border
        let menu_rect = menu_root_response.rect - frame.total_margin();
        let is_hovering_menu = hover_pos.is_some_and(|pos| {
            ui.ctx().layer_id_at(pos) == Some(menu_root_response.layer_id)
                && menu_rect.contains(pos)
        });

        let is_any_open = open_item.is_some();
        let mut is_open = open_item == Some(id);
        let mut set_open = None;

        // We expand the button rect so there is no empty space where no menu is shown
        // TODO(lucasmerlin): Instead, maybe make item_spacing.y 0.0?
        let button_rect = button_response
            .rect
            .expand2(ui.style().spacing.item_spacing / 2.0);

        // In theory some other widget could cover the button and this check would still pass
        // But since we check if no other menu is open, nothing should be able to cover the button
        let is_hovered = hover_pos.is_some_and(|pos| button_rect.contains(pos));

        // The clicked handler is there for accessibility (keyboard navigation)
        let should_open =
            ui.is_enabled() && (button_response.clicked() || (is_hovered && !is_any_open));
        if should_open {
            set_open = Some(true);
            is_open = true;
            // Ensure that all other sub menus are closed when we open the menu
            MenuState::from_id(ui.ctx(), menu_id, |state| {
                state.open_item = None;
            });
        }

        let gap = frame.total_margin().sum().x / 2.0 + 2.0;

        let mut response = button_response.clone();
        // Expand the button rect so that the button and the first item in the submenu are aligned
        let expand = Vec2::new(0.0, frame.total_margin().sum().y / 2.0);
        response.interact_rect = response.interact_rect.expand2(expand);

        let popup_response = Popup::from_response(&response)
            .id(id)
            .open(is_open)
            .align(RectAlign::RIGHT_START)
            .layout(Layout::top_down_justified(Align::Min))
            .gap(gap)
            .style(menu_config.style.clone())
            .frame(frame)
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
