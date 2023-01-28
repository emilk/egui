//! Menu bar functionality (very basic so far).
//!
//! Usage:
//! ```
//! fn show_menu(ui: &mut egui::Ui) {
//!     use egui::{menu, Button};
//!
//!     menu::bar(ui, |ui| {
//!         ui.menu_button("File", |ui| {
//!             if ui.button("Open").clicked() {
//!                 // …
//!             }
//!         });
//!     });
//! }
//! ```

use super::{
    style::WidgetVisuals, Align, Context, Id, InnerResponse, PointerState, Pos2, Rect, Response,
    Sense, TextStyle, Ui, Vec2,
};
use crate::{widgets::*, *};
use epaint::mutex::RwLock;
use std::sync::Arc;

/// What is saved between frames.
#[derive(Clone, Default)]
pub(crate) struct BarState {
    open_menu: MenuRootManager,
}

impl BarState {
    fn load(ctx: &Context, bar_id: Id) -> Self {
        ctx.data_mut(|d| d.get_temp::<Self>(bar_id).unwrap_or_default())
    }

    fn store(self, ctx: &Context, bar_id: Id) {
        ctx.data_mut(|d| d.insert_temp(bar_id, self));
    }

    /// Show a menu at pointer if primary-clicked response.
    /// Should be called from [`Context`] on a [`Response`]
    pub fn bar_menu<R>(
        &mut self,
        response: &Response,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        MenuRoot::stationary_click_interaction(response, &mut self.open_menu, response.id);
        self.open_menu.show(response, add_contents)
    }
}

impl std::ops::Deref for BarState {
    type Target = MenuRootManager;

    fn deref(&self) -> &Self::Target {
        &self.open_menu
    }
}

impl std::ops::DerefMut for BarState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.open_menu
    }
}

fn set_menu_style(style: &mut Style) {
    style.spacing.button_padding = vec2(2.0, 0.0);
    style.visuals.widgets.active.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
}

/// The menu bar goes well in a [`TopBottomPanel::top`],
/// but can also be placed in a [`Window`].
/// In the latter case you may want to wrap it in [`Frame`].
pub fn bar<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.horizontal(|ui| {
        set_menu_style(ui.style_mut());

        // Take full width and fixed height:
        let height = ui.spacing().interact_size.y;
        ui.set_min_size(vec2(ui.available_width(), height));

        add_contents(ui)
    })
}

/// Construct a top level menu in a menu bar. This would be e.g. "File", "Edit" etc.
///
/// Responds to primary clicks.
///
/// Returns `None` if the menu is not open.
pub fn menu_button<R>(
    ui: &mut Ui,
    title: impl Into<WidgetText>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    stationary_menu_impl(ui, title, Box::new(add_contents))
}

/// Construct a top level menu with an image in a menu bar. This would be e.g. "File", "Edit" etc.
///
/// Responds to primary clicks.
///
/// Returns `None` if the menu is not open.
pub fn menu_image_button<R>(
    ui: &mut Ui,
    texture_id: TextureId,
    image_size: impl Into<Vec2>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    stationary_menu_image_impl(ui, texture_id, image_size, Box::new(add_contents))
}

/// Construct a nested sub menu in another menu.
///
/// Opens on hover.
///
/// Returns `None` if the menu is not open.
pub(crate) fn submenu_button<R>(
    ui: &mut Ui,
    parent_state: Arc<RwLock<MenuState>>,
    title: impl Into<WidgetText>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    SubMenu::new(parent_state, title).show(ui, add_contents)
}

/// wrapper for the contents of every menu.
pub(crate) fn menu_ui<'c, R>(
    ctx: &Context,
    menu_id: impl Into<Id>,
    menu_state_arc: &Arc<RwLock<MenuState>>,
    add_contents: impl FnOnce(&mut Ui) -> R + 'c,
) -> InnerResponse<R> {
    let pos = {
        let mut menu_state = menu_state_arc.write();
        menu_state.entry_count = 0;
        menu_state.rect.min
    };

    let area = Area::new(menu_id)
        .order(Order::Foreground)
        .constrain(true)
        .fixed_pos(pos)
        .interactable(true)
        .drag_bounds(ctx.screen_rect());
    let inner_response = area.show(ctx, |ui| {
        set_menu_style(ui.style_mut());

        Frame::menu(ui.style())
            .show(ui, |ui| {
                const DEFAULT_MENU_WIDTH: f32 = 150.0; // TODO(emilk): add to ui.spacing
                ui.set_max_width(DEFAULT_MENU_WIDTH);
                ui.set_menu_state(Some(menu_state_arc.clone()));
                ui.with_layout(Layout::top_down_justified(Align::LEFT), add_contents)
                    .inner
            })
            .inner
    });
    menu_state_arc.write().rect = inner_response.response.rect;
    inner_response
}

