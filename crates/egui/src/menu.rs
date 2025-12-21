#![expect(deprecated)]
//! Deprecated menu API - Use [`crate::containers::menu`] instead.
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
    Align, Context, Id, InnerResponse, PointerState, Pos2, Rect, Response, Sense, TextStyle, Ui,
    Vec2, style::WidgetVisuals,
};
use crate::{
    Align2, Area, Color32, Frame, Key, LayerId, Layout, NumExt as _, Order, Stroke, Style,
    TextWrapMode, UiKind, WidgetText, epaint, vec2,
    widgets::{Button, ImageButton},
};
use epaint::mutex::RwLock;
use std::sync::Arc;

/// What is saved between frames.
#[derive(Clone, Default)]
pub struct BarState {
    open_menu: MenuRootManager,
}

impl BarState {
    pub fn load(ctx: &Context, bar_id: Id) -> Self {
        ctx.data_mut(|d| d.get_temp::<Self>(bar_id).unwrap_or_default())
    }

    pub fn store(self, ctx: &Context, bar_id: Id) {
        ctx.data_mut(|d| d.insert_temp(bar_id, self));
    }

    /// Show a menu at pointer if primary-clicked response.
    ///
    /// Should be called from [`Context`] on a [`Response`]
    pub fn bar_menu<R>(
        &mut self,
        button: &Response,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        MenuRoot::stationary_click_interaction(button, &mut self.open_menu);
        self.open_menu.show(button, add_contents)
    }

    pub(crate) fn has_root(&self) -> bool {
        self.open_menu.inner.is_some()
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
    if style.compact_menu_style {
        style.spacing.button_padding = vec2(2.0, 0.0);
        style.visuals.widgets.active.bg_stroke = Stroke::NONE;
        style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
        style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    }
}

/// The menu bar goes well in a [`crate::Panel::top`],
/// but can also be placed in a [`crate::Window`].
/// In the latter case you may want to wrap it in [`Frame`].
#[deprecated = "Use `egui::MenuBar::new().ui(` instead"]
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

/// Construct a top level menu with a custom button in a menu bar.
///
/// Responds to primary clicks.
///
/// Returns `None` if the menu is not open.
pub fn menu_custom_button<R>(
    ui: &mut Ui,
    button: Button<'_>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    stationary_menu_button_impl(ui, button, Box::new(add_contents))
}

/// Construct a top level menu with an image in a menu bar. This would be e.g. "File", "Edit" etc.
///
/// Responds to primary clicks.
///
/// Returns `None` if the menu is not open.
#[deprecated = "Use `menu_custom_button` instead"]
pub fn menu_image_button<R>(
    ui: &mut Ui,
    image_button: ImageButton<'_>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    stationary_menu_button_impl(
        ui,
        Button::image(image_button.image),
        Box::new(add_contents),
    )
}

/// Construct a nested sub menu in another menu.
///
/// Opens on hover.
///
/// Returns `None` if the menu is not open.
pub fn submenu_button<R>(
    ui: &mut Ui,
    parent_state: Arc<RwLock<MenuState>>,
    title: impl Into<WidgetText>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    SubMenu::new(parent_state, title).show(ui, add_contents)
}

