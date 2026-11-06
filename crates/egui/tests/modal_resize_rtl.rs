//! `Modal` grip-resize behavior under the whole-UI direction flag
//! ([`egui::Style::right_to_left`]): a centered modal wraps its content in an
//! anchored area that re-derives its placement from its live size every frame.
//! While no user resize has happened that is correct — the modal stays centered
//! as its content grows. But once the user drags the [`egui::Resize`] grip,
//! re-centering around the dragged size grows the dialog at double pointer
//! rate (both edges move by half the delta; the reading-start edge runs away).
//! After a user resize the modal's placement pins: the reading-start edge of
//! the placed rect — the right edge under RTL, the left edge under LTR, the
//! top edge in both directions — stays fixed and the dialog grows
//! reading-forward at exactly pointer rate, like a user-resized window.
//!
//! Every case asserts its LTR counterpart. The LTR assertions double as the
//! reviewed LTR bug-fix justification: before the fix an LTR centered modal's
//! grip drag also grew the dialog at double pointer rate with the left
//! (reading-start) edge running away at pointer rate (a "springy" feel); the
//! fix pins it at 1× exactly like the RTL case. No snapshot assertions;
//! geometry only.

use core::cell::Cell;

use egui::{
    Context, Event, Id, Label, Modal, PointerButton, Pos2, RawInput, Rect, Resize, TextWrapMode,
    Vec2, pos2, vec2,
};

fn input_at(size: Vec2, time: f64, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(time),
        events,
        ..Default::default()
    }
}

fn pointer_press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

fn same_rect(a: Rect, b: Rect) -> bool {
    (a.left() - b.left()).abs() <= 0.5
        && (a.right() - b.right()).abs() <= 0.5
        && (a.top() - b.top()).abs() <= 0.5
        && (a.bottom() - b.bottom()).abs() <= 0.5
}

/// What one frame showed: the modal's area rect and the `Resize` corner grip rect.
#[derive(Clone, Copy, Debug)]
struct Snap {
    modal: Rect,
    grip: Rect,
}

/// A reusable modal harness: one context with a fixed screen, one `Modal` id
/// wrapping one `Resize` id, and frame/settle/drag helpers. The drag helper
/// presses and moves on separate frames (a single move frame under-samples the
/// pre-fix double-rate convergence; a coalesced press+move would also shift the
/// pinned edge by half the first frame's delta).
struct Harness {
    ctx: Context,
    screen: Vec2,
    modal_id: Id,
    resize_id: Id,
    /// When set, the content is a wide extend-mode label that cannot wrap, so
    /// the modal auto-grows with its content (the no-drag growth path).
    wide: Cell<bool>,
}

impl Harness {
    /// ~500 points wide when not wrapped — much wider than the 200-point resize.
    const WIDE: &str = "wwwwwwwwww wwwwwwwwww wwwwwwwwww wwwwwwwwww wwwwwwwwww";

    fn new(rtl: bool) -> Self {
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        Self {
            ctx,
            screen: vec2(700.0, 500.0),
            modal_id: Id::unique("modal_resize_rtl_modal"),
            resize_id: Id::unique("modal_resize_rtl_resize"),
            wide: Cell::new(false),
        }
    }

    fn frame(&self, events: Vec<Event>, time: f64) -> Snap {
        let mut snap = Snap {
            modal: Rect::NOTHING,
            grip: Rect::NOTHING,
        };
        let modal_id = self.modal_id;
        let resize_id = self.resize_id;
        let wide = self.wide.get();
        let ctx = &self.ctx;
        let mut output = ctx.run_ui(input_at(self.screen, time, events), |_ui| {
            let response = Modal::new(modal_id)
                .show(ctx, |ui| {
                    Resize::default()
                        .id(resize_id)
                        .default_size(vec2(200.0, 100.0))
                        .show(ui, |ui| {
                            if wide {
                                ui.add(Label::new(Self::WIDE).wrap_mode(TextWrapMode::Extend));
                            } else {
                                ui.label("Content");
                            }
                        });
                })
                .response;
            snap.modal = response.rect;
            if let Some(corner) = ctx.read_response(resize_id.with("__resize_corner")) {
                snap.grip = corner.rect;
            }
        });
        output.textures_delta.clear();
        snap
    }

    /// Run idle frames until the modal stops changing (settling past any
    /// discarded sizing pass).
    fn settle(&self, mut time: f64) -> Snap {
        let mut prev = self.frame(vec![], time);
        for _ in 0..10 {
            time += 0.1;
            let next = self.frame(vec![], time);
            if same_rect(next.modal, prev.modal) {
                return next;
            }
            prev = next;
        }
        prev
    }