/// Build a top level menu with a button.
///
/// Responds to primary clicks.
fn stationary_menu_impl<'c, R>(
    ui: &mut Ui,
    title: impl Into<WidgetText>,
    add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
) -> InnerResponse<Option<R>> {
    let title = title.into();
    let bar_id = ui.id();
    let menu_id = bar_id.with(title.text());

    let mut bar_state = BarState::load(ui.ctx(), bar_id);

    let mut button = Button::new(title);

    if bar_state.open_menu.is_menu_open(menu_id) {
        button = button.fill(ui.visuals().widgets.open.weak_bg_fill);
        button = button.stroke(ui.visuals().widgets.open.bg_stroke);
    }

    let button_response = ui.add(button);
    let inner = bar_state.bar_menu(&button_response, add_contents);

    bar_state.store(ui.ctx(), bar_id);
    InnerResponse::new(inner.map(|r| r.inner), button_response)
}

/// Build a top level menu with an image button.
///
/// Responds to primary clicks.
fn stationary_menu_image_impl<'c, R>(
    ui: &mut Ui,
    texture_id: TextureId,
    image_size: impl Into<Vec2>,
    add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
) -> InnerResponse<Option<R>> {
    let bar_id = ui.id();

    let mut bar_state = BarState::load(ui.ctx(), bar_id);
    let button_response = ui.add(ImageButton::new(texture_id, image_size));
    let inner = bar_state.bar_menu(&button_response, add_contents);

    bar_state.store(ui.ctx(), bar_id);
    InnerResponse::new(inner.map(|r| r.inner), button_response)
}

/// Response to secondary clicks (right-clicks) by showing the given menu.
pub(crate) fn context_menu(
    response: &Response,
    add_contents: impl FnOnce(&mut Ui),
) -> Option<InnerResponse<()>> {
    let menu_id = Id::new("__egui::context_menu");
    let mut bar_state = BarState::load(&response.ctx, menu_id);

    MenuRoot::context_click_interaction(response, &mut bar_state, response.id);
    let inner_response = bar_state.show(response, add_contents);

    bar_state.store(&response.ctx, menu_id);
    inner_response
}

/// Stores the state for the context menu.
#[derive(Clone, Default)]
pub(crate) struct MenuRootManager {
    inner: Option<MenuRoot>,
}

impl MenuRootManager {
    /// Show a menu at pointer if right-clicked response.
    /// Should be called from [`Context`] on a [`Response`]
    pub fn show<R>(
        &mut self,
        response: &Response,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        if let Some(root) = self.inner.as_mut() {
            let (menu_response, inner_response) = root.show(response, add_contents);
            if MenuResponse::Close == menu_response {
                self.inner = None;
            }
            inner_response
        } else {
            None
        }
    }

    fn is_menu_open(&self, id: Id) -> bool {
        self.inner.as_ref().map(|m| m.id) == Some(id)
    }
}

impl std::ops::Deref for MenuRootManager {
    type Target = Option<MenuRoot>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for MenuRootManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Menu root associated with an Id from a Response
#[derive(Clone)]
pub(crate) struct MenuRoot {
    pub menu_state: Arc<RwLock<MenuState>>,
    pub id: Id,
}

impl MenuRoot {
    pub fn new(position: Pos2, id: Id) -> Self {
        Self {
            menu_state: Arc::new(RwLock::new(MenuState::new(position))),
            id,
        }
    }

