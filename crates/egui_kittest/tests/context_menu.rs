use egui::{Modifiers, PointerButton, Popup, Pos2, Ui};
use egui_kittest::Harness;
use kittest::Queryable as _;

const OUTSIDE_POS: Pos2 = Pos2::new(450.0, 280.0);

#[derive(Default)]
struct ContextMenuTest {
    item_clicked: bool,
}

impl ContextMenuTest {
    fn ui(&mut self, ui: &mut Ui) {
        let response = ui.button("Right-click me");
        Popup::context_menu(&response).show(|ui| {
            if ui.button("Item").clicked() {
                self.item_clicked = true;
            }
            ui.menu_button("Submenu", |ui| {
                _ = ui.button("Sub item");
            });
        });
    }

    fn into_harness(self) -> Harness<'static, Self> {
        Harness::builder()
            .with_size(egui::Vec2::new(500.0, 300.0))
            .build_ui_state(|ui, state| state.ui(ui), self)
    }
}

fn press_at(harness: &Harness<'_, ContextMenuTest>, pos: Pos2, button: PointerButton) {
    harness.event(egui::Event::PointerMoved(pos));
    harness.event(egui::Event::PointerButton {
        pos,
        button,
        pressed: true,
        modifiers: Modifiers::default(),
    });
}

fn release_at(harness: &Harness<'_, ContextMenuTest>, pos: Pos2, button: PointerButton) {
    harness.event(egui::Event::PointerButton {
        pos,
        button,
        pressed: false,
        modifiers: Modifiers::default(),
    });
}

/// Secondary-press the anchor button and run, so the menu is open
/// while the button is still held down.
fn open_with_press(harness: &mut Harness<'_, ContextMenuTest>) -> Pos2 {
    let pos = harness.get_by_label("Right-click me").rect().center();
    press_at(harness, pos, PointerButton::Secondary);
    harness.run();
    pos
}

#[test]
fn context_menu_opens_on_secondary_press() {
    let mut harness = ContextMenuTest::default().into_harness();

    // The menu should open on press, before the button is released.
    open_with_press(&mut harness);
    assert!(harness.query_by_label("Item").is_some());
}

#[test]
fn context_menu_stays_open_after_release() {
    let mut harness = ContextMenuTest::default().into_harness();

    let pos = open_with_press(&mut harness);
    release_at(&harness, pos, PointerButton::Secondary);
    harness.run();
    harness.run();
    assert!(harness.query_by_label("Item").is_some());
}

#[test]
fn context_menu_quick_right_click_keeps_menu_open() {
    let mut harness = ContextMenuTest::default().into_harness();

    // Press and release arrive in the same frame here. The release lands at the
    // menu's corner (the menu opens at the pointer), and must not close it.
    harness.get_by_label("Right-click me").click_secondary();
    harness.run();
    harness.run();
    assert!(harness.query_by_label("Item").is_some());
}

#[test]
fn context_menu_closes_on_primary_press_outside() {
    let mut harness = ContextMenuTest::default().into_harness();

    open_with_press(&mut harness);

    // The menu should close on press, before the button is released.
    press_at(&harness, OUTSIDE_POS, PointerButton::Primary);
    harness.run();
    assert!(harness.query_by_label("Item").is_none());
    assert!(!harness.state().item_clicked);

    release_at(&harness, OUTSIDE_POS, PointerButton::Primary);
    harness.run();
    assert!(harness.query_by_label("Item").is_none());
}

#[test]
fn context_menu_closes_on_secondary_press_outside() {
    let mut harness = ContextMenuTest::default().into_harness();

    open_with_press(&mut harness);

    press_at(&harness, OUTSIDE_POS, PointerButton::Secondary);
    harness.run();
    assert!(harness.query_by_label("Item").is_none());

    // A secondary release over empty space should not reopen the menu.
    release_at(&harness, OUTSIDE_POS, PointerButton::Secondary);
    harness.run();
    harness.run();
    assert!(harness.query_by_label("Item").is_none());
}

#[test]
fn context_menu_item_click_fires_and_closes() {
    let mut harness = ContextMenuTest::default().into_harness();

    open_with_press(&mut harness);

    harness.get_by_label("Item").click();
    harness.run();
    assert!(harness.state().item_clicked);
    assert!(harness.query_by_label("Item").is_none());
}

#[test]
fn context_menu_press_inside_does_not_close() {
    let mut harness = ContextMenuTest::default().into_harness();

    open_with_press(&mut harness);

    let item_pos = harness.get_by_label("Item").rect().center();
    press_at(&harness, item_pos, PointerButton::Primary);
    harness.run();
    assert!(harness.query_by_label("Item").is_some());
    assert!(!harness.state().item_clicked);

    // The item fires on release, closing the menu.
    release_at(&harness, item_pos, PointerButton::Primary);
    harness.run();
    assert!(harness.state().item_clicked);
    assert!(harness.query_by_label("Item").is_none());
}

#[test]
fn context_menu_submenu_closes_on_press_outside() {
    let mut harness = ContextMenuTest::default().into_harness();

    open_with_press(&mut harness);

    harness.get_by_label_contains("Submenu").hover();
    harness.run();
    harness.run();
    assert!(harness.query_by_label("Sub item").is_some());

    press_at(&harness, OUTSIDE_POS, PointerButton::Primary);
    harness.run();
    assert!(harness.query_by_label("Sub item").is_none());
    assert!(harness.query_by_label("Item").is_none());
}
