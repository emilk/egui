#![cfg(feature = "datepicker")]
//! fork-only — uses `Ui::language()`, not upstreamable.
//!
//! Checks the `DatePickerButton` locale-resolution chain: an explicit
//! `.locale(...)` wins over the UI language, an Arabic context language
//! (`Context::set_language`, as in `crates/egui/tests/layout_language.rs`)
//! renders the popup in Arabic, and no language means the English default.

use egui::vec2;
use egui_extras::{DateLocale, DatePickerButton};
use egui_kittest::kittest::{By, Queryable as _};
use egui_kittest::Harness;
use jiff::civil::Date;

/// A fixed selection so button labels are deterministic.
const SELECTION: Date = Date::constant(2026, 11, 4);

/// The date shown on the button, in the default `%Y-%m-%d` format.
const BUTTON_DATE: &str = "2026-11-04";

fn harness_with(language: Option<String>, locale: Option<DateLocale>) -> Harness<'static, Date> {
    let mut harness = Harness::builder()
        .with_size(vec2(600.0, 400.0))
        // These tests assert the accesskit tree, not pixels; the default test
        // fonts have no emoji or Arabic glyphs.
        .allow_missing_glyphs()
        // The date picker's combo boxes predate accessible names upstream;
        // not our scope here.
        .with_accessibility_check(false)
        .with_context_setup(move |ctx| {
            if let Some(language) = language {
                ctx.set_language(language);
            }
        })
        .build_ui_state(
            move |ui, selection| {
                let button = DatePickerButton::new(selection);
                let button = if let Some(locale) = locale.clone() {
                    button.locale(locale)
                } else {
                    button
                };
                ui.add(button);
            },
            SELECTION,
        );
    harness.run();
    harness
}

fn open_popup(harness: &mut Harness<'static, Date>) {
    harness.get_by_label_contains(BUTTON_DATE).click();
    harness.run();
}

#[test]
fn kittest_arabic_language_renders_arabic_names() {
    let mut harness = harness_with(Some("ar".to_owned()), None);
    open_popup(&mut harness);

    // The selected month text is stored as the ComboBox's accessible value.
    assert!(
        harness.query(By::new().value("نوفمبر")).is_some(),
        "Arabic month name not found"
    );
    assert!(
        harness.query_by_label("الأسبوع").is_some(),
        "Arabic week label not found"
    );
    assert!(
        harness.query_by_label("السبت").is_some(),
        "Arabic weekday name not found"
    );
    assert!(
        harness.query_by_label("Mo").is_none(),
        "English weekday leaked into an Arabic popup"
    );
}

#[test]
fn kittest_explicit_locale_wins_over_the_language() {
    let mut harness = harness_with(Some("ar".to_owned()), Some(DateLocale::default()));
    open_popup(&mut harness);

    for name in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
        assert!(
            harness.query_by_label(name).is_some(),
            "explicit locale should keep the {name:?} header"
        );
    }
    assert!(
        harness.query(By::new().value("November")).is_some(),
        "explicit locale should keep the English month name"
    );
    assert!(
        harness.query(By::new().value("نوفمبر")).is_none(),
        "explicit locale should not show the Arabic month name"
    );
}

#[test]
fn kittest_no_language_defaults_to_english() {
    let mut harness = harness_with(None, None);
    open_popup(&mut harness);

    for name in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
        assert!(
            harness.query_by_label(name).is_some(),
            "English header cell {name:?} not found"
        );
    }
    assert!(
        harness.query(By::new().value("November")).is_some(),
        "English month name not found"
    );
}
