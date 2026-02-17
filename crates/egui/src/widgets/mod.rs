//! Widgets are pieces of GUI such as [`Label`], [`Button`], [`Slider`] etc.
//!
//! Example widget uses:
//! * `ui.add(Label::new("Text").text_color(color::red));`
//! * `if ui.add(Button::new("Click me")).clicked() { … }`

use crate::{Response, Ui, epaint};

mod button;
mod checkbox;
pub mod color_picker;
pub(crate) mod drag_value;
mod hyperlink;
mod image;
mod image_button;
mod label;
mod progress_bar;
mod radio_button;
mod selected_label;
mod separator;
mod slider;
mod spinner;
pub mod text_edit;

#[expect(deprecated)]
pub use self::selected_label::SelectableLabel;
#[expect(deprecated, reason = "Deprecated in egui 0.33.0")]
pub use self::{
    button::Button,
    checkbox::Checkbox,
    drag_value::DragValue,
    hyperlink::{Hyperlink, Link},
    image::{
        FrameDurations, Image, ImageFit, ImageOptions, ImageSize, ImageSource,
        decode_animated_image_uri, has_gif_magic_header, has_webp_header, paint_texture_at,
    },
    image_button::ImageButton,
    label::Label,
    progress_bar::ProgressBar,
    radio_button::RadioButton,
    separator::Separator,
    slider::{Slider, SliderClamping, SliderOrientation},
    spinner::Spinner,
    text_edit::{TextBuffer, TextEdit},
};

// ----------------------------------------------------------------------------

/// Anything implementing Widget can be added to a [`Ui`] with [`Ui::add`].
///
/// [`Button`], [`Label`], [`Slider`], etc all implement the [`Widget`] trait.
///
/// You only need to implement `Widget` if you care about being able to do `ui.add(your_widget);`.
///
/// Note that the widgets ([`Button`], [`TextEdit`] etc) are
/// [builders](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html),
/// and not objects that hold state.
///
/// Tip: you can `impl Widget for &mut YourThing { }`.
///
/// `|ui: &mut Ui| -> Response { … }` also implements [`Widget`].
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
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

/// Helper so that you can do e.g. `TextEdit::State::load`.
pub trait WidgetWithState {
    type State;
}

// ----------------------------------------------------------------------------

/// Show a button to reset a value to its default.
/// The button is only enabled if the value does not already have its original value.
///
/// The `text` could be something like "Reset foo".
pub fn reset_button<T: Default + PartialEq>(ui: &mut Ui, value: &mut T, text: &str) {
    reset_button_with(ui, value, text, T::default());
}

/// Show a button to reset a value to its default.
/// The button is only enabled if the value does not already have its original value.
///
/// The `text` could be something like "Reset foo".
pub fn reset_button_with<T: PartialEq>(ui: &mut Ui, value: &mut T, text: &str, reset_value: T) {
    if ui
        .add_enabled(*value != reset_value, Button::new(text))
        .clicked()
    {
        *value = reset_value;
    }
}

// ----------------------------------------------------------------------------

#[deprecated = "Use `ui.add(&mut stroke)` instead"]
pub fn stroke_ui(ui: &mut crate::Ui, stroke: &mut epaint::Stroke, text: &str) {
    ui.horizontal(|ui| {
        ui.label(text);
        ui.add(stroke);
    });
}

/// Show a small button to switch to/from dark/light mode (globally).
pub fn global_theme_preference_switch(ui: &mut Ui) {
    if let Some(new_theme) = ui.ctx().theme().small_toggle_button(ui) {
        ui.ctx().set_theme(new_theme);
    }
}

/// Show larger buttons for switching between light and dark mode (globally).
pub fn global_theme_preference_buttons(ui: &mut Ui) {
    let mut theme_preference = ui.options(|opt| opt.theme_preference);
    theme_preference.radio_buttons(ui);
    ui.ctx().set_theme(theme_preference);
}

/// Show a small button to switch to/from dark/light mode (globally).
#[deprecated = "Use global_theme_preference_switch instead"]
pub fn global_dark_light_mode_switch(ui: &mut Ui) {
    global_theme_preference_switch(ui);
}

/// Show larger buttons for switching between light and dark mode (globally).
#[deprecated = "Use global_theme_preference_buttons instead"]
pub fn global_dark_light_mode_buttons(ui: &mut Ui) {
    global_theme_preference_buttons(ui);
}
