//! Regression tests for the direction-aware default popup anchor: a context menu
//! hangs from the pointer at its reading-start edge
//! ([`egui::RectAlign::BOTTOM_START`] in a left-to-right interface, mirrored to
//! [`egui::RectAlign::BOTTOM_END`] when [`egui::Style::right_to_left`] is set),
//! and the screen-edge fallback keeps the popup inside the screen in both
//! directions.
//!
//! No snapshot assertions; geometry only, read from the popup's area rect.

use egui::{
    Context, Event, Id, Key, LayerId, Modifiers, Order, PointerButton, Popup, PopupAnchor, Pos2,
    RawInput, Rect, Sense, SetOpenCommand, Ui, Vec2, pos2, vec2,
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

// ---------------------------------------------------------------------------

/// Open a [`Popup::menu`] on a widget at `widget_rect` through the pointer-fixed
/// anchor *without any pointer events* (a keyboard open: the anchor has no stored
/// position), and return the popup's area rect after the show pass.
///
/// Frames: one open frame (the invisible sizing pass) and one settle (the show
/// pass); geometry is read after the show pass, never from a sizing pass.
fn keyboard_menu_rect(right_to_left: bool, screen: Vec2, widget_rect: Rect) -> Option<Rect> {
    let ctx = Context::default();
    ctx.set_right_to_left(right_to_left);

    let mut popup_id = Id::NULL;

    let mut show = |ui: &mut Ui| {
        let response = ui.allocate_rect(widget_rect, Sense::click());
        let popup = Popup::menu(&response)
            .at_pointer_fixed()
            .open_memory(Some(SetOpenCommand::Bool(true)));
        popup_id = popup.get_id();
        popup.show(|ui| {
            ui.set_min_size(vec2(200.0, 140.0));
            ui.label("row");
        });
    };

    for time in [0.0, 0.1] {
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, screen)),
                time: Some(time),
                ..Default::default()
            },
            &mut show,
        );
        output.drop_without_applying_deltas();
    }

    ctx.memory(|memory| memory.area_rect(popup_id))
}

/// The area rect of a [`Popup::menu`] forced open on a widget at `widget_rect`
/// (the widget-anchored twin of [`keyboard_menu_rect`]).
fn widget_menu_rect(right_to_left: bool, widget_rect: Rect) -> Rect {
    let ctx = Context::default();
    ctx.set_right_to_left(right_to_left);

    let _id = Id::unique("rtl-widget-menu-anchor");
    let mut popup_id = Id::NULL;

    let mut show = |ui: &mut Ui| {
        let response = ui.allocate_rect(widget_rect, Sense::click());
        let popup = Popup::menu(&response).open(true);
        popup_id = popup.get_id();
        popup.show(|ui| {
            ui.set_min_size(vec2(200.0, 140.0));
            ui.label("row");
        });
    };

    for time in [0.0, 0.1] {
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                time: Some(time),
                ..Default::default()
            },
            &mut show,
        );
        output.drop_without_applying_deltas();
    }

    ctx.memory(|memory| memory.area_rect(popup_id))
        .expect("the widget-anchored menu should be shown")
}

/// A keyboard-opened context menu must render at all: a pointer-fixed popup
/// opened without a pointer hangs from its anchor widget's rect instead of
/// silently never showing.
#[test]
fn keyboard_opened_context_menu_is_shown() {
    let rect = keyboard_menu_rect(
        false,
        Vec2::new(800.0, 600.0),
        Rect::from_min_size(pos2(40.0, 40.0), vec2(40.0, 20.0)),
    );
    let rect = rect.expect("the keyboard-opened menu should be shown");
    assert!(rect.is_finite(), "the popup rect must be finite: {rect:?}");
}

/// The keyboard-opened menu hangs below its anchor widget from the widget's
/// reading-start edge: its right edge under RTL, its left edge under LTR.
#[test]
fn keyboard_opened_menu_hangs_from_the_reading_start_edge() {
    // Mid-screen so the default reading-start align fits unflipped in both
    // directions (the 200x140 body plus frame makes the popup ~214 px wide):
    let widget = Rect::from_min_size(pos2(300.0, 40.0), vec2(40.0, 20.0));
    let screen = Vec2::new(800.0, 600.0);

    let rtl = keyboard_menu_rect(true, screen, widget).expect("the menu should be shown");
    assert!(
        (rtl.right() - widget.right()).abs() < 2.0,
        "RTL: the popup's reading-start (right) edge should sit at the widget's \
         right edge (popup {rtl:?}, widget {widget:?})"
    );
    assert!(
        (rtl.top() - widget.bottom()).abs() < 2.0,
        "RTL: the popup should hang below the widget (popup {rtl:?}, widget {widget:?})"
    );

    let ltr = keyboard_menu_rect(false, screen, widget).expect("the menu should be shown");
    assert!(
        (ltr.left() - widget.left()).abs() < 2.0,
        "LTR: the popup's reading-start (left) edge should sit at the widget's \
         left edge (popup {ltr:?}, widget {widget:?})"
    );
    assert!(
        (ltr.top() - widget.bottom()).abs() < 2.0,
        "LTR: the popup should hang below the widget (popup {ltr:?}, widget {widget:?})"
    );
}

