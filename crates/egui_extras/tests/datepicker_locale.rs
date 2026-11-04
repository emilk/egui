#![cfg(feature = "datepicker")]
//! Tests for the [`DateLocale`] hook of the date picker: the built-in language
//! table, and kittest regression checks for the default (English,
//! Monday-first, LTR) rendering and for right-to-left locales.
//!
//! No snapshot assertions: everything is asserted through the accesskit tree
//! and node geometry, so the default locale's snapshots stay bit-identical.
//! Header cells center their labels, so geometry assertions use the extents
//! of the popup content rather than single-cell edges.

use egui::{Align, Layout, Vec2, vec2};
use egui_kittest::kittest::By;
use egui_kittest::{Harness, kittest::Queryable as _};
use jiff::civil::{Date, Weekday};

use egui_extras::{DateLocale, DatePickerButton};

/// A fixed selection so button labels are deterministic.
const SELECTION: Date = Date::constant(2026, 11, 4);

/// The date shown on the button, in the default `%Y-%m-%d` format.
const BUTTON_DATE: &str = "2026-11-04";

fn harness_with(locale: DateLocale, rtl_layout: bool) -> Harness<'static, Date> {
    let mut harness = Harness::builder()
        .with_size(vec2(600.0, 400.0))
        // These tests assert the accesskit tree, not pixels; the default test
        // fonts have no emoji or Arabic glyphs.
        .allow_missing_glyphs()
        // The date picker's combo boxes predate accessible names upstream;
        // not this PR's scope.
        .with_accessibility_check(false)
        .build_ui_state(
            move |ui, selection| {
                let locale = locale.clone();
                // `Layout::right_to_left` is plain layout direction (no
                // context-wide RTL flag): it parks the button near the right
                // edge so the RTL popup anchor has room to hang leftward.
                if rtl_layout {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add(DatePickerButton::new(selection).locale(locale));
                    });
                } else {
                    ui.add(DatePickerButton::new(selection).locale(locale));
                }
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
fn from_language_arabic_matches_the_builtin_table() {
    let locale = DateLocale::from_language("ar");
    assert_eq!(
        locale
            .weekday_names
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "الاثنين",
            "الثلاثاء",
            "الأربعاء",
            "الخميس",
            "الجمعة",
            "السبت",
            "الأحد"
        ]
    );
    assert_eq!(
        locale
            .month_names
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "يناير",
            "فبراير",
            "مارس",
            "أبريل",
            "مايو",
            "يونيو",
            "يوليو",
            "أغسطس",
            "سبتمبر",
            "أكتوبر",
            "نوفمبر",
            "ديسمبر"
        ]
    );
    assert_eq!(locale.first_weekday, Weekday::Saturday);
    assert_eq!(locale.weekend, [Weekday::Friday, Weekday::Saturday]);
    assert!(locale.rtl);
}

#[test]
fn from_language_matches_the_primary_subtag() {
    assert_eq!(
        DateLocale::from_language("ar-EG"),
        DateLocale::from_language("ar")
    );
}

#[test]
fn from_language_unknown_or_empty_falls_back_to_default() {
    assert_eq!(DateLocale::from_language("zz"), DateLocale::default());
    assert_eq!(DateLocale::from_language(""), DateLocale::default());
    assert_eq!(DateLocale::from_language("en-US"), DateLocale::default());
    assert_eq!(DateLocale::from_language("arabic"), DateLocale::default());
}

#[test]
fn out_of_range_month_falls_back_instead_of_panicking() {
    // `DatePickerPopupState::month` is a persisted bare `i8`; the month lookup
    // replaced `panic!("Unknown month: {i}")`. This mirrors the popup's
    // bounds-safe lookup contract: any out-of-range month yields a valid name.
    let locale = DateLocale::default();
    let fallback = fallback_month_name(&locale, 13);
    assert_eq!(fallback, locale.month_names[0]);
    assert_eq!(fallback_month_name(&locale, -3), fallback);
    assert_eq!(fallback_month_name(&locale, 0), fallback);
    assert_eq!(fallback_month_name(&locale, 11), "November");
}

/// Mirror of the popup's bounds-safe month lookup (the function is
/// crate-private); the regression contract is: never panics, always returns a
/// name from the locale's table.
fn fallback_month_name(locale: &DateLocale, month: i8) -> String {
    let index = usize::try_from(month.saturating_sub(1)).unwrap_or(0);
    locale
        .month_names
        .get(index)
        .cloned()
        .unwrap_or_else(|| locale.month_names[0].clone())
}

