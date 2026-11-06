//! Regression test for the "Manual Layout Test" demo's manually placed widget
//! under RTL.
//!
//! The demo builds the manually placed widget's rect with
//! `ui.min_rect().min + widget_offset` — physical, direction-blind math. In a
//! right-to-left interface the window body is re-seeded and laid out at the
//! reading-start (right) edge, so `ui.min_rect().min` is the reading-END edge
//! and the widget ends up hanging off the reading-end side of the window,
//! growing physically rightward (against the reading direction) while the
//! window keeps its reading-start edge fixed and widens leftward. The demo
//! must anchor the widget at the reading-start edge instead, so it grows
//! reading-forward like every other element in the window.
//!
//! The test drives the real demo shell (`DemoWindows::ui`) and opens the
//! "Manual Layout Test" window the way a user does, by clicking its entry in
//! the demo list, then drives the unlabeled offset/size sliders through the
//! AccessKit tree and reads the resulting widget geometry from the
//! `Example button` node (the demo places it with `Ui::put`, which justifies
//! the widget to fill the given rect, so the button rect IS the widget rect).
//! No snapshot assertions; geometry only.
//!
//! Slider selection: the shell renders other default-open demo windows
//! (About, Code Example, Widget Gallery) that own their own sliders, so
//! global tree-order indices are not stable. The demo's sliders are picked by
//! grid row instead: each row's label ("Widget position:" / "Widget size:")
//! is unique in the whole shell, and the row's primary slider (offset.x /
//! size.x) is the one nearest its label.

use egui::Vec2;
use egui::accesskit::Role;
use egui_kittest::{Harness, Node, kittest::Queryable as _};

const DEMO_LIST_ENTRY: &str = "Manual Layout Test";
const RESET_BUTTON: &str = "Reset";
const EXAMPLE_BUTTON: &str = "Example button";
const POSITION_ROW_LABEL: &str = "Widget position:";
const SIZE_ROW_LABEL: &str = "Widget size:";

/// The harness size. Roomy enough that the demo's
/// `.constrain_to(ui.available_rect_before_wrap())` never clamps the window
/// even when the widget grows to its widest asserted size — a clamp would
/// move the reading-start edge and fake a failure.
const HARNESS_SIZE: Vec2 = Vec2::new(1400.0, 900.0);

/// The slider nearest the given grid-row label, among the sliders sharing
/// that label's row (y band). The demo's grid rows each hold exactly two
/// sliders, and the row's first cell after the label is its primary slider
/// (offset.x in the position row, size.x in the size row) — in both reading
/// directions.
fn row_slider<'h>(
    harness: &'h Harness<'_, egui_demo_lib::DemoWindows>,
    row_label: &str,
) -> Node<'h> {
    let label = harness.get_by_label(row_label).rect();
    let mut in_row: Vec<Node<'h>> = harness
        .get_all_by_role(Role::Slider)
        .filter(|s| (s.rect().center().y - label.center().y).abs() < 12.0)
        .collect();
    assert_eq!(
        in_row.len(),
        2,
        "expected exactly two sliders in the {row_label} row of the \
         Manual Layout Test window, got {in_row:?}"
    );
    // The row's first cell after the label is its primary slider (offset.x /
    // size.x): the one nearest the row label, in both reading directions.
    let dist = |s: &Node<'h>| (s.rect().center().x - label.center().x).abs();
    if dist(&in_row[0]) <= dist(&in_row[1]) {
        in_row.remove(0)
    } else {
        in_row.remove(1)
    }
}

fn harness(rtl: bool) -> Harness<'static, egui_demo_lib::DemoWindows> {
    let mut harness = Harness::builder()
        .with_size(HARNESS_SIZE)
        .with_right_to_left(rtl)
        .build_ui_state(
            |ui, demo: &mut egui_demo_lib::DemoWindows| demo.ui(ui),
            egui_demo_lib::DemoWindows::default(),
        );
    harness.run();

    // Open the window via its demo-list entry while it is closed: the entry
    // label is ambiguous once the window is open (the title carries the same
    // text), so it is never queried again afterwards.
    harness.get_by_label(DEMO_LIST_ENTRY).click_accesskit();
    harness.run();
    harness.run();
    harness
}

