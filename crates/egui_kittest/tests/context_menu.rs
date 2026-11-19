use egui::{Modifiers, PointerButton, Popup, Pos2};
use egui_kittest::Harness;
use kittest::Queryable as _;

const OUTSIDE: Pos2 = Pos2::new(450.0, 280.0);

fn harness(opens_on_press: bool) -> Harness<'static, bool> {
    let harness = Harness::builder()
        .with_size(egui::Vec2::new(500.0, 300.0))
        .build_ui_state(
            |ui, item_clicked| {
                let response = ui.button("Right-click me");
                Popup::context_menu(&response).show(|ui| {
                    *item_clicked |= ui.button("Item").clicked();
                    ui.menu_button("Submenu", |ui| {
                        _ = ui.button("Sub item");
                    });
                });

                let container = ui.horizontal(|ui| _ = ui.button("Container child"));
                container.response.container_context_menu(|ui| {
                    _ = ui.button("Container item");
                });
            },
            false,
        );
    harness
        .ctx
        .all_styles_mut(|style| style.interaction.context_menu_opens_on_press = opens_on_press);
    harness
}

/// Press or release `button` at `pos`, then run.
fn pointer(harness: &mut Harness<'_, bool>, pos: Pos2, button: PointerButton, pressed: bool) {
    harness.event(egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

fn center(harness: &Harness<'_, bool>, label: &str) -> Pos2 {
    harness.get_by_label(label).rect().center()
}

fn is_shown(harness: &Harness<'_, bool>, label: &str) -> bool {
    harness.query_by_label(label).is_some()
}

#[test]
fn default_opens_and_closes_on_click() {
    let mut harness = harness(false);
    let pos = center(&harness, "Right-click me");

    pointer(&mut harness, pos, PointerButton::Secondary, true);
    assert!(!is_shown(&harness, "Item"));
    pointer(&mut harness, pos, PointerButton::Secondary, false);
    assert!(is_shown(&harness, "Item"));

    pointer(&mut harness, OUTSIDE, PointerButton::Primary, true);
    assert!(is_shown(&harness, "Item"));
    pointer(&mut harness, OUTSIDE, PointerButton::Primary, false);
    assert!(!is_shown(&harness, "Item"));
}

#[test]
fn opens_and_closes_on_press() {
    let mut harness = harness(true);
    let pos = center(&harness, "Right-click me");

    for button in [PointerButton::Primary, PointerButton::Secondary] {
        pointer(&mut harness, pos, PointerButton::Secondary, true);
        assert!(is_shown(&harness, "Item"));

        // The release lands on the menu, which opens at the pointer.
        pointer(&mut harness, pos, PointerButton::Secondary, false);
        assert!(is_shown(&harness, "Item"));

        pointer(&mut harness, OUTSIDE, button, true);
        assert!(!is_shown(&harness, "Item"));
        pointer(&mut harness, OUTSIDE, button, false);
        assert!(!is_shown(&harness, "Item"));
    }
}

#[test]
fn opens_on_press_and_release_in_the_same_frame() {
    let mut harness = harness(true);
    harness.get_by_label("Right-click me").click_secondary();
    harness.run();
    assert!(is_shown(&harness, "Item"));
}

#[test]
fn item_click_fires_and_closes() {
    let mut harness = harness(true);
    let pos = center(&harness, "Right-click me");
    pointer(&mut harness, pos, PointerButton::Secondary, true);
    pointer(&mut harness, pos, PointerButton::Secondary, false);

    let item = center(&harness, "Item");
    pointer(&mut harness, item, PointerButton::Primary, true);
    assert!(is_shown(&harness, "Item"));
    assert!(!harness.state());

    pointer(&mut harness, item, PointerButton::Primary, false);
    assert!(!is_shown(&harness, "Item"));
    assert!(harness.state());
}

#[test]
fn submenu_closes_on_press_outside() {
    let mut harness = harness(true);
    let pos = center(&harness, "Right-click me");
    pointer(&mut harness, pos, PointerButton::Secondary, true);

    harness.get_by_label_contains("Submenu").hover();
    harness.run();
    assert!(is_shown(&harness, "Sub item"));

    pointer(&mut harness, OUTSIDE, PointerButton::Primary, true);
    assert!(!is_shown(&harness, "Sub item"));
    assert!(!is_shown(&harness, "Item"));
}

#[test]
fn container_context_menu_opens_and_closes_on_press() {
    let mut harness = harness(true);
    let pos = center(&harness, "Container child");

    pointer(&mut harness, pos, PointerButton::Secondary, true);
    assert!(is_shown(&harness, "Container item"));

    pointer(&mut harness, OUTSIDE, PointerButton::Primary, true);
    assert!(!is_shown(&harness, "Container item"));
}
