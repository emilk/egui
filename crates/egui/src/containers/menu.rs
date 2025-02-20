use crate::{
    Button, Color32, Context, Frame, Id, InnerResponse, Layout, Popup, PopupCloseBehavior,
    Response, Style, Ui, UiBuilder, UiKind, UiStack, UiStackInfo, Widget, WidgetText,
};
use emath::{vec2, Align, RectAlign, Vec2};
use epaint::Stroke;

/// Apply a menu style to the [`Style`].
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
        // It's fine to unwrap since we should always find the root
        .unwrap()
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

#[derive(Clone, Debug)]
pub struct MenuConfig {
    /// Is this a menu bar?
    bar: bool,

    /// If the user clicks, should we close the menu?
    pub close_behavior: PopupCloseBehavior,

    /// Override the menu style.
    ///
    /// Default is [`menu_style`].
    pub style: Option<fn(&mut Style)>,
}

impl Default for MenuConfig {
    fn default() -> Self {
        Self {
            close_behavior: PopupCloseBehavior::CloseOnClickOutside,
            bar: false,
            style: Some(menu_style),
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
    pub fn style(mut self, style: impl Into<Option<fn(&mut Style)>>) -> Self {
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

#[derive(Clone)]
pub struct MenuState {
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
            let state = data.get_temp_mut_or_insert_with(id.with(Self::ID), || Self {
                open_item: None,
                last_visible_pass: pass_nr,
            });
            // If the menu was closed for at least a frame, reset the open item
            if state.last_visible_pass < pass_nr - 1 {
                state.open_item = None;
            }
            state.last_visible_pass = pass_nr;
            f(state)
        })
    }

    /// Is the menu with this id the deepest sub menu? (-> no child sub menu is open)
    pub fn is_deepest_sub_menu(ctx: &Context, id: Id) -> bool {
        Self::from_id(ctx, id, |state| state.open_item.is_none())
    }
}

/// The menu bar goes well in a [`crate::TopBottomPanel::top`],
/// but can also be placed in a [`crate::Window`].
/// In the latter case you may want to wrap it in [`Frame`].
#[derive(Clone, Debug)]
pub struct Bar {
    config: MenuConfig,
    style: Option<fn(&mut Style)>,
}

impl Default for Bar {
    fn default() -> Self {
        Self {
            config: MenuConfig::default(),
            style: Some(menu_style),
        }
    }
}

impl Bar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the style for buttons in the menu bar.
    ///
    /// Doesn't affect the style of submenus, use [`MenuConfig::style`] for that.
    /// Default is [`menu_style`].
    #[inline]
    pub fn style(mut self, style: impl Into<Option<fn(&mut Style)>>) -> Self {
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
        ui.scope_builder(
            UiBuilder::new()
                .layout(Layout::left_to_right(Align::Center))
                .ui_stack_info(
                    UiStackInfo::new(UiKind::Menu)
                        .with_tag_value(MenuConfig::MENU_CONFIG_TAG, config),
                ),
            |ui| {
                if let Some(style) = style {
                    style(ui.style_mut());
                }

                // Take full width and fixed height:
                let height = ui.spacing().interact_size.y;
                ui.set_min_size(vec2(ui.available_width(), height));

                content(ui)
            },
        )
    }
}

/// A thin wrapper around a [`Button`] that shows a [`Popup::menu`] when clicked.
///
/// The only thing this does is search for the current menu config (if set via [`Bar`]).
/// If your menu button is not in a [`Bar`] it's fine to use [`Ui::button`] and [`Popup::menu`]
/// directly.
pub struct MenuButton<'a> {
    pub button: Button<'a>,
    pub config: Option<MenuConfig>,
}

impl<'a> MenuButton<'a> {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self::from_button(Button::new(text))
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
        let config = self.config.unwrap_or_else(|| MenuConfig::find(ui));
        let inner = Popup::menu(&response)
            .close_behavior(config.close_behavior)
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

    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self::from_button(Button::new(text).right_text("⏵"))
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
        if open {
            ui.style_mut().visuals.widgets.inactive = ui.style().visuals.widgets.open;
        }
        let response = self.button.ui(ui);
        ui.style_mut().visuals.widgets.inactive = inactive;

