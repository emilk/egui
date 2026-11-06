//! Regression test for the "Layout test" demo's inner `Resize` area under RTL.
//!
//! The window's inner resizable area applies a raw `Layout` built from the demo
//! state. Explicit horizontal layouts are never interface-mirrored by core (by
//! design), so the demo must resolve the reading direction itself when handing
//! the layout to `with_layout`: under a right-to-left interface a left-to-right
//! main direction must be mirrored, so the row's items follow the reading order
//! (first item rightmost). Under a left-to-right interface the behaviour is
//! unchanged.
//!
//! The test drives the real demo shell (`DemoWindows::ui`) and opens the
//! "Layout test" window the way a user does, by clicking its entry in the demo
//! list, then switches the inner area to a horizontal layout with the
//! "`LeftToRight`" radio and reads the item order from the AccessKit tree.
//! No snapshot assertions; geometry only.

use egui::Vec2;
use egui_kittest::{Harness, kittest::Queryable as _};

const DEMO_LIST_ENTRY: &str = "Layout Test";
const LEFT_TO_RIGHT_RADIO: &str = "LeftToRight";
const CHECKBOX: &str = "checkbox";
const RADIO: &str = "radio";
const BUTTON: &str = "button";

/// The x centers of the three example widgets inside the window's inner
/// `Resize` area, in the order the demo adds them (checkbox, radio, button).
fn inner_area_item_centers(rtl: bool) -> (f32, f32, f32) {
    // The direction flag takes effect from the next frame, so it is set once on
    // the first frame and the harness is settled before the window is opened:
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
    harness.run();

    // Switch the inner area to a horizontal layout (the default state is
    // vertical, which core already mirrors):
    harness.get_by_label(LEFT_TO_RIGHT_RADIO).click();
    harness.run();
    harness.run();

    let checkbox = harness.get_by_label(CHECKBOX).rect().center().x;
    let radio = harness.get_by_label(RADIO).rect().center().x;
    let button = harness.get_by_label(BUTTON).rect().center().x;
    (checkbox, radio, button)
}

/// The inner area's items must follow the reading direction: LTR unchanged
/// (checkbox left of radio left of button), RTL mirrored (checkbox right of
/// radio right of button).
#[test]
fn layout_test_inner_area_follows_reading_direction() {
    let (ltr_checkbox, ltr_radio, ltr_button) = inner_area_item_centers(false);
    assert!(
        ltr_checkbox < ltr_radio && ltr_radio < ltr_button,
        "LTR: the items must keep their left-to-right add order \
         (checkbox {ltr_checkbox}, radio {ltr_radio}, button {ltr_button})"
    );

    let (rtl_checkbox, rtl_radio, rtl_button) = inner_area_item_centers(true);
    assert!(
        rtl_checkbox > rtl_radio && rtl_radio > rtl_button,
        "RTL: the items must follow the right-to-left reading order, \
         first added rightmost (checkbox {rtl_checkbox}, radio {rtl_radio}, \
         button {rtl_button})"
    );
}
