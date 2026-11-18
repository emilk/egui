//! Typing something that isn't a valid date must never change the date.

#![cfg(feature = "datepicker")]

use egui::{Key, Modifiers, accesskit::Role};
use egui_extras::DatePickerButton;
use egui_kittest::{Harness, kittest::Queryable as _};
use jiff::civil::Date;

const ORIGINAL: &str = "2026-05-15";

/// A date picker in a headless UI, with helpers to type into its text field.
struct Picker {
    harness: Harness<'static, Date>,
}

impl Picker {
    fn new() -> Self {
        let mut harness = Harness::builder()
            .with_accessibility_check(false)
            .with_size(egui::vec2(400.0, 300.0))
            .build_ui_state(
                |ui, date: &mut Date| {
                    ui.add(DatePickerButton::new(date));
                },
                Date::constant(2026, 5, 15),
            );
        harness.run();
        Self { harness }
    }

    /// Replaces the text of the field with `text` and presses Enter.
    fn type_and_confirm(&mut self, text: &str) {
        self.harness.get_by_role(Role::TextInput).focus();
        self.harness.run();
        self.harness
            .key_combination_modifiers(Modifiers::COMMAND, &[Key::A]);
        self.harness.run();
        self.harness.get_by_role(Role::TextInput).type_text(text);
        self.harness.run();
        self.harness.key_press(Key::Enter);
        self.harness.run();
        self.harness.run();
    }

    fn text(&self) -> String {
        self.harness
            .get_by_role(Role::TextInput)
            .value()
            .unwrap_or_default()
    }

    fn date(&self) -> String {
        self.harness.state().to_string()
    }

    /// The date and the text must both still be the original one.
    fn assert_unchanged(&self, typed: &str) {
        assert_eq!(
            self.date(),
            ORIGINAL,
            "typing {typed:?} must not change the date"
        );
        assert_eq!(
            self.text(),
            ORIGINAL,
            "typing {typed:?} must put the last valid date back in the field"
        );
    }
}

#[test]
fn day_that_does_not_exist_is_rejected() {
    for typed in ["20260230", "20260431", "20250229", "20260500", "20260532"] {
        let mut picker = Picker::new();
        picker.type_and_confirm(typed);
        picker.assert_unchanged(typed);
    }
}

#[test]
fn month_out_of_range_is_rejected() {
    for typed in ["20261301", "20260001", "20269901"] {
        let mut picker = Picker::new();
        picker.type_and_confirm(typed);
        picker.assert_unchanged(typed);
    }
}

#[test]
fn incomplete_date_is_rejected() {
    for typed in ["2026", "202605", "2026051", "5"] {
        let mut picker = Picker::new();
        picker.type_and_confirm(typed);
        picker.assert_unchanged(typed);
    }
}

#[test]
fn empty_text_is_rejected() {
    let mut picker = Picker::new();
    picker.harness.get_by_role(Role::TextInput).focus();
    picker.harness.run();
    picker
        .harness
        .key_combination_modifiers(Modifiers::COMMAND, &[Key::A]);
    picker.harness.key_press(Key::Backspace);
    picker.harness.run();
    picker.harness.key_press(Key::Enter);
    picker.harness.run();
    picker.harness.run();
    picker.assert_unchanged("");
}

#[test]
fn letters_and_symbols_are_ignored() {
    let mut picker = Picker::new();
    picker.type_and_confirm("abc!?");
    picker.assert_unchanged("abc!?");
}

#[test]
fn valid_date_is_applied() {
    // Control: the same flow with a real date does change it (and leap days work).
    let mut picker = Picker::new();
    picker.type_and_confirm("20260620");
    assert_eq!(picker.date(), "2026-06-20");
    assert_eq!(picker.text(), "2026-06-20");

    let mut picker = Picker::new();
    picker.type_and_confirm("20240229");
    assert_eq!(picker.date(), "2024-02-29");
}
