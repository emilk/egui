//! Menu bar functionality.
//!
//! Usage:
//! ``` rust
//! fn show_menu(ui: &mut egui::Ui) {
//!     use egui::{menu, Button};
//!
//!     menu::bar(ui, |ui| {
//!         menu::menu(ui, "File", |ui| {
//!             if ui.add(Button::new("Open")).clicked {
//!                 // ...
//!             }
//!         });
//!     });
//! }
//! ```

use crate::{widgets::*, *};

/// What is saved between frames.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct BarState {
    #[cfg_attr(feature = "serde", serde(skip))]
    open_menu: Option<Id>,
    #[cfg_attr(feature = "serde", serde(skip))]
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

impl BarState {
    fn load(ctx: &Context, bar_id: &Id) -> Self {
        ctx.memory()
            .menu_bar
            .get(bar_id)
            .cloned()
            .unwrap_or_default()
    }

    fn save(self, ctx: &Context, bar_id: Id) {
        ctx.memory().menu_bar.insert(bar_id, self);
    }

    fn close_menus(ctx: &Context, bar_id: Id) {
        let mut bar_state = BarState::load(ctx, &bar_id);
        bar_state.open_menu = None;
        bar_state.save(ctx, bar_id);
    }
}

pub fn bar<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Rect) {
    ui.inner_layout(Layout::horizontal(Align::Center), |ui| {
        Frame::menu_bar(ui.style()).show(ui, |ui| {
            let mut style = ui.style().clone();
            style.button_padding = vec2(2.0, 0.0);
            // style.interact.active.bg_fill = None;
            style.interact.active.rect_outline = None;
            // style.interact.hovered.bg_fill = None;
            style.interact.hovered.rect_outline = None;
            style.interact.inactive.bg_fill = None;
            style.interact.inactive.rect_outline = None;
            ui.set_style(style);

            // Take full width and fixed height:
            let height = ui.style().menu_bar.height;
            ui.set_desired_height(height);
            ui.expand_to_size(vec2(ui.available().width(), height));

            let ret = add_contents(ui);

            let clicked_outside = !ui.hovered(ui.rect()) && ui.input().mouse.released;
            if clicked_outside || ui.input().key_pressed(Key::Escape) {
                // TODO: this prevent sub-menus in menus. We should fix that.
                let bar_id = ui.id();
                BarState::close_menus(ui.ctx(), bar_id);
            }

            ret
        })
    })
}

/// Construct a top level menu in a menu bar. This would be e.g. "File", "Edit" etc.
pub fn menu(ui: &mut Ui, title: impl Into<String>, add_contents: impl FnOnce(&mut Ui)) {
    menu_impl(ui, title, Box::new(add_contents))
}

fn menu_impl<'c>(
    ui: &mut Ui,
    title: impl Into<String>,
    add_contents: Box<dyn FnOnce(&mut Ui) + 'c>,
) {
    let title = title.into();
    let bar_id = ui.id();
    let menu_id = bar_id.with(&title);

    let mut bar_state = BarState::load(ui.ctx(), &bar_id);

    let mut button = Button::new(title);

    if bar_state.open_menu == Some(menu_id) {
        button = button.fill(Some(ui.style().interact.active.fill));
    }

    let button_interact = ui.add(button);

    interact_with_menu_button(&mut bar_state, ui.input(), menu_id, &button_interact);

    if bar_state.open_menu == Some(menu_id) {
        let area = Area::new(menu_id)
            .order(Order::Foreground)
            .fixed_pos(button_interact.rect.left_bottom());
        let frame = Frame::menu(ui.style());

        let resize = Resize::default().auto_sized().outline(false);

        let menu_interact = area.show(ui.ctx(), |ui| {
            frame.show(ui, |ui| {
                resize.show(ui, |ui| {
                    let mut style = ui.style().clone();
                    style.button_padding = vec2(2.0, 0.0);
                    // style.interact.active.bg_fill = None;
                    style.interact.active.rect_outline = None;
                    // style.interact.hovered.bg_fill = None;
                    style.interact.hovered.rect_outline = None;
                    style.interact.inactive.bg_fill = None;
                    style.interact.inactive.rect_outline = None;
                    ui.set_style(style);
                    ui.set_layout(Layout::justified(Direction::Vertical));
                    add_contents(ui)
                })
            })
        });

        if menu_interact.hovered && ui.input().mouse.released {
            bar_state.open_menu = None;
        }
    }

    bar_state.save(ui.ctx(), bar_id);
}

fn interact_with_menu_button(
    bar_state: &mut BarState,
    input: &InputState,
    menu_id: Id,
    button_interact: &GuiResponse,
) {
    if button_interact.hovered && input.mouse.pressed {
        if bar_state.open_menu.is_some() {
            bar_state.open_menu = None;
        } else {
            bar_state.open_menu = Some(menu_id);
            bar_state.open_time = input.time;
        }
    }

    if button_interact.hovered && input.mouse.released && bar_state.open_menu.is_some() {
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
}
