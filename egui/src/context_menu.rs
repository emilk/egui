use super::{
    style::{Spacing, WidgetVisuals},
    Align, CtxRef, Id, InnerResponse, PointerState, Pos2, Rect, Response, Sense, Style, TextStyle,
    Ui, Vec2,
};

#[derive(Default)]
pub struct ContextMenuSystem {
    context_menu: Option<ContextMenuRoot>,
}

impl ContextMenuSystem {
    /// Sense if a context menu needs to be (re-)created or destroyed
    fn sense_click(&mut self, response: &Response) -> MenuResponse {
        let response = response.interact(Sense::click());
        let pointer = &response.ctx.input().pointer;
        if pointer.any_pressed() {
            if let Some(pos) = pointer.interact_pos() {
                //if let Some(context_menu) = &mut self.context_menu {
                //    let in_old_menu = context_menu.area_contains(pos);
                //    if !in_old_menu {
                //        let in_target = response.rect.contains(pos);
                //        let destroy = context_menu.ui_id == response.id;
                //        if in_target && pointer.secondary_down() {
                //            return MenuResponse::Create(pos);
                //        } else if (in_target && pointer.primary_down()) || destroy {
                //            return MenuResponse::Close;
                //        }
                //    }
                //}
                let mut destroy = false;
                let mut in_old_menu = false;
                if let Some(context_menu) = &mut self.context_menu {
                    in_old_menu = context_menu.area_contains(pos);
                    destroy = context_menu.ui_id == response.id;
                }
                if !in_old_menu {
                    let in_target = response.rect.contains(pos);
                    if in_target && pointer.secondary_down() {
                        return MenuResponse::Create(pos);
                    } else if (in_target && pointer.primary_down()) || destroy {
                        return MenuResponse::Close;
                    }
                }
            }
        }
        MenuResponse::Stay
    }
    /// Show the current context menu
    fn show(
        &mut self,
        response: &Response,
        add_contents: impl FnOnce(&mut Ui, &mut MenuState),
    ) -> MenuResponse {
        if let Some(context_menu) = &mut self.context_menu {
            if context_menu.ui_id == response.id {
                let response = context_menu.show(&response.ctx, add_contents);
                context_menu.rect = response.rect;

                if context_menu.response.is_close() {
                    return MenuResponse::Close;
                }
            }
        }
        MenuResponse::Stay
    }
    /// Show a menu if right-clicked
    /// Should be called from [`Context`] on a [`Response`]
    pub fn context_menu(
        &mut self,
        response: &Response,
        add_contents: impl FnOnce(&mut Ui, &mut MenuState),
    ) {
        match self.sense_click(response) {
            MenuResponse::Create(pos) => {
                self.context_menu = Some(ContextMenuRoot::new(pos, response.id));
            }
            MenuResponse::Close => self.context_menu = None,
            MenuResponse::Stay => {}
        };
        if let MenuResponse::Close = self.show(response, add_contents) {
            self.context_menu = None
        }
    }
}
#[derive(Clone, PartialEq)]
enum MenuResponse {
    Close,
    Stay,
    Create(Pos2),
}
impl MenuResponse {
    pub fn is_close(&self) -> bool {
        *self == Self::Close
    }
}
/// Context menu root associated with an Id from a Response
struct ContextMenuRoot {
    context_menu: MenuState,
    ui_id: Id,
}
impl ContextMenuRoot {
    pub fn new(position: Pos2, ui_id: Id) -> Self {
        Self {
            context_menu: MenuState::new(position),
            ui_id,
        }
    }
    fn show(
        &mut self,
        ctx: &CtxRef,
        add_contents: impl FnOnce(&mut Ui, &mut MenuState),
    ) -> Response {
        self.context_menu.show(ctx, self.ui_id, add_contents)
    }
}
impl std::ops::Deref for ContextMenuRoot {
    type Target = MenuState;
    fn deref(&self) -> &Self::Target {
        &self.context_menu
    }
}
impl std::ops::DerefMut for ContextMenuRoot {
    fn deref_mut(&mut self) -> &mut <Self as std::ops::Deref>::Target {
        &mut self.context_menu
    }
}
pub enum EntryState {
    /// Will not show hover visuals
    Inactive,
    /// Listening for hovers
    Active,
    /// Show open visuals
    Open,
}
impl EntryState {
    fn visuals<'a>(self, ui: &'a Ui, response: &'_ Response) -> &'a WidgetVisuals {
        let widgets = &ui.style().visuals.widgets;
        match self {
            Self::Inactive => &widgets.inactive,
            Self::Active => ui.style().interact(response),
            Self::Open => &widgets.hovered,
        }
    }
    fn submenu(menu_state: &MenuState, sub_id: Id) -> Self {
        if menu_state.is_open(sub_id) {
            Self::Open
        } else if menu_state.any_open() {
            Self::Inactive
        } else {
            Self::Active
        }
    }
    fn entry(menu_state: &MenuState) -> Self {
        if menu_state.any_open() {
            Self::Inactive
        } else {
            Self::Active
        }
    }
}
pub struct MenuEntry {
    text: String,
    icon: String,
    state: EntryState,
    index: usize,
}
impl MenuEntry {
    /// The `icon` can be an emoji (e.g. `⏵` right arrow), shown right of the label
    #[allow(clippy::needless_pass_by_value)]
    fn new(text: impl ToString, icon: impl ToString, state: EntryState, index: usize) -> Self {
        Self {
            text: text.to_string(),
            icon: icon.to_string(),
            state,
            index,
        }
    }
    #[allow(clippy::needless_pass_by_value)]
    pub fn icon(mut self, icon: impl ToString) -> Self {
        self.icon = icon.to_string();
        self
    }
    fn show_with_state(mut self, ui: &mut Ui, state: EntryState) -> Response {
        self.state = state;
        self.show(ui)
    }
    pub fn show(self, ui: &mut Ui) -> Response {
        let MenuEntry {
            text, icon, state, ..
        } = self;

        let text_style = TextStyle::Button;
        let sense = Sense::click();

        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding;
        let text_available_width = ui.available_width() - total_extra.x;
        let text_galley = ui
            .fonts()
            .layout_multiline(text_style, text, text_available_width);

        let icon_available_width = text_available_width - text_galley.size.x;
        let icon_galley = ui
            .fonts()
            .layout_multiline(text_style, icon, icon_available_width);
        let text_and_icon_size = Vec2::new(
            text_galley.size.x + icon_galley.size.x,
            text_galley.size.y.max(icon_galley.size.y),
        );
        let desired_size = text_and_icon_size + 2.0 * button_padding;

        let (rect, response) = ui.allocate_at_least(desired_size, sense);
        response.widget_info(|| {
            crate::WidgetInfo::labeled(crate::WidgetType::Button, &text_galley.text)
        });

        if ui.clip_rect().intersects(rect) {
            response.interact(sense);
            let visuals = state.visuals(ui, &response);
            let text_pos = ui
                .layout()
                .align_size_within_rect(text_galley.size, rect.shrink2(button_padding))
                .min;
            let icon_pos = ui
                .layout()
                .with_cross_align(Align::RIGHT)
                .align_size_within_rect(icon_galley.size, rect.shrink2(button_padding))
                .min;

            let fill = visuals.bg_fill;
            let stroke = crate::Stroke::none();
            ui.painter().rect(
                rect.expand(visuals.expansion),
                visuals.corner_radius,
                fill,
                stroke,
            );

            let text_color = visuals.text_color();
            ui.painter().galley(text_pos, text_galley, text_color);
            ui.painter().galley(icon_pos, icon_galley, text_color);
        }
        response
    }
}
pub struct SubMenu<'a> {
    entry: MenuEntry,
    parent_state: &'a mut MenuState,
}
impl<'a> SubMenu<'a> {
    #[allow(clippy::needless_pass_by_value)]
    fn new(text: impl ToString, parent_state: &'a mut MenuState, index: usize) -> Self {
        Self {
            entry: MenuEntry::new(text, "⏵", EntryState::Active, index),
            parent_state,
        }
    }
    pub fn show(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui, &mut MenuState),
    ) -> InnerResponse<Option<Response>> {
        let sub_id = ui.id().with(self.entry.index);
        let button = self
            .entry
            .show_with_state(ui, EntryState::submenu(self.parent_state, sub_id));
        self.parent_state
            .submenu_button_interaction(ui, sub_id, &button);
        let inner = self
            .parent_state
            .show_submenu(ui.ctx(), sub_id, add_contents);
        InnerResponse::new(inner, button)
    }
}
pub struct MenuState {
    /// The opened sub-menu and its `Id`
    sub_menu: Option<(Id, Box<MenuState>)>,
    /// Bounding box of this menu (without the sub-menu)
    rect: Rect,
    /// Used to check if any menu in the tree wants to close
    response: MenuResponse,
    /// Used to hash different `Id`s for sub-menus
    entry_count: usize,
}
impl MenuState {
    /// Close menu hierarchy.
    pub fn close(&mut self) {
        self.response = MenuResponse::Close;
    }
    /// Create a menu item.
    pub fn item(&mut self, text: impl ToString) -> MenuEntry {
        MenuEntry::new(text, "", EntryState::entry(self), self.next_entry_index())
    }
    /// Create a menu item with an icon (right adjusted).
    pub fn item_with_icon(&mut self, text: impl ToString, icon: impl ToString) -> MenuEntry {
        MenuEntry::new(text, icon, EntryState::entry(self), self.next_entry_index())
    }
    fn next_entry_index(&mut self) -> usize {
        self.entry_count += 1;
        self.entry_count - 1
    }
    /// Create a sub-menu.
    pub fn submenu(&'_ mut self, text: impl ToString) -> SubMenu<'_> {
        let index = self.next_entry_index();
        SubMenu::new(text, self, index)
    }
    fn new(position: Pos2) -> Self {
        Self {
            rect: Rect::from_min_size(position, Vec2::ZERO),
            sub_menu: None,
            response: MenuResponse::Stay,
            entry_count: 0,
        }
    }
    /// Sense button interaction opening and closing submenu.
    fn submenu_button_interaction(&mut self, ui: &mut Ui, sub_id: Id, button: &Response) {
        let pointer = &ui.input().pointer.clone();
        let open = self.is_open(sub_id);
        if self.moving_towards_current_submenu(pointer) {
            // ensure to repaint once even when pointer is not moving
            ui.ctx().request_repaint();
        } else if !open && button.hovered() {
            let pos = button.rect.right_top();
            self.open_submenu(sub_id, pos);
        } else if open && !button.hovered() && !self.hovering_current_submenu(pointer) {
            self.close_submenu();
        }
    }
    fn show_submenu(
        &mut self,
        ctx: &CtxRef,
        id: Id,
        add_contents: impl FnOnce(&mut Ui, &mut MenuState),
    ) -> Option<Response> {
        let (sub_response, response) = self.get_submenu(id).map(|sub| {
            let response = sub.show(ctx, id, add_contents);
            sub.rect = response.rect;
            (sub.response.clone(), response)
        })?;
        self.cascade_response(sub_response);
        Some(response)
    }
    fn show(
        &mut self,
        ctx: &CtxRef,
        id: Id,
        add_contents: impl FnOnce(&mut Ui, &mut MenuState),
    ) -> Response {
        self.entry_count = 0;
        let style = Style {
            spacing: Spacing {
                item_spacing: Vec2::ZERO,
                button_padding: crate::vec2(2.0, 0.0),
                ..Default::default()
            },
            ..Default::default()
        };
        crate::menu::menu_ui(ctx, id, self.rect.min, style, |ui| {
            ui.set_width(100.0);
            add_contents(ui, self)
        })
        .response
    }
    /// Check if position is in the menu hierarchy's area.
    fn area_contains(&self, pos: Pos2) -> bool {
        self.rect.contains(pos)
            || self
                .sub_menu
                .as_ref()
                .map(|(_, sub)| sub.area_contains(pos))
                .unwrap_or(false)
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
        if let Some(sub_menu) = self.get_current_submenu() {
            if let Some(pos) = pointer.hover_pos() {
                return Self::points_at_left_of_rect(pos, pointer.velocity(), sub_menu.rect);
            }
        }
        false
    }
    /// Check if pointer is hovering current submenu.
    fn hovering_current_submenu(&self, pointer: &PointerState) -> bool {
        if let Some(sub_menu) = self.get_current_submenu() {
            if let Some(pos) = pointer.hover_pos() {
                return sub_menu.area_contains(pos);
            }
        }
        false
    }
    /// Cascade close response to menu root.
    fn cascade_response(&mut self, response: MenuResponse) {
        if response.is_close() {
            self.response = response;
        }
    }
    fn any_open(&self) -> bool {
        self.get_sub_id().is_some()
    }
    fn is_open(&self, id: Id) -> bool {
        self.get_sub_id() == Some(id)
    }
    fn get_sub_id(&self) -> Option<Id> {
        self.sub_menu.as_ref().map(|(id, _)| *id)
    }
    fn get_current_submenu(&self) -> Option<&MenuState> {
        self.sub_menu.as_ref().map(|(_, sub)| sub.as_ref())
    }
    fn get_submenu(&mut self, id: Id) -> Option<&mut MenuState> {
        self.sub_menu
            .as_mut()
            .and_then(|(k, sub)| if id == *k { Some(sub.as_mut()) } else { None })
    }
    /// Open submenu at position, if not already open.
    fn open_submenu(&mut self, id: Id, pos: Pos2) {
        if !self.is_open(id) {
            self.sub_menu = Some((id, Box::new(MenuState::new(pos))));
        }
    }
    fn close_submenu(&mut self) {
        self.sub_menu = None;
    }
}
