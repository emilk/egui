//! Keyboard, popup and read-only behavior of the editable `DatePickerButton`.

#![cfg(feature = "datepicker")]

use egui::{Key, Modifiers, Order, Rect, accesskit::Role};
use egui_extras::DatePickerButton;
use egui_kittest::{Harness, kittest::Queryable as _};
use jiff::civil::Date;

type Configure = for<'a> fn(DatePickerButton<'a>) -> DatePickerButton<'a>;

/// A date picker (starting at 2026-05-15) in a headless UI.
struct Picker {
    harness: Harness<'static, Date>,
}

impl Picker {
    fn new() -> Self {
        Self::with(|picker| picker)
    }

    fn with(configure: Configure) -> Self {
        let mut harness = Harness::builder()
            .with_accessibility_check(false)
            .with_size(egui::vec2(500.0, 500.0))
            .build_ui_state(
                move |ui, date: &mut Date| {
                    ui.add(configure(DatePickerButton::new(date)));
                },
                Date::constant(2026, 5, 15),
            );
        harness.run();
        Self { harness }
    }

    fn focus_text(&mut self) {
        self.harness.get_by_role(Role::TextInput).focus();
        self.harness.run();
    }

    fn press(&mut self, key: Key) {
        self.harness.key_press(key);
        self.harness.run();
        self.harness.run();
    }

    fn open_with_keyboard(&mut self) {
        self.focus_text();
        self.harness
            .key_combination_modifiers(Modifiers::ALT, &[Key::ArrowDown]);
        self.harness.run();
    }