    /// Press the grip at its center and move it `steps` times by `step`, one
    /// move per frame, without releasing. Returns the snapshot after every move
    /// frame (in-flight) and the final pointer position.
    fn press_and_drag(&self, time: f64, step: Vec2, steps: usize) -> (Vec<Snap>, Pos2) {
        let idle = self.frame(vec![], time);
        assert!(
            idle.grip.is_positive(),
            "no corner grip response captured (got {idle:?})"
        );
        let start = idle.grip.center();
        self.frame(vec![Event::PointerMoved(start)], time + 0.1);
        self.frame(vec![pointer_press(start, true)], time + 0.2);
        let mut in_flight = Vec::new();
        for i in 1..=steps {
            let target = start + step * (i as f32);
            in_flight.push(self.frame(
                vec![Event::PointerMoved(target)],
                time + 0.2 + i as f64 * 0.1,
            ));
        }
        let pointer = start + step * (steps as f32);
        (in_flight, pointer)
    }

    /// Full drag: press on the grip, `steps` moves of `step`, release, settle.
    /// Returns the in-flight snapshots and the settled post-drag snapshot.
    fn drag(&self, time: f64, step: Vec2, steps: usize) -> (Vec<Snap>, Snap) {
        let (in_flight, pointer) = self.press_and_drag(time, step, steps);
        let release_time = time + 0.2 + steps as f64 * 0.1 + 0.1;
        self.frame(vec![pointer_press(pointer, false)], release_time);
        let settled = self.settle(release_time + 0.1);
        (in_flight, settled)
    }
}

// ---------------------------------------------------------------------------

/// M1: after the user drags the modal's resize grip by 50 points, the dialog
/// grows by exactly 50 points (pointer rate) with its reading-start edge fixed:
/// under RTL the right edge, under LTR the left edge; the top edge does not
/// move.
///
/// LTR bug-fix justification: before the fix this test failed in both
/// directions — the dialog grew at double pointer rate (RTL +91 / LTR +90
/// instead of +50) and the reading-start edge ran away at pointer rate (RTL
/// right +46, LTR left −45) because the anchored placement re-centered around
/// the dragged size every frame.
#[test]
fn modal_grip_drag_keeps_reading_start_edge_fixed() {
    const DRAG: f32 = 50.0;
    const STEP: f32 = 10.0;

    // RTL: the grip sits at the dialog's bottom-left (reading-end) corner and a
    // leftward drag grows the dialog leftward (reading-forward):
    {
        let h = Harness::new(true);
        let before = h.settle(0.0);
        assert!(
            before.modal.is_positive() && before.grip.is_positive(),
            "the modal and its grip should be laid out (got {before:?})"
        );
        let (_, settled) = h.drag(1.0, vec2(-STEP, 0.0), 5);
        let dw = settled.modal.width() - before.modal.width();
        assert!(
            (dw - DRAG).abs() <= 4.0,
            "RTL: a {DRAG}-point grip drag must grow the dialog by {DRAG} points \
             (pointer rate), got {dw:+.1} — the pre-fix dialog grew at double \
             pointer rate (before {:?}, after {:?})",
            before.modal,
            settled.modal
        );
        assert!(
            (settled.modal.right() - before.modal.right()).abs() <= 2.0,
            "RTL: the reading-start (right) edge must stay fixed while the user \
             resizes (before {:?}, after {:?})",
            before.modal,
            settled.modal
        );
        assert!(
            (settled.modal.left() - (before.modal.left() - DRAG)).abs() <= 4.0,
            "RTL: the reading-end (left) edge must move by the width delta \
             (before {:?}, after {:?})",
            before.modal,
            settled.modal
        );
        assert!(
            (settled.modal.top() - before.modal.top()).abs() <= 2.0,
            "RTL: a horizontal drag must not move the top edge \
             (before {:?}, after {:?})",
            before.modal,
            settled.modal
        );
    }

    // LTR counterpart: the grip sits at the bottom-right (reading-end) corner
    // and a rightward drag grows the dialog rightward, the left (reading-start)
    // edge fixed:
    {
        let h = Harness::new(false);
        let before = h.settle(0.0);
        assert!(
            before.modal.is_positive() && before.grip.is_positive(),
            "the modal and its grip should be laid out (got {before:?})"
        );
        let (_, settled) = h.drag(1.0, vec2(STEP, 0.0), 5);
        let dw = settled.modal.width() - before.modal.width();
        assert!(
            (dw - DRAG).abs() <= 4.0,
            "LTR: a {DRAG}-point grip drag must grow the dialog by {DRAG} points \
             (pointer rate), got {dw:+.1} — the pre-fix dialog grew at double \
             pointer rate (before {:?}, after {:?})",
            before.modal,
            settled.modal
        );
        assert!(
            (settled.modal.left() - before.modal.left()).abs() <= 2.0,
            "LTR: the reading-start (left) edge must stay fixed while the user \
             resizes (before {:?}, after {:?})",
            before.modal,
            settled.modal
        );
        assert!(
            (settled.modal.right() - (before.modal.right() + DRAG)).abs() <= 4.0,
            "LTR: the reading-end (right) edge must move by the width delta \
             (before {:?}, after {:?})",
            before.modal,
            settled.modal
        );
        assert!(
            (settled.modal.top() - before.modal.top()).abs() <= 2.0,
            "LTR: a horizontal drag must not move the top edge \
             (before {:?}, after {:?})",
            before.modal,
            settled.modal
        );
    }
}

