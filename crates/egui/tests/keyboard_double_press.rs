use egui::{Event, InputOptions, InputState, Key, Modifiers, RawInput};

fn key(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

fn press(key_code: Key) -> Event {
    key(key_code, true, Modifiers::NONE)
}

fn release(key_code: Key) -> Event {
    key(key_code, false, Modifiers::NONE)
}

fn pass(input: InputState, time: f64, events: Vec<Event>) -> InputState {
    input.begin_pass(
        RawInput {
            time: Some(time),
            events,
            ..Default::default()
        },
        false,
        1.0,
        InputOptions {
            max_double_click_delay: 0.25,
            ..Default::default()
        },
    )
}

#[test]
fn double_press_preserves_ordinary_key_queries() {
    for key_code in [Key::Escape, Key::Space] {
        let input = pass(InputState::default(), 0.0, vec![press(key_code)]);
        assert!(input.key_pressed(key_code));
        assert!(input.key_down(key_code));
        assert!(!input.key_double_pressed(key_code));

        let input = pass(input, 0.0625, vec![release(key_code)]);
        assert!(input.key_released(key_code));
        assert!(!input.key_down(key_code));
        assert!(!input.key_double_pressed(key_code));

        let input = pass(input, 0.125, vec![press(key_code)]);
        assert!(input.key_double_pressed(key_code));
        assert!(input.key_double_pressed(key_code)); // Non-consuming.
        assert!(input.key_pressed(key_code));
        assert!(input.key_down(key_code));
        assert_eq!(input.num_presses(key_code), 1);
        assert_eq!(input.events, vec![press(key_code)]);

        let input = pass(input, 0.1875, vec![]);
        assert!(!input.key_double_pressed(key_code));
    }
}

#[test]
fn double_press_delay_has_a_strict_boundary() {
    for (delay, expected) in [(0.125, true), (0.25, false), (0.5, false)] {
        let input = pass(
            InputState::default(),
            1.0,
            vec![press(Key::Space), release(Key::Space)],
        );
        let input = pass(input, 1.0 + delay, vec![press(Key::Space)]);
        assert_eq!(input.key_double_pressed(Key::Space), expected);
    }
}

#[test]
fn repeats_neither_complete_nor_extend_a_pair() {
    let input = pass(InputState::default(), 0.0, vec![press(Key::Space)]);
    let input = pass(input, 0.125, vec![press(Key::Space)]);
    assert!(input.key_pressed(Key::Space));
    assert!(!input.key_double_pressed(Key::Space));
    assert!(matches!(
        input.events.as_slice(),
        [Event::Key { repeat: true, .. }]
    ));
    let input = pass(input, 0.375, vec![press(Key::Space), release(Key::Space)]);
    assert!(!input.key_double_pressed(Key::Space));
    let input = pass(input, 0.5, vec![press(Key::Space)]);
    assert!(!input.key_double_pressed(Key::Space));
}

#[test]
fn later_rapid_presses_complete_overlapping_pairs() {
    let mut input = InputState::default();
    for (time, expected) in [(0.0, false), (0.125, true), (0.25, true), (1.0, false)] {
        input = pass(input, time, vec![press(Key::Escape), release(Key::Escape)]);
        assert_eq!(input.key_double_pressed(Key::Escape), expected);
    }
}

#[test]
fn other_keys_and_modifiers_do_not_interrupt_a_pair() {
    let input = pass(
        InputState::default(),
        0.0,
        vec![
            key(Key::Escape, true, Modifiers::CTRL),
            release(Key::Escape),
        ],
    );
    let input = pass(input, 0.0625, vec![press(Key::Space), release(Key::Space)]);
    assert!(!input.key_double_pressed(Key::Escape));
    assert!(!input.key_double_pressed(Key::Space));
    let input = pass(input, 0.125, vec![press(Key::Escape)]);
    assert!(input.key_double_pressed(Key::Escape));
    assert!(!input.key_double_pressed(Key::Space));
}

#[test]
fn consumed_first_presses_still_count_in_one_or_two_passes() {
    for same_pass in [false, true] {
        let first = vec![
            key(Key::Escape, true, Modifiers::CTRL),
            release(Key::Escape),
        ];
        let mut input = if same_pass {
            let mut events = first;
            events.push(press(Key::Escape));
            pass(InputState::default(), 0.0, events)
        } else {
            pass(InputState::default(), 0.0, first)
        };
        assert!(input.consume_key(Modifiers::CTRL, Key::Escape));
        if !same_pass {
            input = pass(input, 0.125, vec![press(Key::Escape)]);
        }
        assert!(input.key_double_pressed(Key::Escape));
        assert!(input.consume_key(Modifiers::NONE, Key::Escape));
        assert!(!input.key_double_pressed(Key::Escape));
    }
}

#[test]
fn consuming_the_completing_press_leaves_no_double_press() {
    let mut input = pass(
        InputState::default(),
        0.0,
        vec![
            press(Key::Escape),
            release(Key::Escape),
            key(Key::Escape, true, Modifiers::CTRL),
            press(Key::Escape), // A repeat left behind after consuming the completing press.
        ],
    );
    assert!(input.key_double_pressed(Key::Escape));
    assert!(input.consume_key(Modifiers::CTRL, Key::Escape));
    assert!(input.key_pressed(Key::Escape));
    assert!(!input.key_double_pressed(Key::Escape));
}

#[test]
fn focus_loss_resets_history_and_completed_pairs() {
    let input = pass(
        InputState::default(),
        0.0,
        vec![press(Key::Escape), release(Key::Escape)],
    );
    let input = pass(
        input,
        0.125,
        vec![
            press(Key::Escape), // Completes a pair before losing focus.
            Event::WindowFocused(false),
            Event::WindowFocused(true),
            press(Key::Escape), // First press after regaining focus.
        ],
    );
    assert!(!input.key_double_pressed(Key::Escape));
    let input = pass(
        input,
        0.1875,
        vec![release(Key::Escape), press(Key::Escape)],
    );
    assert!(input.key_double_pressed(Key::Escape));
}

#[test]
fn unfocused_raw_input_resets_history_without_changing_held_keys() {
    let input = pass(
        InputState::default(),
        0.0,
        vec![press(Key::Escape), release(Key::Escape)],
    );
    let input = input.begin_pass(
        RawInput {
            focused: false,
            time: Some(0.0625),
            events: vec![press(Key::Escape)],
            ..Default::default()
        },
        false,
        1.0,
        InputOptions::default(),
    );
    assert!(input.key_down(Key::Escape));
    assert!(!input.key_double_pressed(Key::Escape));
    let input = pass(input, 0.125, vec![release(Key::Escape), press(Key::Escape)]);
    assert!(!input.key_double_pressed(Key::Escape));
}

#[test]
fn missing_time_uses_the_existing_predicted_time() {
    let mut input = InputState::default();
    for expected in [false, true] {
        input = input.begin_pass(
            RawInput {
                predicted_dt: 0.125,
                events: vec![press(Key::Space), release(Key::Space)],
                ..Default::default()
            },
            false,
            1.0,
            InputOptions::default(),
        );
        assert_eq!(input.key_double_pressed(Key::Space), expected);
    }
}

#[test]
fn integrations_need_not_identify_repeats() {
    let mut event = press(Key::Escape);
    if let Event::Key { repeat, .. } = &mut event {
        *repeat = true;
    }
    let input = pass(
        InputState::default(),
        0.0,
        vec![event.clone(), release(Key::Escape), event],
    );
    assert!(input.key_double_pressed(Key::Escape));
}

#[test]
fn an_empty_repass_does_not_replay_double_presses() {
    let mut raw = RawInput {
        time: Some(0.0),
        events: vec![press(Key::Space), release(Key::Space), press(Key::Space)],
        ..Default::default()
    };
    let input = InputState::default().begin_pass(raw.take(), false, 1.0, InputOptions::default());
    assert!(input.key_double_pressed(Key::Space));
    let input = input.begin_pass(raw.take(), false, 1.0, InputOptions::default());
    assert!(!input.key_double_pressed(Key::Space));
}

#[cfg(feature = "persistence")]
#[test]
fn double_press_history_is_not_persisted() -> Result<(), Box<dyn core::error::Error>> {
    let input = pass(
        InputState::default(),
        0.0,
        vec![press(Key::Space), release(Key::Space), press(Key::Space)],
    );
    assert!(input.key_double_pressed(Key::Space));
    let serialized = ron::to_string(&input)?;
    assert!(!serialized.contains("key_press_times"));
    assert!(!serialized.contains("double_pressed_keys"));
    let input: InputState = ron::from_str(&serialized)?;
    assert!(!input.key_double_pressed(Key::Space));
    let input = pass(input, 0.125, vec![release(Key::Space), press(Key::Space)]);
    assert!(!input.key_double_pressed(Key::Space));
    Ok(())
}