    /// Replaces the text of the field with `text` (without confirming it).
    fn type_text(&mut self, text: &str) {
        self.focus_text();
        self.harness
            .key_combination_modifiers(Modifiers::COMMAND, &[Key::A]);
        self.harness.run();
        self.harness.get_by_role(Role::TextInput).type_text(text);
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

    /// The rect of the popup, if it is open.
    fn popup_rect(&self) -> Option<Rect> {
        self.harness.ctx.memory(|memory| {
            memory
                .areas()
                .visible_layer_ids()
                .into_iter()
                .filter(|layer| layer.order == Order::Foreground)
                .find_map(|layer| memory.area_rect(layer.id))
        })
    }

    fn is_open(&self) -> bool {
        self.popup_rect().is_some()
    }

    /// Clicks inside the first rows of the month grid of the open popup.
    fn click_a_day(&mut self) {
        let rect = self.popup_rect().expect("the popup should be open");
        let position = egui::pos2(rect.left() + 100.0, rect.top() + 100.0);
        self.harness.hover_at(position);
        self.harness.run();
        self.harness.drag_at(position);
        self.harness.drop_at(position);
        self.harness.run();
        self.harness.run();
    }

    fn click_outside(&mut self) {
        let position = egui::pos2(490.0, 490.0);
        self.harness.hover_at(position);
        self.harness.run();
        self.harness.drag_at(position);
        self.harness.drop_at(position);
        self.harness.run();
        self.harness.run();
    }
}

#[test]
fn up_and_down_change_the_part_under_the_cursor() {
    let mut picker = Picker::new();
    picker.focus_text();

    // At the end of the text the cursor is on the day.
    picker.press(Key::End);
    picker.press(Key::ArrowUp);
    assert_eq!(picker.date(), "2026-05-16", "up on the day");
    picker.press(Key::ArrowDown);
    picker.press(Key::ArrowDown);
    assert_eq!(picker.date(), "2026-05-14", "down on the day");

    // At the start it is on the year.
    picker.press(Key::Home);
    picker.press(Key::ArrowUp);
    assert_eq!(picker.date(), "2027-05-14", "up on the year");
    assert_eq!(picker.text(), "2027-05-14", "the text follows the date");
}

#[test]
fn up_and_down_roll_over_the_month_end() {
    let mut picker = Picker::new();
    picker.focus_text();
    picker.press(Key::End);
    for _ in 0..17 {
        picker.press(Key::ArrowUp);
    }
    assert_eq!(picker.date(), "2026-06-01", "May 15 + 17 days");
}

#[test]
fn alt_down_opens_the_popup() {
    let mut picker = Picker::new();
    assert!(!picker.is_open(), "closed at the start");
    picker.open_with_keyboard();
    assert!(picker.is_open(), "Alt+Down opens the popup");
}

#[test]
fn enter_applies_the_text_and_closes_the_popup() {
    let mut picker = Picker::new();
    picker.open_with_keyboard();
    picker.type_text("20260620");
    picker.press(Key::Enter);
    assert_eq!(picker.date(), "2026-06-20", "Enter applies the typed date");
    assert!(!picker.is_open(), "Enter closes the popup");
}

#[test]
fn escape_closes_the_popup_and_drops_what_was_typed() {
    let mut picker = Picker::new();
    picker.open_with_keyboard();
    picker.type_text("20260620");
    picker.press(Key::Escape);
    assert_eq!(picker.date(), "2026-05-15", "Esc keeps the date");
    assert_eq!(picker.text(), "2026-05-15", "Esc restores the text");
    assert!(!picker.is_open(), "Esc closes the popup");
}

#[test]
fn tab_applies_the_typed_date() {
    let mut picker = Picker::new();
    picker.type_text("20260620");
    picker.press(Key::Tab);
    assert_eq!(picker.date(), "2026-06-20", "Tab applies the typed date");
    assert_eq!(picker.text(), "2026-06-20");
}

#[test]
fn clicking_a_day_applies_it_and_closes_the_popup() {
    let mut picker = Picker::new();
    picker.open_with_keyboard();
    picker.click_a_day();
    assert!(!picker.is_open(), "picking a day closes the popup");
    let date = picker.harness.state();
    assert_eq!((date.year(), date.month()), (2026, 5), "same month");
    assert_ne!(picker.date(), "2026-05-15", "a different day was picked");
    assert_eq!(picker.text(), picker.date(), "the text shows the new date");
}

#[test]
fn the_month_buttons_move_the_calendar() {
    let mut picker = Picker::new();
    picker.open_with_keyboard();
    picker.harness.get_by_label("next month").click();
    picker.harness.run();
    assert_eq!(picker.date(), "2026-05-15", "moving the month doesn't pick");
    picker.click_a_day();
    let date = picker.harness.state();
    assert_eq!((date.year(), date.month()), (2026, 6), "next month");

    let mut picker = Picker::new();
    picker.open_with_keyboard();
    picker.harness.get_by_label("previous month").click();
    picker.harness.run();
    picker.click_a_day();
    let date = picker.harness.state();
    assert_eq!((date.year(), date.month()), (2026, 4), "previous month");
}

#[test]
fn clicking_outside_closes_the_popup_without_changes() {
    let mut picker = Picker::new();
    picker.open_with_keyboard();
    picker.click_outside();
    assert!(!picker.is_open(), "a click outside closes the popup");
    assert_eq!(picker.date(), "2026-05-15");
}

#[test]
fn the_popup_keeps_its_size_while_hovered() {
    let mut picker = Picker::new();
    picker.open_with_keyboard();
    let rect = picker.popup_rect().expect("open");
    for (x, y) in [(20.0, 20.0), (100.0, 100.0), (200.0, 150.0), (300.0, 200.0)] {
        picker
            .harness
            .hover_at(egui::pos2(rect.left() + x, rect.top() + y));
        picker.harness.run();
        picker.harness.run();
        assert_eq!(
            picker.popup_rect(),
            Some(rect),
            "the popup moved or resized when hovering at ({x}, {y})"
        );
    }
}

#[test]
fn a_typed_date_is_applied_with_another_format() {
    let mut picker = Picker::with(|picker| picker.format("%d/%m/%Y"));
    picker.type_text("25122026");
    assert_eq!(picker.text(), "25/12/2026", "separators are inserted");
    picker.press(Key::Enter);
    assert_eq!(picker.date(), "2026-12-25");
    assert_eq!(picker.text(), "25/12/2026");
}

#[test]
fn a_format_without_mask_still_has_to_parse() {
    let mut picker = Picker::with(|picker| picker.format("%b %d, %Y"));
    assert_eq!(picker.text(), "May 15, 2026");

    picker.type_text("not a date");
    picker.press(Key::Enter);
    assert_eq!(picker.date(), "2026-05-15", "garbage is rejected");
    assert_eq!(picker.text(), "May 15, 2026");

    picker.type_text("Dec 25, 2026");
    picker.press(Key::Enter);
    assert_eq!(picker.date(), "2026-12-25", "a valid date is applied");
}

#[test]
fn without_icon_there_is_no_calendar_button() {
    let picker = Picker::with(|picker| picker.show_icon(false));
    assert!(
        picker.harness.query_by_role(Role::Button).is_none(),
        "no button when the icon is hidden"
    );
    assert!(picker.harness.query_by_role(Role::TextInput).is_some());
}

#[test]
fn a_read_only_picker_cannot_change_the_date() {
    let mut picker = Picker::with(|picker| picker.editable(false));

    picker.focus_text();
    picker.press(Key::End);
    picker.press(Key::ArrowUp);
    assert_eq!(picker.date(), "2026-05-15", "Up doesn't change the date");

    picker.harness.get_by_role(Role::TextInput).type_text("9");
    picker.harness.run();
    picker.press(Key::Enter);
    assert_eq!(
        picker.date(),
        "2026-05-15",
        "typing doesn't change the date"
    );
    assert_eq!(picker.text(), "2026-05-15");

    picker
        .harness
        .key_combination_modifiers(Modifiers::ALT, &[Key::ArrowDown]);
    picker.harness.run();
    assert!(!picker.is_open(), "Alt+Down doesn't open the popup");

    picker.harness.get_by_role(Role::Button).click();
    picker.harness.run();
    assert!(!picker.is_open(), "the calendar button is disabled");
}