/// M2: while the grip drag is in flight (no release), each move frame grows the
/// dialog by exactly that frame's pointer delta, and the reported reading-start
/// edge stays constant across the in-flight frames. Mid-drag the reported edge
/// sits one frame's growth above its settled value (the new size is folded
/// through the begin-time left-top), which is itself constant at a constant
/// rate — so the constancy is asserted within the in-flight phase only.
#[test]
fn modal_grip_drag_rate_is_pointer_rate_mid_drag() {
    const STEP: f32 = 10.0;

    for (rtl, step) in [(true, vec2(-STEP, 0.0)), (false, vec2(STEP, 0.0))] {
        let dir = if rtl { "RTL" } else { "LTR" };
        let h = Harness::new(rtl);
        let before = h.settle(0.0);
        let (in_flight, _pointer) = h.press_and_drag(1.0, step, 5);
        assert_eq!(in_flight.len(), 5, "{dir}: expected five in-flight frames");

        // Each move frame grows the modal by that frame's pointer delta:
        let mut prev_width = before.modal.width();
        for (i, snap) in in_flight.iter().enumerate() {
            assert!(
                (snap.modal.width() - prev_width - STEP).abs() <= 2.0,
                "{dir}: move frame {i} must grow the dialog by the {STEP}-point \
                 pointer delta (previous width {prev_width:.1}, got {:.1}) — the \
                 pre-fix dialog grew at double pointer rate mid-drag",
                snap.modal.width()
            );
            prev_width = snap.modal.width();
        }

        // The reading-start edge is constant across the in-flight frames (at
        // base it stepped +10 points per frame — the anchor chased the
        // re-centering):
        let edge_0 = if rtl {
            in_flight[0].modal.right()
        } else {
            in_flight[0].modal.left()
        };
        for (i, snap) in in_flight.iter().enumerate().skip(1) {
            let edge = if rtl {
                snap.modal.right()
            } else {
                snap.modal.left()
            };
            assert!(
                (edge - edge_0).abs() <= 2.0,
                "{dir}: the reading-start edge must stay constant while the drag \
                 is in flight (frame 0 {edge_0:.1}, frame {i} {edge:.1})"
            );
        }
    }
}

/// M3: the pin persists after the drag. On release the settled rect is stable
/// across idle frames (no recentering jump), and a second drag from the new
/// grip position keeps the same reading-start edge fixed at pointer rate — the
/// frozen placement size never re-freezes, so the pin stays on the original
/// reading-start edge.
#[test]
fn modal_stays_put_after_release_and_second_drag() {
    const DRAG: f32 = 50.0;
    const STEP: f32 = 10.0;

    let h = Harness::new(true);
    let _before = h.settle(0.0);
    let (_, first) = h.drag(1.0, vec2(-STEP, 0.0), 5);

    // No recentering jump on release: idle frames leave the settled rect alone.
    let idle1 = h.frame(vec![], 20.0);
    let idle2 = h.frame(vec![], 20.1);
    assert!(
        same_rect(idle1.modal, first.modal) && same_rect(idle2.modal, first.modal),
        "RTL: the settled dialog must stay put after release (settled {:?}, \
         idle1 {idle1:?}, idle2 {idle2:?})",
        first.modal
    );

    // Second drag from the new grip position: the reading-start edge stays
    // fixed and the growth is again pointer rate (at base the edge kept running
    // and the rate stayed double).
    let (in_flight, second) = h.drag(21.0, vec2(-STEP, 0.0), 5);
    assert!(
        (second.modal.right() - first.modal.right()).abs() <= 2.0,
        "RTL: the second drag must keep the same reading-start (right) edge \
         fixed (first {first:?}, second {second:?})"
    );
    let dw = second.modal.width() - first.modal.width();
    assert!(
        (dw - DRAG).abs() <= 4.0,
        "RTL: the second {DRAG}-point drag must grow the dialog by {DRAG} points \
         (pointer rate), got {dw:+.1} (first {first:?}, second {second:?})"
    );
    let mut prev_width = first.modal.width();
    for (i, snap) in in_flight.iter().enumerate() {
        assert!(
            (snap.modal.width() - prev_width - STEP).abs() <= 2.0,
            "RTL: second-drag move frame {i} must grow the dialog by the \
             {STEP}-point pointer delta (previous width {prev_width:.1}, got {:.1})",
            snap.modal.width()
        );
        prev_width = snap.modal.width();
    }
}

