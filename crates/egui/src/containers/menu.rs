use crate::{
    Button, Color32, Frame, Id, InnerResponse, Layout, PointerState, Popup, Response, Style, Ui,
    UiKind, UiStack, Widget, WidgetText,
};
use emath::{vec2, Align, RectAlign};
use epaint::Stroke;
use std::sync::Arc;

pub fn menu_style(style: &mut Style) {
    style.spacing.button_padding = vec2(2.0, 0.0);
    style.visuals.widgets.active.bg_stroke = Stroke::NONE;
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
    pub fn from_ui<R>(ui: &Ui, f: impl FnOnce(&mut Self, &UiStack) -> R) -> R {
        let stack = find_sub_menu_root(ui);
        ui.data_mut(|data| {
            let state = data.get_temp_mut_or_default(stack.id);
            f(state, stack)
        })
    }

    pub fn from_id<R>(ui: &Ui, id: Id, f: impl FnOnce(&mut Self) -> R) -> R {
        ui.data_mut(|data| {
            let state = data.get_temp_mut_or_default(id);
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
            button: Button::new(text).shortcut_text("⏵"), // TODO: Somehow set a color for the shortcut text
            sub_menu: SubMenu,
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
        let response = self.button.ui(ui);

        let popup_response = self.sub_menu.show(ui, &response, content);

        (response, popup_response)
    }
}

pub struct SubMenu;

impl SubMenu {
    pub fn show<R>(
        self,
        ui: &Ui,
        response: &Response,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        let frame = Frame::menu(ui.style());

        let id = response.id.with("submenu");

        let (open_item, menu_id) =
            MenuState::from_ui(ui, |state, stack| (state.open_item, stack.id));

        let mut menu_root_response = ui
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

        if !is_any_open && is_hovered {
            set_open = Some(true);
            is_open = true;
        }

        let gap = frame.total_margin().sum().x / 2.0;

        let popup_response = Popup::from_response(&response)
            .open(is_open)
            .align(RectAlign::RIGHT_START)
            .layout(Layout::top_down_justified(Align::Min))
            .gap(gap)
            .style(menu_style)
            .show(content);

        if let Some(popup_response) = &popup_response {
            let is_moving_towards_rect = ui.input(|i| {
                i.pointer
                    .is_moving_towards_rect(&popup_response.response.rect)
            });
            if is_moving_towards_rect {
                // We need to repaint while this is true, so we can detect when
                // the pointer is no longer moving towards the rect
                ui.ctx().request_repaint();
            }
            if is_open
                && !is_hovered
                && !popup_response.response.contains_pointer()
                && !is_moving_towards_rect
                && is_hovering_menu
            {
                set_open = Some(false);
            }
        }

        if let Some(set_open) = set_open {
            MenuState::from_id(ui, menu_id, |state| {
                state.open_item = set_open.then_some(id);
            });
        }

        popup_response
    }
}
