//! Widgets are pieces of GUI such as [`Label`], [`Button`], [`Slider`] etc.
//!
//! Example widget uses:
//! * `ui.add(Label::new("Text").text_color(color::red));`
//! * `if ui.add(Button::new("Click me")).clicked() { … }`

use crate::*;

mod button;
pub mod color_picker;
pub(crate) mod drag_value;
mod hyperlink;
mod image;
mod label;
pub mod plot;
mod progress_bar;
mod selected_label;
mod separator;
mod slider;
mod spinner;
pub mod text_edit;

pub use button::*;
pub use drag_value::DragValue;
pub use hyperlink::*;
pub use image::Image;
pub use label::*;
pub use progress_bar::ProgressBar;
pub use selected_label::SelectableLabel;
pub use separator::Separator;
pub use slider::*;
pub use spinner::*;
pub use text_edit::{TextBuffer, TextEdit};

// ----------------------------------------------------------------------------

/// Anything implementing Widget can be added to a [`Ui`] with [`Ui::add`].
///
/// [`Button`], [`Label`], [`Slider`], etc all implement the [`Widget`] trait.
///
/// Note that the widgets ([`Button`], [`TextEdit`] etc) are
/// [builders](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html),
/// and not objects that hold state.
///
/// Tip: you can `impl Widget for &mut YourThing { }`.
///
/// `|ui: &mut Ui| -> Response { … }` also implements [`Widget`].
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub trait Widget {
    /// Allocate space, interact, paint, and return a [`Response`].
    ///
    /// Note that this consumes `self`.
    /// This is because most widgets ([`Button`], [`TextEdit`] etc) are
    /// [builders](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html)
    ///
    /// Tip: you can `impl Widget for &mut YourObject { }`.
    fn ui(self, ui: &mut Ui) -> Response;
}

/// This enables functions that return `impl Widget`, so that you can
/// create a widget by just returning a lambda from a function.
///
/// For instance: `ui.add(slider_vec2(&mut vec2));` with:
///
/// ```
/// pub fn slider_vec2(value: &mut egui::Vec2) -> impl egui::Widget + '_ {
///    move |ui: &mut egui::Ui| {
///        ui.horizontal(|ui| {
///            ui.add(egui::Slider::new(&mut value.x, 0.0..=1.0).text("x"));
///            ui.add(egui::Slider::new(&mut value.y, 0.0..=1.0).text("y"));
///        })
///        .response
///    }
/// }
/// ```
impl<F> Widget for F
where
    F: FnOnce(&mut Ui) -> Response,
{
    fn ui(self, ui: &mut Ui) -> Response {
        self(ui)
    }
}

/// Helper so that you can do `TextEdit::State::read…`
pub trait WidgetWithState {
    type State;
}

// ----------------------------------------------------------------------------

/// Show a button to reset a value to its default.
/// The button is only enabled if the value does not already have its original value.
pub fn reset_button<T: Default + PartialEq>(ui: &mut Ui, value: &mut T) {
    reset_button_with(ui, value, T::default());
}

/// Show a button to reset a value to its default.
/// The button is only enabled if the value does not already have its original value.
pub fn reset_button_with<T: PartialEq>(ui: &mut Ui, value: &mut T, reset_value: T) {
    if ui
        .add_enabled(*value != reset_value, Button::new("Reset"))
        .clicked()
    {
        *value = reset_value;
    }
}

// ----------------------------------------------------------------------------

pub fn stroke_ui(ui: &mut crate::Ui, stroke: &mut epaint::Stroke, text: &str) {
    let epaint::Stroke { width, color } = stroke;
    ui.horizontal(|ui| {
        ui.add(DragValue::new(width).speed(0.1).clamp_range(0.0..=5.0))
            .on_hover_text("Width");
        ui.color_edit_button_srgba(color);
        ui.label(text);

        // stroke preview:
        let (_id, stroke_rect) = ui.allocate_space(ui.spacing().interact_size);
        let left = stroke_rect.left_center();
        let right = stroke_rect.right_center();
        ui.painter().line_segment([left, right], (*width, *color));
    });
}

pub(crate) fn shadow_ui(ui: &mut Ui, shadow: &mut epaint::Shadow, text: &str) {
    let epaint::Shadow { extrusion, color } = shadow;
    ui.horizontal(|ui| {
        ui.label(text);
        ui.add(
            DragValue::new(extrusion)
                .speed(1.0)
                .clamp_range(0.0..=100.0),
        )
        .on_hover_text("Extrusion");
        ui.color_edit_button_srgba(color);
    });
}

/// Show a small button to switch to/from dark/light mode (globally).
pub fn global_dark_light_mode_switch(ui: &mut Ui) {
    let style: crate::Style = (*ui.ctx().style()).clone();
    let new_visuals = style.visuals.light_dark_small_toggle_button(ui);
    if let Some(visuals) = new_visuals {
        ui.ctx().set_visuals(visuals);
    }
}

/// Show larger buttons for switching between light and dark mode (globally).
pub fn global_dark_light_mode_buttons(ui: &mut Ui) {
    let mut visuals = ui.ctx().style().visuals.clone();
    visuals.light_dark_radio_buttons(ui);
    ui.ctx().set_visuals(visuals);
}
