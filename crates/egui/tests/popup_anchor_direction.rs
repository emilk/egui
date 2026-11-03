//! Regression tests for the direction-aware default popup anchor: a context menu
//! hangs from the pointer at its reading-start edge
//! ([`egui::RectAlign::BOTTOM_START`] in a left-to-right interface, mirrored to
//! [`egui::RectAlign::BOTTOM_END`] when [`egui::Style::right_to_left`] is set),
//! and the screen-edge fallback keeps the popup inside the screen in both
//! directions.
//!
//! No snapshot assertions; geometry only, read from the popup's area rect.

use egui::{
    Context, Event, Id, Modifiers, PointerButton, Popup, Pos2, RawInput, Rect, Sense, Ui, pos2,
    vec2,
};

/// The context menu popup's area rect after opening it with a secondary click on
/// a full-screen widget at `anchor` (the body is 200x140, so the outer popup is
/// ≈212x152). Frames: hover (a press on the very first frame finds no hovered
/// widget), secondary press, secondary release (opens the menu as its invisible
/// sizing pass), then two settles so it shows at its real position.
fn context_menu_rect(right_to_left: bool, anchor: Pos2) -> Rect {
    let ctx = Context::default();
    ctx.set_right_to_left(right_to_left);

    let screen = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0));
    let raw = |pressed: Option<bool>| {
        let mut events = vec![Event::PointerMoved(anchor)];
        if let Some(pressed) = pressed {
            events.push(Event::PointerButton {
                pos: anchor,
                button: PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        RawInput {
            screen_rect: Some(screen),
            events,
            ..Default::default()
        }
    };

    let id = Id::unique("rtl-popup-anchor-root");
    let mut popup_id = Id::NULL;

    let mut show = |ui: &mut Ui| {
        let response = ui.interact(screen, id, Sense::click());
        let popup = Popup::context_menu(&response);
        popup_id = popup.get_id();
        popup.show(|ui| {
            ui.set_min_size(vec2(200.0, 140.0));
            ui.label("row");
        });
    };

    for input in [
        raw(None),
        raw(Some(true)),
        raw(Some(false)),
        raw(None),
        raw(None),
    ] {
        let output = ctx.run_ui(input, &mut show);
        output.drop_without_applying_deltas();
    }

    ctx.memory(|memory| memory.area_rect(popup_id))
        .expect("the context menu should be shown")
}

/// A20.1: the default anchor mirrors with the interface direction — the popup's
/// reading-start edge sits at the pointer (its right edge under RTL, its left
/// edge under LTR) and it hangs below the pointer in both directions.
#[test]
fn context_menu_anchor_side_flips_with_direction() {
    let anchor = pos2(500.0, 100.0);

    let rtl = context_menu_rect(true, anchor);
    assert!(
        (rtl.right() - anchor.x).abs() < 0.5,
        "RTL: the popup's reading-start (right) edge should sit at the anchor: {rtl:?}"
    );
    assert!(
        (rtl.top() - anchor.y).abs() < 0.5,
        "RTL: the popup should hang below the anchor: {rtl:?}"
    );

    let ltr = context_menu_rect(false, anchor);
    assert!(
        (ltr.left() - anchor.x).abs() < 0.5,
        "LTR: the popup's reading-start (left) edge should sit at the anchor: {ltr:?}"
    );
    assert!(
        (ltr.top() - anchor.y).abs() < 0.5,
        "LTR: the popup should hang below the anchor: {ltr:?}"
    );
}

/// A20.3: near the reading-start screen edge the default side does not fit, so
/// the best-align fallback flips to the other side; either way the whole popup
/// stays inside the screen, in both directions.
#[test]
fn context_menu_stays_clamped_inside_the_screen_in_both_directions() {
    let screen = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0));

    // RTL near the reading-start (left) edge: BOTTOM_END does not fit, so the
    // popup flips to rightward growth:
    let rtl = context_menu_rect(true, pos2(50.0, 100.0));
    assert!(
        screen.contains_rect(rtl),
        "RTL near the left edge: the popup should stay inside the screen: {rtl:?}"
    );
    assert!(
        (rtl.left() - 50.0).abs() < 0.5,
        "RTL near the left edge: the popup should grow rightward from the pointer: {rtl:?}"
    );

    // LTR near the reading-end (right) edge: BOTTOM_START does not fit, so the
    // popup flips to leftward growth:
    let ltr = context_menu_rect(false, pos2(790.0, 100.0));
    assert!(
        screen.contains_rect(ltr),
        "LTR near the right edge: the popup should stay inside the screen: {ltr:?}"
    );
    assert!(
        (ltr.right() - 790.0).abs() < 0.5,
        "LTR near the right edge: the popup should grow leftward from the pointer: {ltr:?}"
    );
}
