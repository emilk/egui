use crate::{widgets::*, *};

use super::*;

#[derive(Clone, Copy, Debug, serde_derive::Deserialize, serde_derive::Serialize)]
pub struct BarState {
    #[serde(skip)]
    open_menu: Option<Id>,
    #[serde(skip)]
    /// When did we open a menu?
    open_time: f64,
}

impl Default for BarState {
    fn default() -> Self {
        Self {
            open_menu: None,
            open_time: f64::NEG_INFINITY,
        }
    }
}

pub fn bar(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui)) -> InteractInfo {
    ui.inner_layout(Layout::horizontal(Align::Center), |ui| {
        Frame::menu_bar(ui.style()).show(ui, |ui| {
            let mut style = ui.style().clone();
            style.button_padding = vec2(2.0, 0.0);
            // style.interact.active.bg_fill_color = None;
            style.interact.active.rect_outline = None;
            // style.interact.hovered.bg_fill_color = None;
            style.interact.hovered.rect_outline = None;
            style.interact.inactive.bg_fill_color = None;
            style.interact.inactive.rect_outline = None;
            ui.set_style(style);

            // Take full width and fixed height:
            let height = ui.style().menu_bar.height;
            ui.set_desired_height(height);
            ui.expand_to_size(vec2(ui.available().width(), height));
            add_contents(ui)
        })
    })
}

/// Construct a top level menu in a menu bar. This would be e.g. "File", "Edit" etc.
pub fn menu(ui: &mut Ui, title: impl Into<String>, add_contents: impl FnOnce(&mut Ui)) {
    let title = title.into();
    let bar_id = ui.id();
    let menu_id = Id::new(&title);

    let mut bar_state = ui
        .memory()
        .menu_bar
        .get(&bar_id)
        .cloned()
        .unwrap_or_default();

    let mut button = Button::new(title);

    if bar_state.open_menu == Some(menu_id) {
        button = button.fill_color(Some(ui.style().interact.active.fill_color));
    }

    let button_interact = ui.add(button);

    interact_with_menu_button(&mut bar_state, ui.input(), menu_id, &button_interact);

    if bar_state.open_menu == Some(menu_id) {
        let area = Area::new(menu_id)
            .order(Order::Foreground)
            .fixed_pos(button_interact.rect.left_bottom());
        let frame = Frame::menu(ui.style());

        let resize = Resize::default().auto_sized();

        let menu_interact = area.show(ui.ctx(), |ui| {
            frame.show(ui, |ui| {
                resize.show(ui, |ui| {
                    let mut style = ui.style().clone();
                    style.button_padding = vec2(2.0, 0.0);
                    // style.interact.active.bg_fill_color = None;
                    style.interact.active.rect_outline = None;
                    // style.interact.hovered.bg_fill_color = None;
                    style.interact.hovered.rect_outline = None;
                    style.interact.inactive.bg_fill_color = None;
                    style.interact.inactive.rect_outline = None;
                    ui.set_style(style);
                    ui.set_layout(Layout::justified(Direction::Vertical));
                    add_contents(ui)
                })
            })
        });

        if menu_interact.hovered && ui.input().mouse_released {
            bar_state.open_menu = None;
        }
    }

    ui.memory().menu_bar.insert(bar_id, bar_state);
}

fn interact_with_menu_button(
    bar_state: &mut BarState,
    input: &GuiInput,
    menu_id: Id,
    button_interact: &GuiResponse,
) {
    if button_interact.hovered && input.mouse_pressed {
        if bar_state.open_menu.is_some() {
            bar_state.open_menu = None;
        } else {
            bar_state.open_menu = Some(menu_id);
            bar_state.open_time = input.time;
        }
    }

    if button_interact.hovered && input.mouse_released && bar_state.open_menu.is_some() {
        let time_since_open = input.time - bar_state.open_time;
        if time_since_open < 0.4 {
            // A quick click
            bar_state.open_menu = Some(menu_id);
            bar_state.open_time = input.time;
        } else {
            // A long hold, then release
            bar_state.open_menu = None;
        }
    }

    if button_interact.hovered && bar_state.open_menu.is_some() {
        bar_state.open_menu = Some(menu_id);
    }

    let pressed_escape = input.events.iter().any(|event| {
        matches!(
            event,
            Event::Key {
                key: Key::Escape,
                pressed: true
            }
        )
    });
    if pressed_escape {
        bar_state.open_menu = None;
    }
}
