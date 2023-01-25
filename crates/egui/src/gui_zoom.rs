//! Helpers for zooming the whole GUI of an app (changing [`Context::pixels_per_point`].
//!
use crate::*;

/// The suggested keyboard shortcuts for global gui zooming.
pub mod kb_shortcuts {
    use super::*;

    pub const ZOOM_IN: KeyboardShortcut =
        KeyboardShortcut::new(Modifiers::COMMAND, Key::PlusEquals);
    pub const ZOOM_OUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Minus);
    pub const ZOOM_RESET: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::Num0);
}

/// Let the user scale the GUI (change `Context::pixels_per_point`) by pressing
/// Cmd+Plus, Cmd+Minus or Cmd+0, just like in a browser.
///
/// When using [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe), you want to call this as:
/// ```ignore
/// // On web, the browser controls the gui zoom.
/// if !frame.is_web() {
///     egui::gui_zoom::zoom_with_keyboard_shortcuts(
///         ctx,
///         frame.info().native_pixels_per_point,
///     );
/// }
/// ```
pub fn zoom_with_keyboard_shortcuts(ctx: &Context, native_pixels_per_point: Option<f32>) {
    if ctx.input_mut(|i| i.consume_shortcut(&kb_shortcuts::ZOOM_RESET)) {
        if let Some(native_pixels_per_point) = native_pixels_per_point {
            ctx.set_pixels_per_point(native_pixels_per_point);
        }
    } else {
        if ctx.input_mut(|i| i.consume_shortcut(&kb_shortcuts::ZOOM_IN)) {
            zoom_in(ctx);
        }
        if ctx.input_mut(|i| i.consume_shortcut(&kb_shortcuts::ZOOM_OUT)) {
            zoom_out(ctx);
        }
    }
}

const MIN_PIXELS_PER_POINT: f32 = 0.2;
const MAX_PIXELS_PER_POINT: f32 = 4.0;

/// Make everything larger.
pub fn zoom_in(ctx: &Context) {
    let mut pixels_per_point = ctx.pixels_per_point();
    pixels_per_point += 0.1;
    pixels_per_point = pixels_per_point.clamp(MIN_PIXELS_PER_POINT, MAX_PIXELS_PER_POINT);
    pixels_per_point = (pixels_per_point * 10.).round() / 10.;
    ctx.set_pixels_per_point(pixels_per_point);
}

/// Make everything smaller.
pub fn zoom_out(ctx: &Context) {
    let mut pixels_per_point = ctx.pixels_per_point();
    pixels_per_point -= 0.1;
    pixels_per_point = pixels_per_point.clamp(MIN_PIXELS_PER_POINT, MAX_PIXELS_PER_POINT);
    pixels_per_point = (pixels_per_point * 10.).round() / 10.;
    ctx.set_pixels_per_point(pixels_per_point);
}

/// Show buttons for zooming the ui.
///
/// This is meant to be called from within a menu (See [`Ui::menu_button`]).
///
/// When using [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe), you want to call this as:
/// ```ignore
/// // On web, the browser controls the gui zoom.
/// if !frame.is_web() {
///     ui.menu_button("View", |ui| {
///         egui::gui_zoom::zoom_menu_buttons(
///             ui,
///             frame.info().native_pixels_per_point,
///         );
///     });
/// }
/// ```
pub fn zoom_menu_buttons(ui: &mut Ui, native_pixels_per_point: Option<f32>) {
    if ui
        .add_enabled(
            ui.ctx().pixels_per_point() < MAX_PIXELS_PER_POINT,
            Button::new("Zoom In").shortcut_text(ui.ctx().format_shortcut(&kb_shortcuts::ZOOM_IN)),
        )
        .clicked()
    {
        zoom_in(ui.ctx());
        ui.close_menu();
    }

    if ui
        .add_enabled(
            ui.ctx().pixels_per_point() > MIN_PIXELS_PER_POINT,
            Button::new("Zoom Out")
                .shortcut_text(ui.ctx().format_shortcut(&kb_shortcuts::ZOOM_OUT)),
        )
        .clicked()
    {
        zoom_out(ui.ctx());
        ui.close_menu();
    }

    if let Some(native_pixels_per_point) = native_pixels_per_point {
        if ui
            .add_enabled(
                ui.ctx().pixels_per_point() != native_pixels_per_point,
                Button::new("Reset Zoom")
                    .shortcut_text(ui.ctx().format_shortcut(&kb_shortcuts::ZOOM_RESET)),
            )
            .clicked()
        {
            ui.ctx().set_pixels_per_point(native_pixels_per_point);
            ui.close_menu();
        }
    }
}
