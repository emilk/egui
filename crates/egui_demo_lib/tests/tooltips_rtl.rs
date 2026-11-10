//! Regression test for the Tooltips demo's two-column order under RTL.
//!
//! The Tooltips window has two columns: the central "misc tests" pane is the
//! first-read column and the side scroll pane is the second-read one. The side
//! pane must therefore anchor at the *reading end* (`egui::Panel::end`): to the
//! right of the misc pane in a left-to-right interface (unchanged behaviour) and
//! to its left when the interface is right-to-left, so the column order follows
//! the reading direction instead of staying LTR.
//!
//! The test drives the real demo shell (`DemoWindows::ui`) and opens the
//! Tooltips window the way a user does, by clicking its entry in the demo list,
//! then reads the two columns' rects from the AccessKit tree. No snapshot
//! assertions; geometry only.

use egui::Vec2;
use egui_kittest::{Harness, kittest::Queryable as _};

const DEMO_LIST_ENTRY: &str = "💬 Tooltips";
const SCROLL_PANE_LABEL: &str = "This is line 0";
const MISC_PANE_LABEL: &str = "All labels in this demo have tooltips.";

/// Open the Tooltips demo window in the given reading direction and return the
/// AccessKit rects of the scroll pane's and the misc pane's content.
fn tooltips_column_rects(rtl: bool) -> (egui::Rect, egui::Rect) {
    // The direction flag takes effect from the next frame, so it is set once on
    // the first frame, the harness is settled, and only then is the window
    // opened via its demo-list toggle:
    let mut direction_applied = false;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(
            |ui, demo_windows: &mut egui_demo_lib::DemoWindows| {
                if !direction_applied {
                    direction_applied = true;
                    ui.ctx().set_right_to_left(rtl);
                }
                demo_windows.ui(ui);
            },
            egui_demo_lib::DemoWindows::default(),
        );
    harness.run();

    // The entry sits far down the demo list's scroll area, so it is opened via
    // its AccessKit click action, which works for not-currently-visible widgets:
    harness.get_by_label(DEMO_LIST_ENTRY).click_accesskit();
    harness.run();

    let scroll = harness.get_by_label(SCROLL_PANE_LABEL).rect();
    let misc = harness.get_by_label(MISC_PANE_LABEL).rect();
    (scroll, misc)
}

/// The two-column order must follow the reading direction: LTR unchanged
/// (scroll pane right of the misc pane), RTL mirrored (scroll pane left of it).
#[test]
fn tooltips_two_column_order_follows_reading_direction() {
    let (ltr_scroll, ltr_misc) = tooltips_column_rects(false);
    assert!(
        ltr_scroll.center().x > ltr_misc.center().x,
        "LTR: the scroll pane must stay to the right of the misc pane \
         (scroll {ltr_scroll:?}, misc {ltr_misc:?})"
    );

    let (rtl_scroll, rtl_misc) = tooltips_column_rects(true);
    assert!(
        rtl_scroll.center().x < rtl_misc.center().x,
        "RTL: the second-read scroll pane must mirror to the left of the \
         first-read misc pane (scroll {rtl_scroll:?}, misc {rtl_misc:?})"
    );
}