/// M4: a vertical grip drag pins the top edge in both directions (the y axis is
/// direction-neutral): the dialog grows downward from the pinned top edge at
/// pointer rate. At base the height doubled (+90 per 50-point drag) and the top
/// edge ran away upward (−45).
#[test]
fn modal_vertical_grip_drag_pins_top_edge() {
    const DRAG: f32 = 50.0;
    const STEP: f32 = 10.0;

    for rtl in [true, false] {
        let dir = if rtl { "RTL" } else { "LTR" };
        let h = Harness::new(rtl);
        let before = h.settle(0.0);
        assert!(
            before.modal.is_positive() && before.grip.is_positive(),
            "{dir}: the modal and its grip should be laid out (got {before:?})"
        );
        let (_, settled) = h.drag(1.0, vec2(0.0, STEP), 5);
        let dh = settled.modal.height() - before.modal.height();
        assert!(
            (dh - DRAG).abs() <= 4.0,
            "{dir}: a {DRAG}-point vertical grip drag must grow the dialog by \
             {DRAG} points (pointer rate), got {dh:+.1} — the pre-fix dialog \
             grew at double pointer rate (before {:?}, after {:?})",
            before.modal,
            settled.modal
        );
        assert!(
            (settled.modal.top() - before.modal.top()).abs() <= 2.0,
            "{dir}: the top edge must stay fixed while the user resizes \
             vertically (before {:?}, after {:?}) — the pre-fix top edge ran away",
            before.modal,
            settled.modal
        );
        assert!(
            (settled.modal.bottom() - (before.modal.bottom() + DRAG)).abs() <= 4.0,
            "{dir}: the bottom edge must move by the height delta \
             (before {:?}, after {:?})",
            before.modal,
            settled.modal
        );
    }
}

/// M5: without a user resize the modal stays centered — content growth
/// re-centers it around the screen center, with both edges moving. This guards
/// the unpinned path (byte-identical to the pre-fix behavior) and that the pin
/// never engages without a drag.
#[test]
fn modal_without_drag_stays_centered() {
    const SCREEN: Vec2 = vec2(700.0, 500.0);

    for rtl in [true, false] {
        let dir = if rtl { "RTL" } else { "LTR" };
        let h = Harness::new(rtl);
        let before = h.settle(0.0);
        assert!(
            before.modal.is_positive(),
            "{dir}: the modal should be laid out (got {before:?})"
        );
        h.wide.set(true);
        let after = h.settle(10.0);
        assert!(
            after.modal.width() > before.modal.width() + 10.0,
            "{dir}: the longer label must grow the dialog (before {:?}, after {:?})",
            before.modal,
            after.modal
        );
        let screen_center = pos2(SCREEN.x / 2.0, SCREEN.y / 2.0);
        for (name, snap) in [("before", before), ("after", after)] {
            let center = snap.modal.center();
            assert!(
                (center.x - screen_center.x).abs() <= 2.0
                    && (center.y - screen_center.y).abs() <= 2.0,
                "{dir}: the modal must stay centered before any user resize \
                 ({name} center {center:?}, screen center {screen_center:?})",
            );
        }
        assert!(
            (after.modal.left() - before.modal.left()).abs() > 1.0
                && (after.modal.right() - before.modal.right()).abs() > 1.0,
            "{dir}: centered growth must move both edges (before {:?}, after {:?})",
            before.modal,
            after.modal
        );
    }
}