/// wrapper for the contents of every menu.
fn menu_popup<'c, R>(
    ctx: &Context,
    parent_layer: LayerId,
    menu_state_arc: &Arc<RwLock<MenuState>>,
    menu_id: Id,
    add_contents: impl FnOnce(&mut Ui) -> R + 'c,
) -> InnerResponse<R> {
    let pos = {
        let mut menu_state = menu_state_arc.write();
        menu_state.entry_count = 0;
        menu_state.rect.min
    };

    let area_id = menu_id.with("__menu");

    ctx.pass_state_mut(|fs| {
        fs.layers
            .entry(parent_layer)
            .or_default()
            .open_popups
            .insert(area_id)
    });

    let area = Area::new(area_id)
        .kind(UiKind::Menu)
        .order(Order::Foreground)
        .fixed_pos(pos)
        .default_width(ctx.global_style().spacing.menu_width)
        .sense(Sense::hover());

    let mut sizing_pass = false;

    let area_response = area.show(ctx, |ui| {
        sizing_pass = ui.is_sizing_pass();

        set_menu_style(ui.style_mut());

        Frame::menu(ui.style())
            .show(ui, |ui| {
                ui.set_menu_state(Some(Arc::clone(menu_state_arc)));
                ui.with_layout(Layout::top_down_justified(Align::LEFT), add_contents)
                    .inner
            })
            .inner
    });

    let area_rect = area_response.response.rect;

    menu_state_arc.write().rect = if sizing_pass {
        // During the sizing pass we didn't know the size yet,
        // so we might have just constrained the position unnecessarily.
        // Therefore keep the original=desired position until the next frame.
        Rect::from_min_size(pos, area_rect.size())
    } else {
        // We knew the size, and this is where it ended up (potentially constrained to screen).
        // Remember it for the future:
        area_rect
    };

    area_response
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
fn stationary_menu_button_impl<'c, R>(
    ui: &mut Ui,
    button: Button<'_>,
    add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
) -> InnerResponse<Option<R>> {
    let bar_id = ui.id();

    let mut bar_state = BarState::load(ui.ctx(), bar_id);
    let button_response = ui.add(button);
    let inner = bar_state.bar_menu(&button_response, add_contents);

    bar_state.store(ui.ctx(), bar_id);
    InnerResponse::new(inner.map(|r| r.inner), button_response)
}

pub(crate) const CONTEXT_MENU_ID_STR: &str = "__egui::context_menu";

/// Response to secondary clicks (right-clicks) by showing the given menu.
pub fn context_menu(
    response: &Response,
    add_contents: impl FnOnce(&mut Ui),
) -> Option<InnerResponse<()>> {
    let menu_id = Id::new(CONTEXT_MENU_ID_STR);
    let mut bar_state = BarState::load(&response.ctx, menu_id);

    MenuRoot::context_click_interaction(response, &mut bar_state);
    let inner_response = bar_state.show(response, add_contents);

    bar_state.store(&response.ctx, menu_id);
    inner_response
}

/// Returns `true` if the context menu is opened for this widget.
pub fn context_menu_opened(response: &Response) -> bool {
    let menu_id = Id::new(CONTEXT_MENU_ID_STR);
    let bar_state = BarState::load(&response.ctx, menu_id);
    bar_state.is_menu_open(response.id)
}

/// Stores the state for the context menu.
#[derive(Clone, Default)]
pub struct MenuRootManager {
    inner: Option<MenuRoot>,
}

