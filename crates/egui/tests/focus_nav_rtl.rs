//! Regression tests for spatial arrow-key focus navigation under a right-to-left
//! interface: the horizontal arrows navigate in READING order. The mirroring happens
//! at resolution (`Focus::find_widget_in_direction`, driven by the interface flag
//! passed out of `Memory::end_pass`) — the key mapping and the menu code's own
//! physical-arrow interpretation are untouched, and Tab / Shift-Tab registration
//! order stays untouched in both directions.
//!
//! Every mirrored case also asserts its left-to-right counterpart, so this file
//! doubles as the LTR-unchanged regression.

use egui::{
    Button, Context, Event, FullOutput, Id, Key, Modifiers, Pos2, RawInput, Rect, Ui, Vec2, vec2,
};

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

fn input_with(size: Vec2, time: f64, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(time),
        events,
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
    output
}

fn key_press(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

/// The `(id, physical center x)` of the three buttons of a horizontal row, with the
/// middle button focused. The row reads A, B, C.
fn setup_row(ctx: &Context, flag_rtl: bool) -> [(Id, f32); 3] {
    let screen = vec2(400.0, 120.0);
    let mut row = None;
    run(ctx, input(screen), |ui| {
        ui.horizontal(|ui| {
            let a = ui.add(Button::new("A"));
            let b = ui.add(Button::new("B"));
            let c = ui.add(Button::new("C"));
            b.request_focus();
            row = Some([
                (a.id, a.rect.center().x),
                (b.id, b.rect.center().x),
                (c.id, c.rect.center().x),
            ]);
        });
    });
    let row = row.expect("the row closure always runs");
    // Sanity: under the flag the row mirrors, so `A` (the reading start) sits at the
    // physical right; left-to-right keeps it at the physical left:
    assert_eq!(
        row[0].1 > row[1].1 && row[1].1 > row[2].1,
        flag_rtl,
        "the row should mirror with the interface flag (A at {}, B at {}, C at {})",
        row[0].1,
        row[1].1,
        row[2].1
    );
    row
}

/// The id of the currently focused widget.
fn focused(ctx: &Context) -> Option<Id> {
    ctx.memory(|mem| mem.focused())
}

/// `ArrowRight` always navigates reading-forward: the physically LEFT button under the
/// flag (the row reads A, B, C right-to-left), the physically right one in LTR. Both
/// are `C`. At parent (no mirror) the flag leg moved focus to the physically right
/// button `A` instead.
#[test]
fn arrow_right_moves_focus_along_the_reading_direction() {
    for flag_rtl in [false, true] {
        let ctx = Context::default();
        if flag_rtl {
            ctx.set_right_to_left(true);
        }
        let screen = vec2(400.0, 120.0);
        let [(_a, ax), (b, _bx), (c, cx)] = setup_row(&ctx, flag_rtl);

        // The middle button has focus after the setup pass:
        assert_eq!(
            focused(&ctx),
            Some(b),
            "the middle button should be focused (flag_rtl = {flag_rtl})"
        );

        // ArrowRight navigates reading-forward — under the flag that is the
        // physically LEFT button; in LTR it is the physically right one. Both are `C`:
        run(
            &ctx,
            input_with(screen, 1.0, vec![key_press(Key::ArrowRight)]),
            |ui| {
                ui.horizontal(|ui| {
                    ui.add(Button::new("A"));
                    ui.add(Button::new("B"));
                    ui.add(Button::new("C"));
                });
            },
        );
        assert_eq!(
            focused(&ctx),
            Some(c),
            "ArrowRight should move focus to the reading-forward neighbor (flag_rtl = {flag_rtl}, \
             A at {ax}, C at {cx})"
        );
    }
}

/// `ArrowLeft` is the mirror image: reading-backward, which is the physically LEFT
/// button in LTR and the physically RIGHT one under the flag. Both are `A`.
#[test]
fn arrow_left_moves_focus_against_the_reading_direction() {
    for flag_rtl in [false, true] {
        let ctx = Context::default();
        if flag_rtl {
            ctx.set_right_to_left(true);
        }
        let screen = vec2(400.0, 120.0);
        let [(a, ax), (_b, _bx), (_c, cx)] = setup_row(&ctx, flag_rtl);

        run(
            &ctx,
            input_with(screen, 1.0, vec![key_press(Key::ArrowLeft)]),
            |ui| {
                ui.horizontal(|ui| {
                    ui.add(Button::new("A"));
                    ui.add(Button::new("B"));
                    ui.add(Button::new("C"));
                });
            },
        );
        assert_eq!(
            focused(&ctx),
            Some(a),
            "ArrowLeft should move focus to the reading-backward neighbor (flag_rtl = {flag_rtl}, \
             A at {ax}, C at {cx})"
        );
    }
}

/// Vertical navigation stays physical in both directions: with all buttons in one
/// horizontal row there is no neighbor above or below, so `ArrowUp` and `ArrowDown` keep
/// the focus on the middle button, under the flag and off.
#[test]
fn vertical_arrows_stay_physical_in_both_directions() {
    for flag_rtl in [false, true] {
        for key in [Key::ArrowUp, Key::ArrowDown] {
            let ctx = Context::default();
            if flag_rtl {
                ctx.set_right_to_left(true);
            }
            let screen = vec2(400.0, 120.0);
            let [(_a, _), (b, _), (_c, _)] = setup_row(&ctx, flag_rtl);

            run(&ctx, input_with(screen, 1.0, vec![key_press(key)]), |ui| {
                ui.horizontal(|ui| {
                    ui.add(Button::new("A"));
                    ui.add(Button::new("B"));
                    ui.add(Button::new("C"));
                });
            });
            assert_eq!(
                focused(&ctx),
                Some(b),
                "{key:?} should keep the focus (flag_rtl = {flag_rtl})"
            );
        }
    }
}