/// Near the reading-start screen edge the default side does not fit, so the
/// best-align fallback flips the growth side; either way the whole popup stays
/// inside the screen, in both directions.
#[test]
fn keyboard_opened_menu_flips_at_screen_edge_in_both_directions() {
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

    // RTL near the reading-start (left) edge: the reading-start side does not
    // fit, so the popup grows rightward:
    let rtl_widget = Rect::from_min_size(pos2(10.0, 100.0), vec2(40.0, 20.0));
    let rtl = keyboard_menu_rect(true, Vec2::new(800.0, 600.0), rtl_widget)
        .expect("the menu should be shown");
    assert!(
        screen.contains_rect(rtl),
        "RTL near the left edge: the popup should stay inside the screen: {rtl:?}"
    );
    assert!(
        rtl.min.x >= rtl_widget.min.x - 1.0,
        "RTL near the left edge: the popup should grow rightward from the widget \
         (popup {rtl:?}, widget {rtl_widget:?})"
    );

    // LTR near the reading-start (right) edge: the reading-start side does not
    // fit, so the popup grows leftward:
    let ltr_widget = Rect::from_min_size(pos2(750.0, 100.0), vec2(40.0, 20.0));
    let ltr = keyboard_menu_rect(false, Vec2::new(800.0, 600.0), ltr_widget)
        .expect("the menu should be shown");
    assert!(
        screen.contains_rect(ltr),
        "LTR near the right edge: the popup should stay inside the screen: {ltr:?}"
    );
    assert!(
        ltr.max.x <= ltr_widget.max.x + 1.0,
        "LTR near the right edge: the popup should grow leftward from the widget \
         (popup {ltr:?}, widget {ltr_widget:?})"
    );
}

/// A keyboard-opened pointer-fixed menu sits exactly where the same widget's
/// widget-anchored menu sits: the fallback transplants the click-open geometry
/// (below the widget, reading-start edges aligned) to the keyboard case.
#[test]
fn keyboard_opened_menu_matches_the_widget_anchored_menu() {
    let widget = Rect::from_min_size(pos2(40.0, 40.0), vec2(40.0, 20.0));

    let rtl_keyboard = keyboard_menu_rect(true, Vec2::new(800.0, 600.0), widget)
        .expect("the keyboard menu should be shown");
    let rtl_widget_anchored = widget_menu_rect(true, widget);
    assert!(
        (rtl_keyboard.min - rtl_widget_anchored.min).length() < 2.0
            && (rtl_keyboard.max - rtl_widget_anchored.max).length() < 2.0,
        "RTL: the keyboard-opened menu should sit where the widget-anchored menu \
         sits (keyboard {rtl_keyboard:?}, anchored {rtl_widget_anchored:?})"
    );

    let ltr_keyboard = keyboard_menu_rect(false, Vec2::new(800.0, 600.0), widget)
        .expect("the keyboard menu should be shown");
    let ltr_widget_anchored = widget_menu_rect(false, widget);
    assert!(
        (ltr_keyboard.min - ltr_widget_anchored.min).length() < 2.0
            && (ltr_keyboard.max - ltr_widget_anchored.max).length() < 2.0,
        "LTR: the keyboard-opened menu should sit where the widget-anchored menu \
         sits (keyboard {ltr_keyboard:?}, anchored {ltr_widget_anchored:?})"
    );
}

/// A menu wider and taller than a narrow screen still opens: the align fallback
/// plus the area clamp keep it inside (or at the edge of) the screen, without
/// panicking or producing NaN.
#[test]
fn keyboard_opened_menu_on_a_narrow_screen_stays_finite_and_clamped() {
    let screen_size = Vec2::new(200.0, 150.0);
    let screen = Rect::from_min_size(Pos2::ZERO, screen_size);

    let rect = keyboard_menu_rect(
        false,
        screen_size,
        Rect::from_min_size(pos2(20.0, 10.0), vec2(40.0, 20.0)),
    )
    .expect("the menu should be shown even when it is wider than the screen");
    assert!(rect.is_finite(), "the popup rect must be finite: {rect:?}");
    assert!(
        rect.min.x >= screen.min.x - 0.5 && rect.min.y >= screen.min.y - 0.5,
        "the popup should be clamped to the screen (popup {rect:?}, screen {screen:?})"
    );
    assert!(
        rect.intersects(screen),
        "the popup should overlap the screen (popup {rect:?}, screen {screen:?})"
    );
}

