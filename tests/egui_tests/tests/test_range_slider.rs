//! Tests for [`egui::RangeSlider`], which places two handles on one rail.

use egui::accesskit::Role;
use egui::{Key, RangeSlider, Rangef, SliderClamping};
use egui_kittest::{Harness, kittest::Queryable as _};

struct State {
    range: Rangef,
}

fn harness() -> Harness<'static, State> {
    harness_with_step(1.0)
}

fn harness_with_step(step: f64) -> Harness<'static, State> {
    harness_with(Rangef::new(20.0, 80.0), move |slider| slider.step_by(step))
}

/// A range slider over `0..=100` labeled "Range", set up further by `configure`.
fn harness_with(
    range: Rangef,
    configure: impl 'static + for<'a> Fn(RangeSlider<'a>) -> RangeSlider<'a>,
) -> Harness<'static, State> {
    let mut harness = Harness::new_ui_state(
        move |ui, state: &mut State| {
            let slider = RangeSlider::new(&mut state.range.min, &mut state.range.max, 0.0..=100.0)
                .text("Range");
            ui.add(configure(slider));
        },
        State { range },
    );
    harness.run();
    harness
}

#[test]
fn each_handle_is_its_own_focus_stop() {
    let harness = harness();
    harness.get_by_role_and_label(Role::Slider, "Range low");
    harness.get_by_role_and_label(Role::Slider, "Range high");
}

#[test]
fn arrow_keys_move_the_focused_handle() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(Role::Slider, "Range low")
        .focus();
    harness.run();

    harness.key_press(Key::ArrowRight);
    harness.run();

    assert_eq!(harness.state().range.min, 21.0);
    assert_eq!(
        harness.state().range.max,
        80.0,
        "the other handle stays put"
    );
}

#[test]
fn a_handle_stops_at_its_neighbor() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(Role::Slider, "Range low")
        .focus();
    harness.run();

    for _ in 0..70 {
        harness.key_press(Key::ArrowRight);
        harness.run();
    }

    assert_eq!(harness.state().range.max, 80.0);
    assert_eq!(
        harness.state().range.min,
        80.0,
        "the low handle stops on the high one rather than passing it"
    );
}

#[test]
fn existing_values_are_written_back_into_range() {
    // With the default clamping the caller's variables are pulled into range on the first
    // frame, as `Slider` does, rather than only being drawn as if they were.
    let harness = harness_with(Rangef::new(-20.0, 150.0), |slider| slider.step_by(1.0));
    assert_eq!(harness.state().range.min, 0.0);
    assert_eq!(harness.state().range.max, 100.0);
}

#[test]
fn rounding_to_the_step_never_crosses_the_other_handle() {
    // `high` sits off the step grid, which `Edits` leaves alone, so the step that would land
    // on 12 has to stop at 10.
    let mut harness = harness_with(Rangef::new(0.0, 10.0), |slider| {
        slider.step_by(4.0).clamping(SliderClamping::Edits)
    });
    harness
        .get_by_role_and_label(Role::Slider, "Range low")
        .focus();
    harness.run();

    for _ in 0..5 {
        harness.key_press(Key::ArrowRight);
        harness.run();
    }

    assert_eq!(
        harness.state().range.max,
        10.0,
        "the high handle must not be pushed"
    );
    assert_eq!(harness.state().range.min, 10.0);
}

#[test]
fn on_a_vertical_rail_the_up_arrow_raises_the_value_with_or_without_a_step() {
    for step in [0.0, 1.0] {
        let mut harness = harness_with(Rangef::new(20.0, 80.0), move |slider| {
            slider.vertical().step_by(step)
        });
        harness
            .get_by_role_and_label(Role::Slider, "Range low")
            .focus();
        harness.run();

        harness.key_press(Key::ArrowUp);
        harness.run();

        assert!(
            20.0 < harness.state().range.min,
            "step {step}: up should raise the value, got {}",
            harness.state().range.min
        );
    }
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

    assert_eq!(harness.state().range.min, 25.0);
}
