//! Idle-width stability for the real demo windows in Arabic/RTL.

use egui::{Rect, Vec2, accesskit::Role};
use egui_demo_lib::{DemoWindows, i18n};
use egui_kittest::{Harness, kittest::Queryable as _};

const HARNESS_SIZE: Vec2 = Vec2::new(1400.0, 900.0);

fn assert_idle_width_stable(
    demo_list_entry: &str,
    window_title: &str,
    translate_window_title: bool,
) {
    let arabic = i18n::Lang::from_code("ar").expect("Arabic is a registered language");
    i18n::set_current(arabic);

    let mut harness = Harness::builder()
        .with_size(HARNESS_SIZE)
        .with_right_to_left(true)
        .with_rtl_font()
        .allow_missing_glyphs()
        .build_ui_state(
            |ui, demo_windows: &mut DemoWindows| demo_windows.ui(ui),
            DemoWindows::default(),
        );
    harness.run();

    harness
        .get_by_label(i18n::t(demo_list_entry))
        .click_accesskit();
    harness.run_steps(3);

    let window_title = if translate_window_title {
        i18n::t(window_title)
    } else {
        window_title
    };
    let baseline = window_rect(&harness, window_title);
    let mut rects = vec![baseline];
    for _ in 0..30 {
        harness.step();
        rects.push(window_rect(&harness, window_title));
    }
    for (frame, current) in rects.iter().copied().enumerate().skip(1) {
        assert_stable(baseline, current, demo_list_entry, frame - 1, &rects);
    }

    i18n::set_current(i18n::Lang::EN);
}

fn window_rect(harness: &Harness<'_, DemoWindows>, name: &str) -> Rect {
    harness.get_by_role_and_label(Role::Window, name).rect()
}

#[track_caller]
fn assert_stable(baseline: Rect, current: Rect, demo_name: &str, frame: usize, rects: &[Rect]) {
    assert!(
        (current.width() - baseline.width()).abs() <= 1.0,
        "{demo_name}: idle frame {frame} changed the window width \
         (baseline {baseline:?}, current {current:?}, rects {rects:?})"
    );
    assert!(
        (current.right() - baseline.right()).abs() <= 1.0,
        "{demo_name}: idle frame {frame} moved the reading-start edge \
         (baseline {baseline:?}, current {current:?}, rects {rects:?})"
    );
}

#[test]
fn text_layout_window_idle_width_is_stable_in_arabic_rtl() {
    assert_idle_width_stable("📄 Text Layout", "📄 Text Layout", false);
}

#[test]
fn id_test_window_idle_width_is_stable_in_arabic_rtl() {
    assert_idle_width_stable("ID Test", "ID Test", true);
}

#[test]
fn strip_window_idle_width_is_stable_in_arabic_rtl() {
    assert_idle_width_stable("▣ Strip", "▣ Strip", true);
}

#[test]
fn resizable_text_edit_window_idle_width_is_stable_in_arabic_rtl() {
    assert_idle_width_stable("Window Resize Test", "↔️ resizable with TextEdit", true);
}