    pub fn show<R>(
        &mut self,
        response: &Response,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> (MenuResponse, Option<InnerResponse<R>>) {
        if self.id == response.id {
            let inner_response =
                MenuState::show(&response.ctx, &self.menu_state, self.id, add_contents);
            let mut menu_state = self.menu_state.write();
            menu_state.rect = inner_response.response.rect;

            if menu_state.response.is_close() {
                return (MenuResponse::Close, Some(inner_response));
            }
        }
        (MenuResponse::Stay, None)
    }

    /// Interaction with a stationary menu, i.e. fixed in another Ui.
    ///
    /// Responds to primary clicks.
    fn stationary_interaction(
        response: &Response,
        root: &mut MenuRootManager,
        id: Id,
    ) -> MenuResponse {
        if (response.clicked() && root.is_menu_open(id))
            || response.ctx.input(|i| i.key_pressed(Key::Escape))
        {
            // menu open and button clicked or esc pressed
            return MenuResponse::Close;
        } else if (response.clicked() && !root.is_menu_open(id))
            || (response.hovered() && root.is_some())
        {
            // menu not open and button clicked
            // or button hovered while other menu is open
            let mut pos = response.rect.left_bottom();
            if let Some(root) = root.inner.as_mut() {
                let menu_rect = root.menu_state.read().rect;
                let screen_rect = response.ctx.input(|i| i.screen_rect);

                if pos.y + menu_rect.height() > screen_rect.max.y {
                    pos.y = screen_rect.max.y - menu_rect.height() - response.rect.height();
                }

                if pos.x + menu_rect.width() > screen_rect.max.x {
                    pos.x = screen_rect.max.x - menu_rect.width();
                }
            }

            return MenuResponse::Create(pos, id);
        } else if response
            .ctx
            .input(|i| i.pointer.any_pressed() && i.pointer.primary_down())
        {
            if let Some(pos) = response.ctx.input(|i| i.pointer.interact_pos()) {
                if let Some(root) = root.inner.as_mut() {
                    if root.id == id {
                        // pressed somewhere while this menu is open
                        let menu_state = root.menu_state.read();
                        let in_menu = menu_state.area_contains(pos);
                        if !in_menu {
                            return MenuResponse::Close;
                        }
                    }
                }
            }
        }
        MenuResponse::Stay
    }

    /// Interaction with a context menu (secondary clicks).
    fn context_interaction(
        response: &Response,
        root: &mut Option<MenuRoot>,
        id: Id,
    ) -> MenuResponse {
        let response = response.interact(Sense::click());
        response.ctx.input(|input| {
            let pointer = &input.pointer;
            if pointer.any_pressed() {
                if let Some(pos) = pointer.interact_pos() {
                    let mut destroy = false;
                    let mut in_old_menu = false;
                    if let Some(root) = root {
                        let menu_state = root.menu_state.read();
                        in_old_menu = menu_state.area_contains(pos);
                        destroy = root.id == response.id;
                    }
                    if !in_old_menu {
                        if response.hovered() && pointer.secondary_down() {
                            return MenuResponse::Create(pos, id);
                        } else if (response.hovered() && pointer.primary_down()) || destroy {
                            return MenuResponse::Close;
                        }
                    }
                }
            }
            MenuResponse::Stay
        })
    }

    fn handle_menu_response(root: &mut MenuRootManager, menu_response: MenuResponse) {
        match menu_response {
            MenuResponse::Create(pos, id) => {
                root.inner = Some(MenuRoot::new(pos, id));
            }
            MenuResponse::Close => root.inner = None,
            MenuResponse::Stay => {}
        }
    }

    /// Respond to secondary (right) clicks.
    pub fn context_click_interaction(response: &Response, root: &mut MenuRootManager, id: Id) {
        let menu_response = Self::context_interaction(response, root, id);
        Self::handle_menu_response(root, menu_response);
    }

    // Responds to primary clicks.
    pub fn stationary_click_interaction(response: &Response, root: &mut MenuRootManager, id: Id) {
        let menu_response = Self::stationary_interaction(response, root, id);
        Self::handle_menu_response(root, menu_response);
    }
}

#[derive(Copy, Clone, PartialEq)]
pub(crate) enum MenuResponse {
    Close,
    Stay,
    Create(Pos2, Id),
}

impl MenuResponse {
    pub fn is_close(&self) -> bool {
        *self == Self::Close
    }
}

pub struct SubMenuButton {
    text: WidgetText,
    icon: WidgetText,
    index: usize,
}

impl SubMenuButton {
    /// The `icon` can be an emoji (e.g. `⏵` right arrow), shown right of the label
    fn new(text: impl Into<WidgetText>, icon: impl Into<WidgetText>, index: usize) -> Self {
        Self {
            text: text.into(),
            icon: icon.into(),
            index,
        }
    }

