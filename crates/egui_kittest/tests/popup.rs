use egui::{
    Align, Layout, Modifiers, PointerButton, Popup, PopupCloseBehavior, Pos2, Sense,
    SetOpenCommand, Vec2,
};
use egui_kittest::Harness;
use kittest::Queryable as _;

#[test]
fn reopened_popup_resizes_for_wider_items() {
    const POPUP_BUTTON: &str = "Dynamic popup";
    const SHORT_ITEM: &str = "Short item";
    const WIDE_ITEM: &str = "Newly added item with a much wider label";

    #[derive(Default)]
    struct State {
        open: bool,
        show_wide_item: bool,
    }

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(500.0, 300.0))
        .build_ui_state(
            |ui, state| {
                let response = ui.button(POPUP_BUTTON);
                if response.clicked() {
                    state.open = !state.open;
                }

                Popup::from_response(&response)
                    .open(state.open)
                    .layout(Layout::top_down_justified(Align::Min))
                    .show(|ui| {
                        _ = ui.selectable_label(false, SHORT_ITEM);
                        _ = ui.selectable_label(false, "Another short item");
                        if state.show_wide_item {
                            _ = ui.selectable_label(false, WIDE_ITEM);
                        }
                    });
            },
            State::default(),
        );

    harness.get_by_label(POPUP_BUTTON).click();
    harness.run();
    let initial_row_size = harness.get_by_label(SHORT_ITEM).rect().size();

    harness.get_by_label(POPUP_BUTTON).click();
    harness.run();
    assert!(harness.query_by_label(SHORT_ITEM).is_none());

    harness.state_mut().show_wide_item = true;
    harness.run();
    harness.get_by_label(POPUP_BUTTON).click();
    harness.run();

    let reopened_row_size = harness.get_by_label(SHORT_ITEM).rect().size();
    let wide_row_size = harness.get_by_label(WIDE_ITEM).rect().size();

    assert!(
        reopened_row_size.x > initial_row_size.x,
        "reopened row width ({}) did not grow beyond its initial width ({})",
        reopened_row_size.x,
        initial_row_size.x
    );
    assert!(
        wide_row_size.y <= initial_row_size.y + 0.5,
        "new row height ({}) exceeds the single-line row height ({})",
        wide_row_size.y,
        initial_row_size.y
    );
}

#[test]
fn open_popup_resizes_after_explicit_sizing_pass() {
    const POPUP_BUTTON: &str = "Growing popup";
    const MAX_HEIGHT: f32 = 100.0;

    struct State {
        item_count: usize,
        needs_sizing_pass: bool,
        popup_height: f32,
        viewport_height: f32,
        content_height: f32,
    }

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(500.0, 300.0))
        .build_ui_state(
            |ui, state| {
                let response = ui.button(POPUP_BUTTON);
                let needs_sizing_pass = core::mem::take(&mut state.needs_sizing_pass);
                let item_count = state.item_count;

                if let Some(popup) = Popup::from_response(&response)
                    .sizing_pass(needs_sizing_pass)
                    .show(|ui| {
                        egui::ScrollArea::vertical()
                            .max_height(MAX_HEIGHT)
                            .show(ui, |ui| {
                                for index in 0..item_count {
                                    ui.label(format!("Item {index}"));
                                }
                            })
                    })
                {
                    state.popup_height = popup.response.rect.height();
                    state.viewport_height = popup.inner.inner_rect.height();
                    state.content_height = popup.inner.content_size.y;
                }
            },
            State {
                item_count: 2,
                needs_sizing_pass: false,
                popup_height: 0.0,
                viewport_height: 0.0,
                content_height: 0.0,
            },
        );

    harness.run();
    let initial_popup_height = harness.state().popup_height;

    harness.state_mut().item_count = 20;
    harness.run();
    let stale_viewport_height = harness.state().viewport_height;
    assert!(
        stale_viewport_height < MAX_HEIGHT,
        "viewport unexpectedly reached its maximum without a sizing pass"
    );

    harness.state_mut().needs_sizing_pass = true;
    harness.run();

    assert!(
        harness.state().popup_height > initial_popup_height,
        "popup did not grow after an explicit sizing pass"
    );
    assert!(
        harness.state().viewport_height > stale_viewport_height,
        "scroll viewport did not grow after an explicit sizing pass"
    );
    assert!(
        (harness.state().viewport_height - MAX_HEIGHT).abs() <= 0.5,
        "scroll viewport did not stop at its maximum height"
    );
    assert!(
        harness.state().content_height > harness.state().viewport_height,
        "popup contents did not remain scrollable at the maximum height"
    );
}

#[test]
fn test_interactive_tooltip() {
    struct State {
        link_clicked: bool,
    }

    let mut harness = egui_kittest::Harness::new_ui_state(
        |ui, state| {
            ui.label("I have a tooltip").on_hover_ui(|ui| {
                if ui.link("link").clicked() {
                    state.link_clicked = true;
                }
            });
        },
        State {
            link_clicked: false,
        },
    );

    harness.get_by_label_contains("tooltip").hover();
    harness.run();
    harness.get_by_label("link").hover();
    harness.run();
    harness.get_by_label("link").click();

    harness.run();

    assert!(harness.state().link_clicked);
}

