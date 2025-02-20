use crate::{
    Button, Color32, Context, Frame, Id, InnerResponse, Layout, PointerState, Popup, Response,
    RichText, Style, Ui, UiKind, UiStack, Widget, WidgetText,
};
use emath::{vec2, Align, RectAlign, Vec2};
use epaint::Stroke;

pub fn menu_style(style: &mut Style) {
    style.spacing.button_padding = vec2(2.0, 0.0);
    style.visuals.widgets.active.bg_stroke = Stroke::NONE;
    style.visuals.widgets.open.bg_stroke = Stroke::NONE;
    style.visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    style.visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_stroke = Stroke::NONE;
}

pub fn global_menu_state_id() -> Id {
    Id::new("global_menu_state")
}

pub fn find_sub_menu_root(ui: &Ui) -> &UiStack {
    ui.stack()
        .iter()
        .find(|stack| {
            // TODO: Add a MenuContainer widget that allows one to create a submenu from anywhere using UiStack::tags
            stack.is_root_ui() || [Some(UiKind::Popup), Some(UiKind::Menu)].contains(&stack.kind())
        })
        // It's fine to unwrap since we should always find the root
        .unwrap()
}

pub struct GlobalMenuState {
    popup_id: Option<Id>,
}

#[derive(Default, Clone)]
pub struct MenuState {
    pub open_item: Option<Id>,
}

impl MenuState {
    pub const ID: &'static str = "menu_state";
    pub fn from_ui<R>(ui: &Ui, f: impl FnOnce(&mut Self, &UiStack) -> R) -> R {
        let stack = find_sub_menu_root(ui);
        ui.data_mut(|data| {
            let state = data.get_temp_mut_or_default(stack.id.with(Self::ID));
            f(state, stack)
        })
    }

    pub fn from_id<R>(ctx: &Context, id: Id, f: impl FnOnce(&mut Self) -> R) -> R {
        ctx.data_mut(|data| {
            let state = data.get_temp_mut_or_default(id.with(Self::ID));
            f(state)
        })
    }
}

/// A submenu button that shows a [`SubMenu`] if a [`Button`] is hovered.
pub struct SubMenuButton<'a> {
    pub button: Button<'a>,
    pub sub_menu: SubMenu,
}

impl<'a> SubMenuButton<'a> {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            button: Button::new(text).right_text(RichText::new("⏵")),
            sub_menu: SubMenu::default(),
        }
    }

    pub fn button_mut(&mut self) -> &mut Button<'a> {
        &mut self.button
    }

    pub fn sub_menu_mut(&mut self) -> &mut SubMenu {
        &mut self.sub_menu
    }

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
pub struct SubMenu {}

impl SubMenu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id_from_widget_id(widget_id: Id) -> Id {
        widget_id.with("submenu")
    }

    pub fn show<R>(
        self,
        ui: &mut Ui,
        response: &Response,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let frame = Frame::menu(ui.style());

        let id = Self::id_from_widget_id(response.id);

        let (open_item, menu_id) =
            MenuState::from_ui(ui, |state, stack| (state.open_item, stack.id));

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

        let gap = frame.total_margin().sum().x / 2.0;

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
            .style(menu_style)
            .frame(frame)
            .show(content);

        if let Some(popup_response) = &popup_response {
            let has_any_open = MenuState::from_id(ui.ctx(), id, |state| state.open_item.is_some());
            // If no child sub menu is open means we must be the deepest child sub menu.
            // If the user clicks and the cursor is not hovering over our menu rect, it's
            // safe to assume they clicked outside the menu, so we close everything.
            // If they were to hover some other parent submenu we wouldn't be open.
            // Only edge case is the user hovering this submenu's button, so we also check
            // if we clicked outside the parent menu (which we luckily have access to here).
            let clicked_outside = !has_any_open
                && popup_response.response.clicked_elsewhere()
                && menu_root_response.clicked_elsewhere();

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
            if close_called || clicked_outside {
                ui.close();
            }

            if hovering_other_menu_entry || close_called || clicked_outside {
                set_open = Some(false);
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