    fn visuals<'a>(
        ui: &'a Ui,
        response: &'_ Response,
        menu_state: &'_ MenuState,
        sub_id: Id,
    ) -> &'a WidgetVisuals {
        if menu_state.is_open(sub_id) {
            &ui.style().visuals.widgets.open
        } else {
            ui.style().interact(response)
        }
    }

    pub fn icon(mut self, icon: impl Into<WidgetText>) -> Self {
        self.icon = icon.into();
        self
    }

    pub(crate) fn show(self, ui: &mut Ui, menu_state: &MenuState, sub_id: Id) -> Response {
        let SubMenuButton { text, icon, .. } = self;

        let text_style = TextStyle::Button;
        let sense = Sense::click();

        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding;
        let text_available_width = ui.available_width() - total_extra.x;
        let text_galley =
            text.into_galley(ui, Some(true), text_available_width, text_style.clone());

        let icon_available_width = text_available_width - text_galley.size().x;
        let icon_galley = icon.into_galley(ui, Some(true), icon_available_width, text_style);
        let text_and_icon_size = Vec2::new(
            text_galley.size().x + icon_galley.size().x,
            text_galley.size().y.max(icon_galley.size().y),
        );
        let mut desired_size = text_and_icon_size + 2.0 * button_padding;
        desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);

        let (rect, response) = ui.allocate_at_least(desired_size, sense);
        response.widget_info(|| {
            crate::WidgetInfo::labeled(crate::WidgetType::Button, text_galley.text())
        });

        if ui.is_rect_visible(rect) {
            let visuals = Self::visuals(ui, &response, menu_state, sub_id);
            let text_pos = Align2::LEFT_CENTER
                .align_size_within_rect(text_galley.size(), rect.shrink2(button_padding))
                .min;
            let icon_pos = Align2::RIGHT_CENTER
                .align_size_within_rect(icon_galley.size(), rect.shrink2(button_padding))
                .min;

            if ui.visuals().button_frame {
                ui.painter().rect_filled(
                    rect.expand(visuals.expansion),
                    visuals.rounding,
                    visuals.weak_bg_fill,
                );
            }

            let text_color = visuals.text_color();
            text_galley.paint_with_fallback_color(ui.painter(), text_pos, text_color);
            icon_galley.paint_with_fallback_color(ui.painter(), icon_pos, text_color);
        }
        response
    }
}

pub struct SubMenu {
    button: SubMenuButton,
    parent_state: Arc<RwLock<MenuState>>,
}

impl SubMenu {
    fn new(parent_state: Arc<RwLock<MenuState>>, text: impl Into<WidgetText>) -> Self {
        let index = parent_state.write().next_entry_index();
        Self {
            button: SubMenuButton::new(text, "⏵", index),
            parent_state,
        }
    }

    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<Option<R>> {
        let sub_id = ui.id().with(self.button.index);
        let button = self.button.show(ui, &self.parent_state.read(), sub_id);
        self.parent_state
            .write()
            .submenu_button_interaction(ui, sub_id, &button);
        let inner = self
            .parent_state
            .write()
            .show_submenu(ui.ctx(), sub_id, add_contents);
        InnerResponse::new(inner, button)
    }
}

pub(crate) struct MenuState {
    /// The opened sub-menu and its [`Id`]
    sub_menu: Option<(Id, Arc<RwLock<MenuState>>)>,

    /// Bounding box of this menu (without the sub-menu)
    pub rect: Rect,

    /// Used to check if any menu in the tree wants to close
    pub response: MenuResponse,

    /// Used to hash different [`Id`]s for sub-menus
    entry_count: usize,
}

