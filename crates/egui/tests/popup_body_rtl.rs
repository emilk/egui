//! The default popup and tooltip body layout hugs the popup area's reading-start
//! edge: the body rows sit at the left edge in a left-to-right interface and at
//! the right edge in a right-to-left one.
//!
//! This pins the canonicalization contract ([`egui::Context::set_right_to_left`]
//! flips a vertical layout's cross align, so the default
//! `top_down(Align::Min)` body already mirrored); the explicit flag-derived
//! default in [`egui::Popup::new`] is behavior-preserving by design: the
//! assertions describe today's behavior, guarding it.
//!
//! Covered: the plain [`egui::PopupKind::Popup`] path ([`egui::Popup::new`]) and
//! the [`Tooltip`] path ([`egui::PopupKind::Tooltip`], the other [`egui::Popup`]
//! consumer of the default). Not covered: [`egui::Popup::menu`] /
//! [`egui::Popup::context_menu`], which pass their own
//! `top_down_justified` layout explicitly and so never see the default.
//!
//! No snapshot assertions; geometry only, read from the body's own rects.

use egui::{Context, Id, Popup, PopupAnchor, Pos2, RawInput, Rect, Sense, Tooltip, Ui, pos2, vec2};

fn input() -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
        ..Default::default()
    }
}

/// Show an always-open plain [`Popup`] with a wide body and one small row, and
/// return the body `Ui`'s rect and the row's rect.
fn plain_popup_body(right_to_left: bool) -> (Rect, Rect) {
    let ctx = Context::default();
    ctx.set_right_to_left(right_to_left);

    let id = Id::unique("rtl-popup-body-plain");
    let mut body_and_row: Option<(Rect, Rect)> = None;

    let mut show = |ui: &mut Ui| {
        Popup::new(
            id,
            ui.ctx().clone(),
            PopupAnchor::ParentRect(ui.max_rect()),
            ui.layer_id(),
        )
        .show(|ui| {
            ui.set_min_size(vec2(200.0, 140.0));
            let row = ui.label("row");
            body_and_row = Some((ui.max_rect(), row.rect));
        });
    };

    for _ in 0..3 {
        let mut output = ctx.run_ui(input(), &mut show);
        output.textures_delta.clear();
    }

    body_and_row.expect("the popup should have been shown")
}

/// Show an always-open [`Tooltip`] with a wide body and one small row, and
/// return the body `Ui`'s rect and the row's rect.
fn tooltip_body(right_to_left: bool) -> (Rect, Rect) {
    let ctx = Context::default();
    ctx.set_right_to_left(right_to_left);

    let id = Id::unique("rtl-popup-body-tooltip");
    let mut body_and_row: Option<(Rect, Rect)> = None;

    let mut show = |ui: &mut Ui| {
        let response = ui.interact(
            Rect::from_min_size(pos2(300.0, 100.0), vec2(100.0, 20.0)),
            id,
            Sense::hover(),
        );
        // The parent widget's rect is only registered once its pass has run, so
        // the tooltip shows from the second frame on.
        Tooltip::always_open(
            ui.ctx().clone(),
            ui.layer_id(),
            id,
            PopupAnchor::ParentRect(response.rect),
        )
        .show(|ui| {
            ui.set_min_size(vec2(160.0, 60.0));
            let row = ui.label("row");
            body_and_row = Some((ui.max_rect(), row.rect));
        });
    };

    for _ in 0..3 {
        let mut output = ctx.run_ui(input(), &mut show);
        output.textures_delta.clear();
    }

    body_and_row.expect("the tooltip should have been shown")
}

/// The row hugs the body's reading-start edge, in both directions, for both the
/// plain [`Popup`] and the [`Tooltip`].
#[test]
fn popup_and_tooltip_body_rows_hug_the_reading_start_edge() {
    fn case(name: &str, body: Rect, row: Rect) {
        assert!(
            body.width() > 50.0 && row.width() < body.width() - 50.0,
            "{name}: the body should be much wider than the row for this pin to \
             mean anything (body {body:?}, row {row:?})"
        );
    }

    // Plain popup:
    let (body, row) = plain_popup_body(true);
    case("rtl popup", body, row);
    assert!(
        (row.right() - body.right()).abs() < 1.0,
        "RTL: the popup body row should hug the reading-start (right) edge \
         (body {body:?}, row {row:?})"
    );

    let (body, row) = plain_popup_body(false);
    case("ltr popup", body, row);
    assert!(
        (row.left() - body.left()).abs() < 1.0,
        "LTR: the popup body row should hug the reading-start (left) edge \
         (body {body:?}, row {row:?})"
    );

    // Tooltip:
    let (body, row) = tooltip_body(true);
    case("rtl tooltip", body, row);
    assert!(
        (row.right() - body.right()).abs() < 1.0,
        "RTL: the tooltip body row should hug the reading-start (right) edge \
         (body {body:?}, row {row:?})"
    );

    let (body, row) = tooltip_body(false);
    case("ltr tooltip", body, row);
    assert!(
        (row.left() - body.left()).abs() < 1.0,
        "LTR: the tooltip body row should hug the reading-start (left) edge \
         (body {body:?}, row {row:?})"
    );
}