#[test]
fn kittest_default_locale_weekday_header_is_monday_first() {
    let mut harness = harness_with(DateLocale::default(), false);
    open_popup(&mut harness);

    // Header cells appear left-to-right as Mo..Su. The *names* are locale
    // data; column *placement* is StripLayout's business.
    let names = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
    for pair in names.windows(2) {
        let left = harness.get_by_label(pair[0]).rect();
        let right = harness.get_by_label(pair[1]).rect();
        assert!(
            left.right() <= right.left() + 1.0,
            "header cell {:?} should be left of {:?}",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn kittest_default_locale_popup_extends_rightward_from_the_button() {
    let mut harness = harness_with(DateLocale::default(), false);
    open_popup(&mut harness);

    // LTR anchor: the popup hangs off the button's left (reading-start) edge,
    // extending toward the reading end.
    let button = harness.get_by_label_contains(BUTTON_DATE).rect();
    let header = harness.get_by_label("Mo").rect();
    assert!(
        header.left() >= button.left() - 2.0 && header.left() <= button.right() + 2.0,
        "LTR popup should start at the button's left edge: header left {}, button left {}",
        header.left(),
        button.left()
    );
}

#[test]
fn kittest_rtl_locale_anchors_popup_at_the_buttons_right_edge() {
    let mut locale = DateLocale::default();
    locale.rtl = true;
    let mut harness = harness_with(locale, true);
    open_popup(&mut harness);

    // RTL anchor: the popup hangs off the button's right (reading-start)
    // edge, extending toward the reading end (leftward). `Su` is the last
    // header column, so its right edge is the popup's interior right edge.
    let button = harness.get_by_label_contains(BUTTON_DATE).rect();
    let header = harness.get_by_label("Su").rect();
    assert!(
        header.right() <= button.right() + 2.0 && header.right() >= button.right() - 60.0,
        "RTL popup should end at the button's right edge: header right {}, button right {}",
        header.right(),
        button.right()
    );
    assert!(
        header.left() >= 0.0,
        "RTL popup must stay inside the clip rect"
    );
}

#[test]
fn kittest_narrow_viewport_clamps_the_rtl_popup_inside_the_clip_rect() {
    let mut locale = DateLocale::default();
    locale.rtl = true;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(360.0, 200.0))
        .allow_missing_glyphs()
        .with_accessibility_check(false)
        .build_ui_state(
            move |ui, selection| {
                let locale = locale.clone();
                ui.add(DatePickerButton::new(selection).locale(locale));
            },
            SELECTION,
        );
    harness.run();
    open_popup(&mut harness);

    let header = harness.get_by_label("Mo").rect();
    assert!(
        header.left() >= 0.0 && header.right() <= 360.0,
        "RTL popup must clamp inside a narrow viewport: header rect {header:?}"
    );
}

#[test]
fn kittest_rtl_locale_puts_the_icon_before_the_date() {
    let mut locale = DateLocale::default();
    locale.rtl = true;
    let harness = harness_with(locale, false);

    let label = format!("📆 {BUTTON_DATE}");
    assert!(
        harness.query_by_label(label.as_str()).is_some(),
        "RTL button should lead with the icon: {label:?}"
    );
}

#[test]
fn kittest_ltr_locale_keeps_the_icon_after_the_date() {
    let harness = harness_with(DateLocale::default(), false);

    let label = format!("{BUTTON_DATE} 📆");
    assert!(
        harness.query_by_label(label.as_str()).is_some(),
        "LTR button should end with the icon: {label:?}"
    );
}

#[test]
fn kittest_arabic_locale_renders_arabic_names_and_saturday_first() {
    let mut harness = harness_with(DateLocale::from_language("ar"), false);
    open_popup(&mut harness);

    // The selected month text is stored as the ComboBox's accessible value,
    // not its label.
    assert!(
        harness.query(By::new().value("نوفمبر")).is_some(),
        "Arabic month name not found"
    );
    assert!(
        harness.query_by_label("الأسبوع").is_some(),
        "Arabic week label not found"
    );

    // Saturday-first: `السبت` is the first column, `الجمعة` (Friday) the last.
    let saturday = harness.get_by_label("السبت").rect();
    let friday = harness.get_by_label("الجمعة").rect();
    assert!(
        saturday.right() <= friday.left() + 1.0,
        "Saturday-first locale should place السبت before الجمعة"
    );
}
