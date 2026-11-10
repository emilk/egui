//! Regression test for the atom-row direction of vertical-Center bodies: with
//! [`egui::Style::right_to_left`] set, the atom rows of `ui.vertical_centered`
//! bodies (e.g. dialog button rows) read right-to-left, mirroring the way
//! `ui.vertical` (`Align::Min`) bodies and explicit vertical `Align::Max`
//! bodies already do. Every mirrored case also asserts its LTR counterpart, so
//! the file doubles as the LTR-unchanged regression.
//!
//! Geometry only, read from the per-atom rects the widget registers at paint
//! time; no snapshot assertions.

use core::cell::RefCell;

use egui::{Align, Atom, Context, IdSalt, Layout, RawInput, Rect, Ui, Vec2, WidgetAtom, vec2};

const SCREEN: Vec2 = vec2(400.0, 300.0);

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) {
    let mut output = ctx.run_ui(input, run_ui);
    output.textures_delta.clear();
}

/// Allocate and paint a widget with two custom atoms inside `ui`, storing the
/// painted rects of the first (`a`, 20 wide) and second (`b`, 10 wide) atom.
/// Per-atom rects are registered at paint time, not at allocate time.
fn paint_two_atoms(ui: &mut Ui, rects_out: &RefCell<Option<(Rect, Rect)>>) {
    let id_a = IdSalt::new("atom_row_rtl_a");
    let id_b = IdSalt::new("atom_row_rtl_b");
    let prepared = WidgetAtom::new((
        Atom::custom(id_a, vec2(20.0, 10.0)),
        Atom::custom(id_b, vec2(10.0, 10.0)),
    ))
    .allocate(ui);
    let response = prepared.paint(ui);
    if let (Some(a), Some(b)) = (response.rect(id_a), response.rect(id_b)) {
        *rects_out.borrow_mut() = Some((a, b));
    }
}

/// Read the captured atom rects, failing the test when they did not paint.
fn atom_rects(rects: &RefCell<Option<(Rect, Rect)>>) -> (Rect, Rect) {
    rects.borrow().expect("the two atoms should paint")
}

/// Does the first atom sit to the right of the second one?
fn first_is_rightmost((a, b): (Rect, Rect)) -> bool {
    a.left() > b.left()
}

/// Under the interface flag, the first atom of a `ui.vertical_centered` row is
/// the rightmost (the row reads right-to-left); without the flag it stays the
/// leftmost.
#[test]
fn vertical_centered_atom_rows_mirror_in_rtl() {
    for (rtl, want_first_rightmost) in [(false, false), (true, true)] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let rects = RefCell::new(None);
        run(&ctx, input(SCREEN), |ui| {
            ui.vertical_centered(|ui| paint_two_atoms(ui, &rects));
        });
        assert_eq!(
            first_is_rightmost(atom_rects(&rects)),
            want_first_rightmost,
            "the first atom must sit at the reading-start side (rtl = {rtl}: \
             rects {:?})",
            rects.borrow()
        );
    }
}

/// A `ui.vertical` (`Align::Min`) body already mirrors its atom rows under the
/// flag; the vertical-Center change must not disturb it.
#[test]
fn vertical_min_atom_rows_mirror_in_rtl() {
    for (rtl, want_first_rightmost) in [(false, false), (true, true)] {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        let rects = RefCell::new(None);
        run(&ctx, input(SCREEN), |ui| {
            ui.vertical(|ui| paint_two_atoms(ui, &rects));
        });
        assert_eq!(
            first_is_rightmost(atom_rects(&rects)),
            want_first_rightmost,
            "the first atom must sit at the reading-start side (rtl = {rtl}: \
             rects {:?})",
            rects.borrow()
        );
    }
}

/// An explicit vertical `Align::Max` body mirrors its atom rows through the
/// pure-layout term, regardless of the interface flag.
#[test]
fn explicit_vertical_max_body_mirrors_atoms() {
    for flag in [false, true] {
        let ctx = Context::default();
        if flag {
            ctx.set_right_to_left(true);
        }
        let rects = RefCell::new(None);
        run(&ctx, input(SCREEN), |ui| {
            ui.with_layout(Layout::top_down(Align::Max), |ui| {
                paint_two_atoms(ui, &rects);
            });
        });
        assert!(
            first_is_rightmost(atom_rects(&rects)),
            "the first atom must sit at the reading-start side in a vertical \
             Align::Max body (flag = {flag}: rects {:?})",
            rects.borrow()
        );
    }
}

/// Without the flag, only vertical `Align::Max` bodies mirror: `Align::Min` and
/// `Align::Center` stay left-to-right.
#[test]
fn ltr_vertical_bodies_keep_ltr_atom_rows() {
    for align in [Align::Min, Align::Center] {
        let ctx = Context::default();
        let rects = RefCell::new(None);
        run(&ctx, input(SCREEN), |ui| {
            ui.with_layout(Layout::top_down(align), |ui| {
                paint_two_atoms(ui, &rects);
            });
        });
        assert!(
            !first_is_rightmost(atom_rects(&rects)),
            "without the flag the atom row must stay left-to-right \
             (align = {align:?}: rects {:?})",
            rects.borrow()
        );
    }
}