// ---------------------------------------------------------------------------

/// A popup that cannot compute an anchor rect (a pointer-fixed popup with
/// neither a stored position nor an anchor widget, or a pointer-anchored popup
/// with no pointer) renders nothing but still counts as an open popup, so
/// tooltips stay suppressed while it is stuck.
#[test]
fn anchorless_popups_stay_registered_as_open() {
    let screen = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0));

    for anchor in [PopupAnchor::PointerFixed, PopupAnchor::Pointer] {
        let ctx = Context::default();
        let popup_id = Id::unique("rtl-anchorless-popup");
        let layer_id = LayerId::new(Order::Foreground, Id::unique("rtl-anchorless-layer"));

        let mut show = |ui: &mut Ui| {
            Popup::new(popup_id, ui.ctx().clone(), anchor, layer_id).show(|ui| {
                ui.label("row");
            });
        };
        for time in [0.0, 0.1] {
            let output = ctx.run_ui(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(time),
                    ..Default::default()
                },
                &mut show,
            );
            output.drop_without_applying_deltas();
        }

        assert!(
            ctx.memory(|memory| memory.area_rect(popup_id)).is_none(),
            "the anchorless {anchor:?} popup should render nothing"
        );
        assert!(
            ctx.any_popup_open(),
            "the anchorless {anchor:?} popup should stay registered as open, so \
             tooltips stay suppressed while it is stuck"
        );
    }
}

/// A submenu opened from the keyboard of a keyboard-opened context menu
/// (arrow down to focus the row, Enter to open it) opens on the reading side
/// of its row: to the right in a left-to-right interface, mirrored to the
/// left when [`egui::Style::right_to_left`] is set.
#[test]
fn keyboard_opened_context_menu_opens_a_submenu_on_the_reading_side() {
    for (right_to_left, reading_side_is_left) in [(false, false), (true, true)] {
        let dir = if right_to_left { "RTL" } else { "LTR" };
        let ctx = Context::default();
        ctx.set_right_to_left(right_to_left);

        let screen = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0));
        let widget = Rect::from_min_size(pos2(300.0, 200.0), vec2(40.0, 20.0));

        let row_id = core::cell::RefCell::new(Id::NULL);
        let row_rect = core::cell::RefCell::new(Rect::NOTHING);
        let mut popup_id = Id::NULL;

        let mut show = |ui: &mut Ui| {
            let response = ui.allocate_rect(widget, Sense::click());
            let popup = Popup::menu(&response)
                .at_pointer_fixed()
                .open_memory(Some(SetOpenCommand::Bool(true)));
            popup_id = popup.get_id();
            popup.show(|ui| {
                let row = ui.menu_button("Sub", |ui| {
                    let _ = ui.button("Item");
                });
                *row_id.borrow_mut() = row.response.id;
                *row_rect.borrow_mut() = row.response.rect;
            });
        };

        let input = |time: f64, events: Vec<Event>| RawInput {
            screen_rect: Some(screen),
            time: Some(time),
            events,
            ..Default::default()
        };
        let key = |key: Key, pressed: bool| Event::Key {
            key,
            pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
            physical_key: None,
        };

        // Sizing pass and show pass, no pointer anywhere (a keyboard open).
        for time in [0.0, 0.1] {
            let output = ctx.run_ui(input(time, vec![]), &mut show);
            output.drop_without_applying_deltas();
        }
        // Arrow down focuses the first row, Enter opens its submenu; the last
        // frame is the submenu's show pass.
        for (time, events) in [
            (0.2, vec![key(Key::ArrowDown, true)]),
            (0.3, vec![key(Key::ArrowDown, false)]),
            (0.4, vec![key(Key::Enter, true)]),
            (0.5, vec![key(Key::Enter, false)]),
            (0.6, vec![]),
        ] {
            let output = ctx.run_ui(input(time, events), &mut show);
            output.drop_without_applying_deltas();
        }

        let submenu_id =
            egui::containers::menu::SubMenu::id_from_widget_id(row_id.borrow().to_owned());
        let submenu = ctx
            .memory(|memory| memory.area_rect(submenu_id))
            .unwrap_or_else(|| panic!("{dir}: the submenu should have been shown"));
        let row = row_rect.borrow().to_owned();
        assert!(
            !row.is_negative(),
            "{dir}: the row should have been laid out"
        );

        if reading_side_is_left {
            assert!(
                submenu.max.x <= row.min.x + 2.0,
                "{dir}: the submenu should open on the reading-end (left) side of \
                 its row (submenu {submenu:?}, row {row:?})"
            );
        } else {
            assert!(
                submenu.min.x >= row.max.x - 2.0,
                "{dir}: the submenu should open on the reading-end (right) side of \
                 its row (submenu {submenu:?}, row {row:?})"
            );
        }
    }
}