/// The widget must follow the reading direction under RTL: anchored at the
/// content's reading-start edge (`Reset.right()`, where the body lays out its
/// first widget), offset toward the reading end, and growing reading-forward
/// (leftward, with the reading-start edge fixed) when its size increases.
///
/// The four assertions are the fail-before evidence for the demo fix; each
/// fails at base for its own reason (containment: the widget pokes past the
/// reading-start edge; anchor: mirrored offset arithmetic; growth: the delta
/// has the wrong sign; containment-after: the grown widget is still outside).
/// The containment assertions double as an end-to-end pin of the auto-sized
/// window's reading-start-anchored growth: at `size.x = 300` the widget
/// extends 600 px to the reading-end side of the content's own ~407 px, so
/// they can only pass if the window grew on the reading-end side while its
/// reading-start edge stayed fixed.
#[test]
fn manual_layout_widget_follows_the_reading_direction_under_rtl() {
    let mut harness = harness(true);

    // Move the widget 300 px in from the content's reading-start edge (the
    // demo's offset slider range is 0..=400):
    row_slider(&harness, POSITION_ROW_LABEL).set_value_accesskit(300.0);
    harness.run();
    harness.run();

    let reset = harness.get_by_label(RESET_BUTTON).rect();
    let button = harness.get_by_label(EXAMPLE_BUTTON).rect();

    // Containment: the widget must live on the reading-start side of the
    // content's reading-start edge (the `2.0` covers frame rounding).
    assert!(
        button.right() <= reset.right() + 2.0,
        "RTL: the widget must not poke past the content's reading-start edge \
         (button.right() {}, Reset.right() {})",
        button.right(),
        reset.right()
    );

    // Exact anchor: with `size.x` still at its demo default of 200, the
    // widget's center must sit at reading-start − offset − size.x / 2. The
    // `100.0` is half the demo's default widget width (the demo's default
    // `widget_size` is (200, 100)); hard-coding it is the point — a silent
    // demo-default change should fail loudly here.
    let expected_center_x = reset.right() - 300.0 - 100.0;
    assert!(
        (button.center().x - expected_center_x).abs() <= 2.0,
        "RTL: the widget center must sit at reading-start − offset − \
         size.x/2 = {expected_center_x}, got {}",
        button.center().x
    );

    // Grow the widget from its default width 200 to 300 (the demo's size
    // slider range is 0..=400):
    row_slider(&harness, SIZE_ROW_LABEL).set_value_accesskit(300.0);
    harness.run();
    harness.run();
    let button2 = harness.get_by_label(EXAMPLE_BUTTON).rect();

    // Reading-forward growth: growing `size.x` by 100 must move the widget 50
    // px toward the reading end (leftward) with the reading-start edge fixed.
    let growth = button2.center().x - button.center().x;
    assert!(
        (growth + 50.0).abs() <= 2.0,
        "RTL: growing size.x by 100 must move the widget 50 px toward the \
         reading end (leftward), got a shift of {growth}"
    );

    // Containment still holds after the growth:
    assert!(
        button2.right() <= reset.right() + 2.0,
        "RTL: the grown widget must stay on the reading-start side of the \
         content's reading-start edge (button.right() {}, Reset.right() {})",
        button2.right(),
        reset.right()
    );
}

/// LTR twin: unchanged behaviour. Under a left-to-right interface the
/// content's reading-start edge is `Reset.left()` and the widget's center
/// must sit at content-left + offset + size.x/2 — exactly the original
/// formula's consequence. Green at base and must stay green (the fix's LTR
/// branch is the original expression verbatim).
#[test]
fn manual_layout_widget_ltr_is_unchanged() {
    let mut harness = harness(false);

    row_slider(&harness, POSITION_ROW_LABEL).set_value_accesskit(300.0);
    harness.run();
    harness.run();
    row_slider(&harness, SIZE_ROW_LABEL).set_value_accesskit(300.0);
    harness.run();
    harness.run();

    let reset = harness.get_by_label(RESET_BUTTON).rect();
    let button = harness.get_by_label(EXAMPLE_BUTTON).rect();

    let expected_center_x = reset.left() + 300.0 + 150.0;
    assert!(
        (button.center().x - expected_center_x).abs() <= 2.0,
        "LTR: the widget center must sit at content-left + offset + \
         size.x/2 = {expected_center_x}, got {}",
        button.center().x
    );
}

/// LTR growth pin: growing `size.x` must grow the widget reading-forward
/// (rightward) with its left edge unchanged. Guards the LTR branch of the
/// demo's rect math against future edits. Green at base and must stay green.
#[test]
fn manual_layout_widget_ltr_grows_reading_forward() {
    let mut harness = harness(false);

    let b1 = harness.get_by_label(EXAMPLE_BUTTON).rect();

    row_slider(&harness, SIZE_ROW_LABEL).set_value_accesskit(300.0);
    harness.run();
    harness.run();
    let b2 = harness.get_by_label(EXAMPLE_BUTTON).rect();

    let growth = b2.center().x - b1.center().x;
    assert!(
        (growth - 50.0).abs() <= 2.0,
        "LTR: growing size.x by 100 must move the widget 50 px toward the \
         reading end (rightward), got a shift of {growth}"
    );
    // The widget's left edge must be unchanged. The button's own left edge
    // moves when the justified widget grows rightward, so this routes through
    // center − size.x/2 (size.x 200 → 300):
    let left_edge_delta = (b2.center().x - 150.0) - (b1.center().x - 100.0);
    assert!(
        left_edge_delta.abs() <= 2.0,
        "LTR: growing size.x must leave the widget's left edge unchanged, \
         got a shift of {left_edge_delta}"
    );
}
