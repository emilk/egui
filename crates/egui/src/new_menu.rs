use std::sync::Arc;

use emath::Rect;
use mutex::RwLock;

use crate::*;

pub(crate) const CONTEXT_MENU_ID_STR: &str = "__egui::context_menu";

pub fn menu<R>(
    ui: &Ui,
    response: &Response,
    handle: MenuHandle,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<InnerResponse<R>> {
    Menu::new(ui.id(), handle)
        .parent(ui.new_menu_state())
        .show(ui, response, add_contents)
}

#[derive(Clone, Default)]
pub struct MenuState {
    open_menu: MenuRoot,
}

impl MenuState {
    pub fn load(ctx: &Context, menu_id: Id) -> Self {
        ctx.data_mut(|d| d.get_temp::<Self>(menu_id).unwrap_or_default())
    }

    pub fn store(self, ctx: &Context, menu_id: Id) {
        ctx.data_mut(|d| d.insert_temp(menu_id, self));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuHandle {
    Click,
    Hover,
    Context,
}

impl MenuHandle {
    pub fn as_dyn(&self) -> MenuHandleDyn {
        match self {
            Self::Click => Box::new(click_handle),
            Self::Hover => Box::new(hover_handle),
            Self::Context => Box::new(context_handle),
        }
    }
}

pub type MenuHandleDyn = Box<dyn Fn(&Ui, &Response, Id, &MenuRoot) -> MenuResponse>;

pub fn hover_handle(ui: &Ui, response: &Response, id: Id, root: &MenuRoot) -> MenuResponse {
    let pointer = response.ctx.input(|i| i.pointer.clone());
    let open = root.is_menu_open(id);
    if root
        .inner
        .clone()
        .is_some_and(|menu| menu.inner.read().is_moving_towards_current_menu(&pointer))
    {
        // We don't close the submenu if the pointer is on its way to hover it.
        // ensure to repaint once even when pointer is not moving
        response.ctx.request_repaint();
        MenuResponse::Stay
    } else if !open && response.hovered() {
        // TODO(emilk): open menu to the left if there isn't enough space to the right
        let mut pos = response.rect.right_top();

        pos.x += ui.spacing().menu_spacing;
        pos.y -= Frame::menu(ui.style()).total_margin().top; // align the first button in the submenu with the parent button

        MenuResponse::Create(pos, id)
    } else if open
        && ui.interact_bg(Sense::hover()).contains_pointer()
        && !response.hovered()
        && !root
            .inner
            .clone()
            .is_some_and(|menu| menu.inner.read().is_hovering_current_menu(&pointer))
    {
        // We are hovering something else in the menu, so close the submenu.
        MenuResponse::Close
    } else {
        MenuResponse::Stay
    }
}

pub fn click_handle(_: &Ui, response: &Response, id: Id, root: &MenuRoot) -> MenuResponse {
    if (response.clicked() && root.is_menu_open(id))
        || response.ctx.input(|i| i.key_pressed(Key::Escape))
    {
        // menu open and button clicked or esc pressed
        return MenuResponse::Close;
    } else if (response.clicked() && !root.is_menu_open(id))
        || (response.hovered() && root.inner.is_some())
    {
        // menu not open and button clicked
        // or button hovered while other menu is open
        let mut pos = response.rect.left_bottom();

        let menu_frame = Frame::menu(&response.ctx.style());
        pos.x -= menu_frame.total_margin().left; // Make fist button in menu align with the parent button
        pos.y += response.ctx.style().spacing.menu_spacing;

        if let Some(root) = root.inner.as_ref() {
            let menu_rect = root.inner.read().rect;
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
            if let Some(root) = root.inner.as_ref() {
                if root.id == id {
                    // pressed somewhere while this menu is open
                    let in_menu = root.inner.read().area_contains(pos);
                    if !in_menu {
                        return MenuResponse::Close;
                    }
                }
            }
        }
    }
    MenuResponse::Stay
}

pub fn context_handle(_: &Ui, response: &Response, _: Id, root: &MenuRoot) -> MenuResponse {
    let response = response.interact(Sense::click());
    let hovered = response.hovered();
    let secondary_clicked = response.secondary_clicked();

    response.ctx.input(|input| {
        let pointer = &input.pointer;
        if let Some(pos) = pointer.interact_pos() {
            let mut in_old_menu = false;
            let mut destroy = false;

            if let Some(root) = &root.inner {
                in_old_menu = root.inner.read().area_contains(pos);
                destroy = !in_old_menu && pointer.any_pressed() && root.id == response.id;
            }

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

#[derive(Debug, Clone, Copy)]
pub enum MenuResponse {
    Create(Pos2, Id),
    Stay,
    Close,
}

#[derive(Clone, Default)]
pub struct MenuRoot {
    pub inner: Option<MenuDataWithId>,
}

#[derive(Clone)]
pub struct MenuDataWithId {
    pub id: Id,
    pub inner: Arc<RwLock<MenuData>>,
}

impl MenuDataWithId {
    pub fn new(pos: Pos2, id: Id) -> Self {
        Self {
            id,
            inner: Arc::new(RwLock::new(MenuData::new(pos))),
        }
    }
}
impl MenuRoot {
    pub fn is_menu_open(&self, id: Id) -> bool {
        self.inner.as_ref().map(|m| m.id) == Some(id)
    }

    pub fn handle_menu_response(&mut self, ctx: &Context, menu_response: MenuResponse) {
        match menu_response {
            MenuResponse::Create(pos, id) => {
                self.inner = Some(MenuDataWithId::new(pos, id));
            }
            MenuResponse::Close => {
                if let Some(data) = &self.inner {
                    data.inner.write().close(ctx);
                };
                self.inner = None;
            }
            MenuResponse::Stay => {}
        }
    }
}

pub struct MenuData {
    pub sub_menu: Option<MenuDataWithId>,
    pub rect: Rect,
}

impl MenuData {
    pub fn new(pos: Pos2) -> Self {
        Self {
            sub_menu: None,
            rect: Rect::from_min_size(pos, Vec2::ZERO),
        }
    }

    pub fn area_contains(&self, pos: Pos2) -> bool {
        self.rect.contains(pos)
            || self
                .sub_menu
                .as_ref()
                .map_or(false, |menu| menu.inner.read().area_contains(pos))
    }

    fn close(&mut self, ctx: &Context) {
        if let Some(MenuDataWithId { id, inner }) = self.sub_menu.as_ref() {
            inner.write().close(ctx);
            MenuState::default().store(ctx, *id);
        }

        self.sub_menu = None;
    }

    fn is_moving_towards_current_menu(&self, pointer: &PointerState) -> bool {
        if pointer.is_still() {
            return false;
        }

        pointer.hover_pos().map_or(false, |pos| {
            self.rect
                .intersects_ray(pos, pointer.velocity().normalized())
        })
    }

    fn is_hovering_current_menu(&self, pointer: &PointerState) -> bool {
        pointer
            .hover_pos()
            .map_or(false, |pos| self.area_contains(pos))
    }
}

pub struct Menu {
    pub id: Id,
    pub intaractable: bool,
    pub parent: Option<Arc<RwLock<MenuData>>>,
    pub handle: MenuHandleDyn,
    pub pivot: Align2,
}

impl Menu {
    pub fn new(mut id: Id, handle: MenuHandle) -> Self {
        if handle == MenuHandle::Context {
            id = Id::new(CONTEXT_MENU_ID_STR);
        }
        Self {
            id,
            intaractable: true,
            parent: None,
            handle: handle.as_dyn(),
            pivot: Align2::LEFT_TOP,
        }
    }

    pub fn interactable(mut self, interactable: bool) -> Self {
        self.intaractable = interactable;
        self
    }

    pub fn parent(mut self, parent: Option<Arc<RwLock<MenuData>>>) -> Self {
        self.parent = parent;
        self
    }

    pub fn pivot(mut self, pivot: Align2) -> Self {
        self.pivot = pivot;
        self
    }

    pub fn show<R>(
        self,
        ui: &Ui,
        response: &Response,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let mut state = MenuState::load(ui.ctx(), self.id);

        self.set_sub_menu(&state);
        self.handle_interaction(ui, response, &mut state);

        let response = state
            .open_menu
            .inner
            .as_ref()
            .map(|root| self.show_internal(ui.ctx(), &root.inner, add_contents));

        state.store(ui.ctx(), self.id);

        response
    }

    fn handle_interaction(&self, ui: &Ui, response: &Response, state: &mut MenuState) {
        let menu_response = (self.handle)(ui, response, self.id, &mut state.open_menu);
        state
            .open_menu
            .handle_menu_response(ui.ctx(), menu_response);
    }

    fn set_sub_menu(&self, state: &MenuState) {
        if let Some(parent) = &self.parent {
            let mut parent = parent.write();
            if parent.sub_menu.is_none() {
                parent.sub_menu = state.open_menu.inner.clone();
            }
        }
    }

    fn show_internal<R>(
        &self,
        ctx: &Context,
        menu_data_arc: &Arc<RwLock<MenuData>>,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let area = Area::new(self.id.with("__menu"))
            .kind(UiKind::Menu)
            .order(Order::Foreground)
            .fixed_pos(menu_data_arc.read().rect.min)
            .interactable(self.intaractable)
            .default_width(ctx.style().spacing.menu_width)
            .pivot(self.pivot)
            .sense(Sense::hover());

        let mut sizing_pass = false;

        let area_response = area.show(ctx, |ui| {
            sizing_pass = ui.is_sizing_pass();

            Frame::menu(ui.style())
                .show(ui, |ui| {
                    ui.set_new_menu_state(Some(menu_data_arc.clone()));
                    add_contents(ui)
                })
                .inner
        });

        menu_data_arc.write().rect = if sizing_pass {
            // During the sizing pass we didn't know the size yet,
            // so we might have just constrained the position unnecessarily.
            // Therefore keep the original=desired position until the next frame.
            Rect::from_min_size(
                menu_data_arc.read().rect.min,
                area_response.response.rect.size(),
            )
        } else {
            // We knew the size, and this is where it ended up (potentially constrained to screen).
            // Remember it for the future:
            area_response.response.rect
        };

        area_response
    }
}
