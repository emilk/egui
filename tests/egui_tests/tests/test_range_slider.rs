//! Tests for [`egui::RangeSlider`], which places two handles on one rail.

use egui::accesskit::Role;
use egui::{Key, RangeSlider};
use egui_kittest::{Harness, kittest::Queryable as _};

struct State {
    low: f32,
    high: f32,
}

fn harness() -> Harness<'static, State> {
    harness_with_step(1.0)
}

fn harness_with_step(step: f64) -> Harness<'static, State> {
    harness_with(20.0, 80.0, step)
}

fn harness_with(low: f32, high: f32, step: f64) -> Harness<'static, State> {
    let mut harness = Harness::new_ui_state(
        move |ui, state: &mut State| {
            ui.add(
                RangeSlider::new(&mut state.low, &mut state.high, 0.0..=100.0)
                    .step_by(step)
                    .text("Range"),
            );
        },
        State { low, high },
    );
    harness.run();
    harness
}

#[test]
fn each_handle_is_its_own_focus_stop() {
    let harness = harness();
    assert_eq!(
        harness.query_all_by_role(Role::Slider).count(),
        2,
        "a screen reader should find one slider per handle"
    );
}

#[test]
fn arrow_keys_move_the_focused_handle() {
    let mut harness = harness();
    harness
        .query_all_by_role(Role::Slider)
        .next()
        .unwrap()
        .focus();
    harness.run();

    harness.key_press(Key::ArrowRight);
    harness.run();

    assert_eq!(harness.state().low, 21.0);
    assert_eq!(harness.state().high, 80.0, "the other handle stays put");
}

#[test]
fn a_handle_stops_at_its_neighbor() {
    let mut harness = harness();
    harness
        .query_all_by_role(Role::Slider)
        .next()
        .unwrap()
        .focus();
    harness.run();

    for _ in 0..70 {
        harness.key_press(Key::ArrowRight);
        harness.run();
    }

    assert_eq!(harness.state().high, 80.0);
    assert_eq!(
        harness.state().low,
        80.0,
        "the low handle stops on the high one rather than passing it"
    );
}

#[test]
fn rounding_to_the_step_never_crosses_the_other_handle() {
    // `high` sits off the step grid, so the step that would land on 12 has to stop at 10.
    let mut harness = harness_with(0.0, 10.0, 4.0);
    harness
        .query_all_by_role(Role::Slider)
        .next()
        .unwrap()
        .focus();
    harness.run();

    for _ in 0..5 {
        harness.key_press(Key::ArrowRight);
        harness.run();
    }

    assert_eq!(
        harness.state().high,
        10.0,
        "the high handle must not be pushed"
    );
    assert_eq!(harness.state().low, 10.0);
}

#[test]
fn the_numbers_beside_the_rail_move_by_the_step() {
    // An arrow key on a focused number moves it by `step`, as it does for `Slider`. Without
    // this the number moves by the drag speed and rounding snaps it straight back.
    let mut harness = harness_with_step(5.0);
    harness
        .query_all_by_role(Role::SpinButton)
        .next()
        .unwrap()
        .focus();
    harness.run();

    harness.key_press(Key::ArrowUp);
    harness.run();

    assert_eq!(harness.state().low, 25.0);
}
