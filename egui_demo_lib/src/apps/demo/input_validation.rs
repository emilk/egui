use super::*;
use egui::{
    widgets::{
        text_edit::{Action, InputData, ValidateInput},
        TextEdit,
    },
    *,
};

/// Showcase text input validation.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[derive(Default)]
pub struct InputValidationWindow {
    u8_buffer: String,
    u16_buffer: String,
    u32_buffer: String,
    u64_buffer: String,
    i8_buffer: String,
    i16_buffer: String,
    i32_buffer: String,
    i64_buffer: String,
    a_buffer: String,
}

// Clippy being annoying since it does not consider whether a function
// is used as a function pointer.
// https://github.com/rust-lang/rust-clippy/issues/2434
#[allow(clippy::needless_pass_by_value)]
fn custom_validator(data: InputData<'_>) -> Action {
    Action::Insert(data.input.as_str().replace(|c: char| c != 'a', ""))
}

impl Demo for InputValidationWindow {
    fn name(&self) -> &'static str {
        "🔤 Input Validation"
    }

    fn show(&mut self, ctx: &Context, open: &mut bool) {
        Window::new(self.name())
            .open(open)
            .vscroll(true)
            .hscroll(true)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl View for InputValidationWindow {
    fn ui(&mut self, ui: &mut Ui) {
        ui.set_min_width(250.0);

        ui.label("Use a `TextEdit` validator to do the input validation for you!");

        ui.horizontal(|ui| {
            ui.label("u8:");
            TextEdit::singleline(&mut self.u8_buffer)
                .validator(&mut u8::validate_input)
                .hint_text("a valid `u8`")
                .show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("u16:");
            TextEdit::singleline(&mut self.u16_buffer)
                .validator(&mut u16::validate_input)
                .hint_text("a valid `u16`")
                .show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("u32:");
            TextEdit::singleline(&mut self.u32_buffer)
                .validator(&mut u32::validate_input)
                .hint_text("a valid `u32`")
                .show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("u64:");
            TextEdit::singleline(&mut self.u64_buffer)
                .validator(&mut u64::validate_input)
                .hint_text("a valid `u64`")
                .show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("i8:");
            TextEdit::singleline(&mut self.i8_buffer)
                .validator(&mut i8::validate_input)
                .hint_text("a valid `i8`")
                .show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("i16:");
            TextEdit::singleline(&mut self.i16_buffer)
                .validator(&mut i16::validate_input)
                .hint_text("a valid `i16`")
                .show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("i32:");
            TextEdit::singleline(&mut self.i32_buffer)
                .validator(&mut i32::validate_input)
                .hint_text("a valid `i32`")
                .show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("i64:");
            TextEdit::singleline(&mut self.i64_buffer)
                .validator(&mut i64::validate_input)
                .hint_text("a valid `i64`")
                .show(ui);
        });

        ui.horizontal(|ui| {
            ui.label("custom validator:");
            TextEdit::singleline(&mut self.a_buffer)
                .validator(&mut custom_validator)
                .hint_text("can only be filled with `a` characters")
                .show(ui);
        });
    }
}