impl MenuRootManager {
    /// Show a menu at pointer if right-clicked response.
    ///
    /// Should be called from [`Context`] on a [`Response`]
    pub fn show<R>(
        &mut self,
        button: &Response,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        if let Some(root) = self.inner.as_mut() {
            let (menu_response, inner_response) = root.show(button, add_contents);
            if menu_response.is_close() {
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
pub struct MenuRoot {
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
        &self,
        button: &Response,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> (MenuResponse, Option<InnerResponse<R>>) {
        if self.id == button.id {
            let inner_response = menu_popup(
                &button.ctx,
                button.layer_id,
                &self.menu_state,
                self.id,
                add_contents,
            );
            let menu_state = self.menu_state.read();

            let escape_pressed = button.ctx.input(|i| i.key_pressed(Key::Escape));
            if menu_state.response.is_close()
                || escape_pressed
                || inner_response.response.should_close()
            {
                return (MenuResponse::Close, Some(inner_response));
            }
        }
        (MenuResponse::Stay, None)
    }

    /// Interaction with a stationary menu, i.e. fixed in another Ui.
    ///
    /// Responds to primary clicks.
    fn stationary_interaction(button: &Response, root: &mut MenuRootManager) -> MenuResponse {
        let id = button.id;

        if (button.clicked() && root.is_menu_open(id))
            || button.ctx.input(|i| i.key_pressed(Key::Escape))
        {
            // menu open and button clicked or esc pressed
            return MenuResponse::Close;
        } else if (button.clicked() && !root.is_menu_open(id))
            || (button.hovered() && root.is_some())
        {
            // menu not open and button clicked
            // or button hovered while other menu is open
            let mut pos = button.rect.left_bottom();

            let menu_frame = Frame::menu(&button.ctx.global_style());
            pos.x -= menu_frame.total_margin().left; // Make fist button in menu align with the parent button
            pos.y += button.ctx.global_style().spacing.menu_spacing;

            if let Some(root) = root.inner.as_mut() {
                let menu_rect = root.menu_state.read().rect;
                let content_rect = button.ctx.input(|i| i.content_rect());

                if pos.y + menu_rect.height() > content_rect.max.y {
                    pos.y = content_rect.max.y - menu_rect.height() - button.rect.height();
                }

                if pos.x + menu_rect.width() > content_rect.max.x {
                    pos.x = content_rect.max.x - menu_rect.width();
                }
            }

            if let Some(to_global) = button.ctx.layer_transform_to_global(button.layer_id) {
                pos = to_global * pos;
            }

            return MenuResponse::Create(pos, id);
        } else if button
            .ctx
            .input(|i| i.pointer.any_pressed() && i.pointer.primary_down())
            && let Some(pos) = button.ctx.input(|i| i.pointer.interact_pos())
            && let Some(root) = root.inner.as_mut()
            && root.id == id
        {
            // pressed somewhere while this menu is open
            let in_menu = root.menu_state.read().area_contains(pos);
            if !in_menu {
                return MenuResponse::Close;
            }
        }
        MenuResponse::Stay
    }

    /// Interaction with a context menu (secondary click).
    pub fn context_interaction(response: &Response, root: &mut Option<Self>) -> MenuResponse {
        let response = response.interact(Sense::click());
        let hovered = response.hovered();
        let secondary_clicked = response.secondary_clicked();

        response.ctx.input(|input| {
            let pointer = &input.pointer;
            if let Some(pos) = pointer.interact_pos() {
                let (in_old_menu, destroy) = if let Some(root) = root {
                    let in_old_menu = root.menu_state.read().area_contains(pos);
                    let destroy = !in_old_menu && pointer.any_pressed() && root.id == response.id;
                    (in_old_menu, destroy)
                } else {
                    (false, false)
                };
                if !in_old_menu {
                    if hovered && secondary_clicked {
                        return MenuResponse::Create(pos, response.id);
                    } else if destroy || hovered && pointer.primary_down() {
                        return MenuResponse::Close;
                    }
                }
            }
            MenuResponse::Stay
        })
    }

    pub fn handle_menu_response(root: &mut MenuRootManager, menu_response: MenuResponse) {
        match menu_response {
            MenuResponse::Create(pos, id) => {
                root.inner = Some(Self::new(pos, id));
            }
            MenuResponse::Close => root.inner = None,
            MenuResponse::Stay => {}
        }
    }

    /// Respond to secondary (right) clicks.
    pub fn context_click_interaction(response: &Response, root: &mut MenuRootManager) {
        let menu_response = Self::context_interaction(response, root);
        Self::handle_menu_response(root, menu_response);
    }

    // Responds to primary clicks.
    pub fn stationary_click_interaction(button: &Response, root: &mut MenuRootManager) {
        let menu_response = Self::stationary_interaction(button, root);
        Self::handle_menu_response(root, menu_response);
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum MenuResponse {
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
        response: &Response,
        menu_state: &MenuState,
        sub_id: Id,
    ) -> &'a WidgetVisuals {
        if menu_state.is_open(sub_id) && !response.hovered() {
            &ui.style().visuals.widgets.open
        } else {
            ui.style().interact(response)
        }
    }

    #[inline]
    pub fn icon(mut self, icon: impl Into<WidgetText>) -> Self {
        self.icon = icon.into();
        self
    }

    pub(crate) fn show(self, ui: &mut Ui, menu_state: &MenuState, sub_id: Id) -> Response {
        let Self { text, icon, .. } = self;

        let text_style = TextStyle::Button;
        let sense = Sense::click();

        let text_icon_gap = ui.spacing().item_spacing.x;
        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding;
        let text_available_width = ui.available_width() - total_extra.x;
        let text_galley = text.into_galley(
            ui,
            Some(TextWrapMode::Wrap),
            text_available_width,
            text_style.clone(),
        );

        let icon_available_width = text_available_width - text_galley.size().x;
        let icon_galley = icon.into_galley(
            ui,
            Some(TextWrapMode::Wrap),
            icon_available_width,
            text_style,
        );
        let text_and_icon_size = Vec2::new(
            text_galley.size().x + text_icon_gap + icon_galley.size().x,
            text_galley.size().y.max(icon_galley.size().y),
        );
        let mut desired_size = text_and_icon_size + 2.0 * button_padding;
        desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);

        let (rect, response) = ui.allocate_at_least(desired_size, sense);
        response.widget_info(|| {
            crate::WidgetInfo::labeled(
                crate::WidgetType::Button,
                ui.is_enabled(),
                text_galley.text(),
            )
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
                    visuals.corner_radius,
                    visuals.weak_bg_fill,
                );
            }

            let text_color = visuals.text_color();
            ui.painter().galley(text_pos, text_galley, text_color);
            ui.painter().galley(icon_pos, icon_galley, text_color);
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
        let response = self.button.show(ui, &self.parent_state.read(), sub_id);
        self.parent_state
            .write()
            .submenu_button_interaction(ui, sub_id, &response);
        let inner =
            self.parent_state
                .write()
                .show_submenu(ui.ctx(), ui.layer_id(), sub_id, add_contents);
        InnerResponse::new(inner, response)
    }
}

/// Components of menu state, public for advanced usage.
///
/// Usually you don't need to use it directly.
pub struct MenuState {
    /// The opened sub-menu and its [`Id`]
    sub_menu: Option<(Id, Arc<RwLock<Self>>)>,

    /// Bounding box of this menu (without the sub-menu),
    /// including the frame and everything.
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

    fn show_submenu<R>(
        &mut self,
        ctx: &Context,
        parent_layer: LayerId,
        id: Id,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        let (sub_response, response) = self.submenu(id).map(|sub| {
            let inner_response = menu_popup(ctx, parent_layer, sub, id, add_contents);
            if inner_response.response.should_close() {
                sub.write().close();
            }
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
                .is_some_and(|(_, sub)| sub.read().area_contains(pos))
    }

    fn next_entry_index(&mut self) -> usize {
        self.entry_count += 1;
        self.entry_count - 1
    }

    /// Sense button interaction opening and closing submenu.
    fn submenu_button_interaction(&mut self, ui: &Ui, sub_id: Id, button: &Response) {
        let pointer = ui.input(|i| i.pointer.clone());
        let open = self.is_open(sub_id);
        if self.moving_towards_current_submenu(&pointer) {
            // We don't close the submenu if the pointer is on its way to hover it.
            // ensure to repaint once even when pointer is not moving
            ui.request_repaint();
        } else if !open && button.hovered() {
            // TODO(emilk): open menu to the left if there isn't enough space to the right
            let mut pos = button.rect.right_top();
            pos.x = self.rect.right() + ui.spacing().menu_spacing;
            pos.y -= Frame::menu(ui.style()).total_margin().top; // align the first button in the submenu with the parent button

            self.open_submenu(sub_id, pos);
        } else if open
            && ui.response().contains_pointer()
            && !button.hovered()
            && !self.hovering_current_submenu(&pointer)
        {
            // We are hovering something else in the menu, so close the submenu.
            self.close_submenu();
        }
    }

    /// Check if pointer is moving towards current submenu.
    fn moving_towards_current_submenu(&self, pointer: &PointerState) -> bool {
        if pointer.is_still() {
            return false;
        }

        if let Some(sub_menu) = self.current_submenu()
            && let Some(pos) = pointer.hover_pos()
        {
            let rect = sub_menu.read().rect;
            return rect.intersects_ray(pos, pointer.direction().normalized());
        }
        false
    }

    /// Check if pointer is hovering current submenu.
    fn hovering_current_submenu(&self, pointer: &PointerState) -> bool {
        if let Some(sub_menu) = self.current_submenu()
            && let Some(pos) = pointer.hover_pos()
        {
            return sub_menu.read().area_contains(pos);
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

    fn current_submenu(&self) -> Option<&Arc<RwLock<Self>>> {
        self.sub_menu.as_ref().map(|(_, sub)| sub)
    }

    fn submenu(&self, id: Id) -> Option<&Arc<RwLock<Self>>> {
        let (k, sub) = self.sub_menu.as_ref()?;
        if id == *k { Some(sub) } else { None }
    }

    /// Open submenu at position, if not already open.
    fn open_submenu(&mut self, id: Id, pos: Pos2) {
        if !self.is_open(id) {
            self.sub_menu = Some((id, Arc::new(RwLock::new(Self::new(pos)))));
        }
    }

    fn close_submenu(&mut self) {
        self.sub_menu = None;
    }
}