        let popup_response = self.sub_menu.show(ui, &response, content);

        (response, popup_response)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SubMenu {
    config: Option<MenuConfig>,
}

impl SubMenu {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the config for the submenu.
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
    pub fn show<R>(
        self,
        ui: &Ui,
        response: &Response,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let frame = Frame::menu(ui.style());

        let id = Self::id_from_widget_id(response.id);

        let (open_item, menu_id, parent_config) = MenuState::from_ui(ui, |state, stack| {
            (state.open_item, stack.id, MenuConfig::from_stack(stack))
        });

        let mut menu_config = self.config.unwrap_or_else(|| parent_config.clone());
        menu_config.bar = false;

        let menu_root_response = ui
            .ctx()
            .read_response(menu_id)
            // Since we are a child of that ui, this should always exist
            .unwrap();

        let hover_pos = ui.ctx().pointer_hover_pos();

        // We don't care if the users is hovering over the border
        let menu_rect = menu_root_response.rect - frame.total_margin();
        let is_hovering_menu = hover_pos.is_some_and(|pos| {
            ui.ctx().layer_id_at(pos) == Some(menu_root_response.layer_id)
                && menu_rect.contains(pos)
        });

        let is_any_open = open_item.is_some();
        let mut is_open = open_item == Some(id);
        let mut set_open = None;
        let button_rect = response.rect.expand2(ui.style().spacing.item_spacing / 2.0);
        let is_hovered = hover_pos.is_some_and(|pos| button_rect.contains(pos));

        // The clicked handler is there for accessibility (keyboard navigation)
        if (!is_any_open && is_hovered) || response.clicked() {
            set_open = Some(true);
            is_open = true;
            // Ensure that all other sub menus are closed when we open the menu
            MenuState::from_id(ui.ctx(), id, |state| {
                state.open_item = None;
            });
        }

        let gap = frame.total_margin().sum().x / 2.0 + 2.0;

        let mut response = response.clone();
        // Expand the button rect so that the button and the first item in the submenu are aligned
        response.rect = response
            .rect
            .expand2(Vec2::new(0.0, frame.total_margin().sum().y / 2.0));

        let popup_response = Popup::from_response(&response)
            .id(id)
            .open(is_open)
            .align(RectAlign::RIGHT_START)
            .layout(Layout::top_down_justified(Align::Min))
            .gap(gap)
            .style(menu_config.style)
            .frame(frame)
            .close_behavior(match menu_config.close_behavior {
                // We ignore ClickOutside because it is handled by the menu (see below)
                PopupCloseBehavior::CloseOnClickOutside => PopupCloseBehavior::IgnoreClicks,
                behavior => behavior,
            })
            .info(
                UiStackInfo::new(UiKind::Menu)
                    .with_tag_value(MenuConfig::MENU_CONFIG_TAG, menu_config),
            )
            .show(content);

        if let Some(popup_response) = &popup_response {
            // The other close behaviors are handled by the popup
            if parent_config.close_behavior == PopupCloseBehavior::CloseOnClickOutside {
                let is_deepest_submenu = MenuState::is_deepest_sub_menu(ui.ctx(), id);
                // If no child sub menu is open means we must be the deepest child sub menu.
                // If the user clicks and the cursor is not hovering over our menu rect, it's
                // safe to assume they clicked outside the menu, so we close everything.
                // If they were to hover some other parent submenu we wouldn't be open.
                // Only edge case is the user hovering this submenu's button, so we also check
                // if we clicked outside the parent menu (which we luckily have access to here).
                let clicked_outside = is_deepest_submenu
                    && popup_response.response.clicked_elsewhere()
                    && menu_root_response.clicked_elsewhere();
                if clicked_outside {
                    ui.close();
                }
            }

            let is_moving_towards_rect = ui.input(|i| {
                i.pointer
                    .is_moving_towards_rect(&popup_response.response.rect)
            });
            if is_moving_towards_rect {
                // We need to repaint while this is true, so we can detect when
                // the pointer is no longer moving towards the rect
                ui.ctx().request_repaint();
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

            if hovering_other_menu_entry || ui.should_close() {
                set_open = Some(false);
            }

            if ui.will_close() {
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
