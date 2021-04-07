//! Widgets are pieces of GUI such as [`Label`], [`Button`], [`Slider`] etc.
//!
//! Example widget uses:
//! * `ui.add(Label::new("Text").text_color(color::red));`
//! * `if ui.add(Button::new("Click me")).clicked() { ... }`

use crate::*;

mod button;
pub mod color_picker;
pub(crate) mod drag_value;
mod hyperlink;
mod image;
mod label;
pub mod plot;
mod selected_label;
mod separator;
mod slider;
pub(crate) mod text_edit;

pub use hyperlink::*;
pub use label::*;
pub use selected_label::*;
pub use separator::*;
pub use {button::*, drag_value::DragValue, image::Image, slider::*, text_edit::*};

// ----------------------------------------------------------------------------

/// Anything implementing Widget can be added to a [`Ui`] with [`Ui::add`].
///
/// Examples include `[Button]`, `[Label]` and [`Slider`].
///
/// `|ui: &mut Ui| -> Response { … }` also implemented `Widget`.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub trait Widget {
    /// Allocate space, interact, paint, and return a [`Response`].
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

// ----------------------------------------------------------------------------

/// Show a button to reset a value to its default.
/// The button is only enabled if the value does not already have its original value.
pub fn reset_button<T: Default + PartialEq>(ui: &mut Ui, value: &mut T) {
    let def = T::default();
    if ui
        .add(Button::new("Reset").enabled(*value != def))
        .clicked()
    {
        *value = def;
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