impl MenuState {
    pub fn new(position: Pos2) -> Self {
        Self {
            rect: Rect::from_min_size(position, Vec2::ZERO),
            sub_menu: None,
            response: MenuResponse::Stay,
            entry_count: 0,
        }
    }

    /// Close menu hierarchy.
    pub fn close(&mut self) {
        self.response = MenuResponse::Close;
    }

    pub fn show<R>(
        ctx: &Context,
        menu_state: &Arc<RwLock<Self>>,
        id: Id,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        crate::menu::menu_ui(ctx, id, menu_state, add_contents)
    }

    fn show_submenu<R>(
        &mut self,
        ctx: &Context,
        id: Id,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        let (sub_response, response) = self.submenu(id).map(|sub| {
            let inner_response = Self::show(ctx, sub, id, add_contents);
            (sub.read().response, inner_response.inner)
        })?;
        self.cascade_close_response(sub_response);
        Some(response)
    }

    /// Check if position is in the menu hierarchy's area.
    pub fn area_contains(&self, pos: Pos2) -> bool {
        self.rect.contains(pos)
            || self
                .sub_menu
                .as_ref()
                .map_or(false, |(_, sub)| sub.read().area_contains(pos))
    }

    fn next_entry_index(&mut self) -> usize {
        self.entry_count += 1;
        self.entry_count - 1
    }

    /// Sense button interaction opening and closing submenu.
    fn submenu_button_interaction(&mut self, ui: &mut Ui, sub_id: Id, button: &Response) {
        let pointer = ui.input(|i| i.pointer.clone());
        let open = self.is_open(sub_id);
        if self.moving_towards_current_submenu(&pointer) {
            // ensure to repaint once even when pointer is not moving
            ui.ctx().request_repaint();
        } else if !open && button.hovered() {
            let pos = button.rect.right_top();
            self.open_submenu(sub_id, pos);
        } else if open && !button.hovered() && !self.hovering_current_submenu(&pointer) {
            self.close_submenu();
        }
    }

    /// Check if `dir` points from `pos` towards left side of `rect`.
    fn points_at_left_of_rect(pos: Pos2, dir: Vec2, rect: Rect) -> bool {
        let vel_a = dir.angle();
        let top_a = (rect.left_top() - pos).angle();
        let bottom_a = (rect.left_bottom() - pos).angle();
        bottom_a - vel_a >= 0.0 && top_a - vel_a <= 0.0
    }

    /// Check if pointer is moving towards current submenu.
    fn moving_towards_current_submenu(&self, pointer: &PointerState) -> bool {
        if pointer.is_still() {
            return false;
        }
        if let Some(sub_menu) = self.current_submenu() {
            if let Some(pos) = pointer.hover_pos() {
                return Self::points_at_left_of_rect(pos, pointer.velocity(), sub_menu.read().rect);
            }
        }
        false
    }

    /// Check if pointer is hovering current submenu.
    fn hovering_current_submenu(&self, pointer: &PointerState) -> bool {
        if let Some(sub_menu) = self.current_submenu() {
            if let Some(pos) = pointer.hover_pos() {
                return sub_menu.read().area_contains(pos);
            }
        }
        false
    }

    /// Cascade close response to menu root.
    fn cascade_close_response(&mut self, response: MenuResponse) {
        if response.is_close() {
            self.response = response;
        }
    }

    fn is_open(&self, id: Id) -> bool {
        self.sub_id() == Some(id)
    }

    fn sub_id(&self) -> Option<Id> {
        self.sub_menu.as_ref().map(|(id, _)| *id)
    }

    fn current_submenu(&self) -> Option<&Arc<RwLock<MenuState>>> {
        self.sub_menu.as_ref().map(|(_, sub)| sub)
    }

    fn submenu(&mut self, id: Id) -> Option<&Arc<RwLock<MenuState>>> {
        self.sub_menu
            .as_ref()
            .and_then(|(k, sub)| if id == *k { Some(sub) } else { None })
    }

    /// Open submenu at position, if not already open.
    fn open_submenu(&mut self, id: Id, pos: Pos2) {
        if !self.is_open(id) {
            self.sub_menu = Some((id, Arc::new(RwLock::new(MenuState::new(pos)))));
        }
    }

    fn close_submenu(&mut self) {
        self.sub_menu = None;
    }
}