fn pointer_button(
    harness: &Harness<'_, impl Sized>,
    pos: Pos2,
    button: PointerButton,
    pressed: bool,
) {
    harness.event(egui::Event::PointerMoved(pos));
    harness.event(egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

/// Press, hold for a few frames, then release, like a real user would.
///
/// [`kittest::Node::click`] sends press and release in the same frame,
/// which hides bugs where press and release are handled differently.
fn slow_click(harness: &mut Harness<'_, impl Sized>, pos: Pos2, button: PointerButton) {
    pointer_button(harness, pos, button, true);
    harness.run_steps(3);
    pointer_button(harness, pos, button, false);
    harness.run();
}

fn harness_builder<State>() -> egui_kittest::HarnessBuilder<State> {
    // Short frames, so that press-hold-release is still a click:
    Harness::builder()
        .with_size(Vec2::new(500.0, 300.0))
        .with_step_dt(1.0 / 60.0)
}

/// A popup opened on a pointer *press* should not close on the following *release*.
///
/// See <https://github.com/emilk/egui/pull/7624>
#[test]
fn popup_opened_on_press_stays_open_after_release() {
    const TARGET: &str = "Right-press me";
    const POPUP_CONTENT: &str = "Popup content";
    const OUTSIDE: Pos2 = Pos2::new(450.0, 250.0);

    for close_behavior in [
        PopupCloseBehavior::CloseOnClick,
        PopupCloseBehavior::CloseOnClickOutside,
    ] {
        let mut harness = harness_builder().build_ui(|ui| {
            let response = ui.add(egui::Label::new(TARGET).sense(Sense::click()));
            let pressed = response.hovered()
                && ui.input(|i| i.pointer.button_pressed(PointerButton::Secondary));
            Popup::from_response(&response)
                .open_memory(pressed.then_some(SetOpenCommand::Bool(true)))
                .close_behavior(close_behavior)
                .show(|ui| {
                    ui.label(POPUP_CONTENT);
                });
        });
        harness.run();

        let target = harness.get_by_label(TARGET).rect().center();

        pointer_button(&harness, target, PointerButton::Secondary, true);
        harness.run_steps(3);
        assert!(
            harness.query_by_label(POPUP_CONTENT).is_some(),
            "{close_behavior:?}: popup should open on press"
        );

        pointer_button(&harness, target, PointerButton::Secondary, false);
        harness.run();
        assert!(
            harness.query_by_label(POPUP_CONTENT).is_some(),
            "{close_behavior:?}: popup should stay open after the release that opened it"
        );

        slow_click(&mut harness, OUTSIDE, PointerButton::Primary);
        assert!(
            harness.query_by_label(POPUP_CONTENT).is_none(),
            "{close_behavior:?}: clicking outside should close the popup"
        );
    }
}

/// Clicking an item inside a [`PopupCloseBehavior::CloseOnClick`] popup should register the click
/// *and* close the popup, also when press and release happen in different frames.
#[test]
fn popup_item_click_registers_and_closes() {
    const BUTTON: &str = "Open menu";
    const ITEM: &str = "Item";

    #[derive(Default)]
    struct State {
        item_clicked: bool,
    }

    let mut harness = harness_builder().build_ui_state(
        |ui, state: &mut State| {
            let response = ui.button(BUTTON);
            Popup::menu(&response).show(|ui| {
                if ui.button(ITEM).clicked() {
                    state.item_clicked = true;
                }
            });
        },
        State::default(),
    );
    harness.run();

    let button = harness.get_by_label(BUTTON).rect().center();
    slow_click(&mut harness, button, PointerButton::Primary);
    assert!(harness.query_by_label(ITEM).is_some(), "menu should open");

    let item = harness.get_by_label(ITEM).rect().center();
    slow_click(&mut harness, item, PointerButton::Primary);
    assert!(harness.state().item_clicked, "item click should register");
    assert!(
        harness.query_by_label(ITEM).is_none(),
        "menu should close after clicking an item"
    );

    // Clicking the menu button should toggle the menu open, then closed:
    slow_click(&mut harness, button, PointerButton::Primary);
    assert!(harness.query_by_label(ITEM).is_some(), "menu should reopen");
    slow_click(&mut harness, button, PointerButton::Primary);
    assert!(
        harness.query_by_label(ITEM).is_none(),
        "clicking the menu button should close the menu"
    );
}

/// Starting a drag outside a popup (e.g. on a slider) should not close it,
/// since that is not a click.
#[test]
fn popup_stays_open_when_dragging_outside() {
    const BUTTON: &str = "Open popup";
    const POPUP_CONTENT: &str = "Popup content";

    let mut harness = harness_builder().build_ui_state(
        |ui, value: &mut f32| {
            ui.add(egui::Slider::new(value, 0.0..=100.0).text("Value"));
            let response = ui.button(BUTTON);
            Popup::menu(&response).show(|ui| {
                ui.label(POPUP_CONTENT);
            });
        },
        0.0,
    );
    harness.run();

    let button = harness.get_by_label(BUTTON).rect().center();
    slow_click(&mut harness, button, PointerButton::Primary);
    assert!(harness.query_by_label(POPUP_CONTENT).is_some());

    let slider = harness.get_by_role(egui::accesskit::Role::Slider).rect();
    harness.drag_at(slider.left_center());
    harness.run_steps(3);
    harness.hover_at(slider.center());
    harness.run_steps(3);
    harness.drop_at(slider.center());
    harness.run();

    assert!(*harness.state() > 0.0, "slider should have been dragged");
    assert!(
        harness.query_by_label(POPUP_CONTENT).is_some(),
        "dragging outside the popup should not close it"
    );
}
