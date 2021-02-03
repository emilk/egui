//! Menu bar functionality (very basic so far).
//!
//! Usage:
//! ```
//! fn show_menu(ui: &mut egui::Ui) {
//!     use egui::{menu, Button};
//!
//!     menu::bar(ui, |ui| {
//!         menu::menu(ui, "File", |ui| {
//!             if ui.button("Open").clicked() {
//!                 // ...
//!             }
//!         });
//!     });
//! }
//! ```

use crate::{paint::Stroke, widgets::*, *};

/// What is saved between frames.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BarState {
    open_menu: Option<Id>,
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
}

/// The menu bar goes well in `TopPanel`,
/// but can also be placed in a `Window`.
/// In the latter case you may want to wrap it in `Frame`.
pub fn bar<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> (R, Response) {
    ui.horizontal(|ui| {
        let mut style = (**ui.style()).clone();
        style.spacing.button_padding = vec2(2.0, 0.0);
        // style.visuals.widgets.active.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.active.bg_stroke = Stroke::none();
        // style.visuals.widgets.hovered.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.hovered.bg_stroke = Stroke::none();
        style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = Stroke::none();
        ui.set_style(style);

        // Take full width and fixed height:
        let height = ui.spacing().interact_size.y;
        ui.set_min_size(vec2(ui.available_width(), height));

        add_contents(ui)
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
        button = button.fill(Some(ui.visuals().selection.bg_fill));
    }

    let button_response = ui.add(button);
    if button_response.clicked() {
        // Toggle
        if bar_state.open_menu == Some(menu_id) {
            bar_state.open_menu = None;
        } else {
            bar_state.open_menu = Some(menu_id);
        }
    } else if button_response.hovered() && bar_state.open_menu.is_some() {
        bar_state.open_menu = Some(menu_id);
    }

    if bar_state.open_menu == Some(menu_id) || ui.ctx().memory().everything_is_visible() {
        let area = Area::new(menu_id)
            .order(Order::Foreground)
            .fixed_pos(button_response.rect.left_bottom());
        let frame = Frame::menu(ui.style());

        area.show(ui.ctx(), |ui| {
            frame.show(ui, |ui| {
                let mut style = (**ui.style()).clone();
                style.spacing.button_padding = vec2(2.0, 0.0);
                // style.visuals.widgets.active.bg_fill = Color32::TRANSPARENT;
                style.visuals.widgets.active.bg_stroke = Stroke::none();
                // style.visuals.widgets.hovered.bg_fill = Color32::TRANSPARENT;
                style.visuals.widgets.hovered.bg_stroke = Stroke::none();
                style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
                style.visuals.widgets.inactive.bg_stroke = Stroke::none();
                ui.set_style(style);
                ui.with_layout(Layout::top_down_justified(Align::left()), add_contents);
            })
        });

        // TODO: this prevents sub-menus in menus. We should fix that.
        if ui.input().key_pressed(Key::Escape)
            || ui.input().pointer.any_click() && !button_response.clicked()
        {
            bar_state.open_menu = None;
        }
    }

    bar_state.save(ui.ctx(), bar_id);
}
