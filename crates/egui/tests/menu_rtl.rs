//! Regression tests for popup and menu direction under the whole-UI direction flag
//! ([`egui::Style::right_to_left`]): popup menu bodies mirror, the default anchor
//! hangs from the reading-start edge, submenus open on the reading-direction side,
//! and the screen-edge fallback stays symmetric.
//!
//! Every mirrored case also asserts its LTR counterpart (flag off), so this file
//! doubles as the LTR-unchanged regression. No snapshot assertions; geometry only.

use egui::{
    Context, Event, FullOutput, Id, Modifiers, PointerButton, Popup, Pos2, RawInput, Rect, Sense,
    Ui, Vec2, containers::menu::SubMenu, pos2, vec2,
};

/// All tests run on this screen. Wide enough that the mid-screen and interior
/// pointers of the anchor tests fit their reading-side popup unflipped, and the
/// edge pointers of the fallback tests do not.
const SCREEN: Vec2 = vec2(800.0, 600.0);

fn input_at(size: Vec2, time: f64, events: Vec<Event>) -> RawInput {
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

fn button_event(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// The rect of the popup's [`egui::Area`], read from memory after the frame that
/// showed it. A popup's first frame is its invisible sizing pass, so geometry must
/// never be read from the frame that opened it.
fn popup_area_rect(ctx: &Context, popup_id: Id) -> Rect {
    ctx.memory(|memory| memory.area_rect(popup_id)).expect(
        "the popup should have been shown; geometry is read after a show pass, \
         never from a sizing pass",
    )
}

/// Open a [`Popup::menu`] on a 40x20 anchor widget at (40, 40), run the invisible
/// sizing pass and then the show pass, and return the popup's area rect and the
/// anchor widget's rect as captured on the show pass.
fn shown_menu_popup(rtl: bool, body: &mut dyn FnMut(&mut Ui)) -> (Rect, Rect) {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }

    let mut popup_id = Id::NULL;
    let mut anchor_rect = Rect::NOTHING;

    let mut show = |ui: &mut Ui| {
        let response = ui.allocate_rect(
            Rect::from_min_size(pos2(40.0, 40.0), vec2(40.0, 20.0)),
            Sense::click(),
        );
        let popup = Popup::menu(&response).open(true);
        popup_id = popup.get_id();
        popup.show(|ui| body(ui));
        anchor_rect = response.rect;
    };
    run(&ctx, input_at(SCREEN, 0.0, vec![]), &mut show); // invisible sizing pass
    run(&ctx, input_at(SCREEN, 0.1, vec![]), &mut show); // show pass

    let popup = popup_area_rect(&ctx, popup_id);
    (popup, anchor_rect)
}

// ---------------------------------------------------------------------------

/// A5.1: inside a `Popup::menu` body, a nested `ui.horizontal` row reads
/// right-to-left under RTL (the second member sits to the LEFT of the first) and
/// left-to-right under LTR.
#[test]
fn menu_popup_body_mirrors_rows_under_rtl() {
    fn row_rects(rtl: bool) -> (Rect, Rect) {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }

        let mut first = Rect::NOTHING;
        let mut second = Rect::NOTHING;

        let mut show = |ui: &mut Ui| {
            let response = ui.allocate_rect(
                Rect::from_min_size(pos2(40.0, 40.0), vec2(40.0, 20.0)),
                Sense::click(),
            );
            Popup::menu(&response).open(true).show(|ui| {
                ui.horizontal(|ui| {
                    first = ui.button("Alpha").rect;
                    second = ui.button("Beta").rect;
                });
            });
        };
        run(&ctx, input_at(SCREEN, 0.0, vec![]), &mut show); // invisible sizing pass
        run(&ctx, input_at(SCREEN, 0.1, vec![]), &mut show); // show pass

        (first, second)
    }

    let (rtl_first, rtl_second) = row_rects(true);
    assert!(
        rtl_second.right() < rtl_first.right(),
        "RTL: the second row member must sit to the LEFT of the first \
         (first {rtl_first:?}, second {rtl_second:?})"
    );

    let (ltr_first, ltr_second) = row_rects(false);
    assert!(
        ltr_second.left() > ltr_first.left(),
        "LTR: the second row member must sit to the RIGHT of the first \
         (first {ltr_first:?}, second {ltr_second:?})"
    );
}

/// A5.3: a `Popup::menu` opens below its anchor, hanging from the anchor's
/// reading-end edge ([`egui::RectAlign::BOTTOM_END`] under RTL, `BOTTOM_START`
/// under LTR).
#[test]
fn menu_popup_opens_at_reading_end_under_rtl() {
    // Two short rows keep the popup well inside the screen on both sides, so the
    // default align is used unflipped in both directions:
    let mut rows = |ui: &mut Ui| {
        let _ = ui.button("Alpha");
        let _ = ui.button("Beta");
    };

    let (rtl_popup, rtl_anchor) = shown_menu_popup(true, &mut rows);
    assert!(
        (rtl_popup.max.x - rtl_anchor.max.x).abs() < 2.0,
        "RTL: the popup's reading-end (right) edge should sit at the anchor's \
         reading-end edge (popup {rtl_popup:?}, anchor {rtl_anchor:?})"
    );
    assert!(
        (rtl_popup.top() - rtl_anchor.bottom()).abs() < 2.0,
        "RTL: the popup should hang below the anchor (popup {rtl_popup:?}, \
         anchor {rtl_anchor:?})"
    );

    let (ltr_popup, ltr_anchor) = shown_menu_popup(false, &mut rows);
    assert!(
        (ltr_popup.min.x - ltr_anchor.min.x).abs() < 2.0,
        "LTR: the popup's reading-start (left) edge should sit at the anchor's \
         reading-start edge (popup {ltr_popup:?}, anchor {ltr_anchor:?})"
    );
    assert!(
        (ltr_popup.top() - ltr_anchor.bottom()).abs() < 2.0,
        "LTR: the popup should hang below the anchor (popup {ltr_popup:?}, \
         anchor {ltr_anchor:?})"
    );
}

/// A5.3: a submenu opens on the reading-direction side of its parent row
/// (to the LEFT under RTL, to the RIGHT under LTR). The "Top" button is placed
/// mid-screen with `ui.vertical_centered` so that both reading-end sides have room
/// on screen; that placement is direction-invariant (physical placement mirrors
/// only for vertical cross-`Align::Min` bodies), so "Top" sits at x ≈ 400 in both
/// directions and no popup needs the screen-edge fallback.
#[test]
fn submenu_opens_on_reading_direction_side_under_rtl() {
    fn submenu_rects(rtl: bool) -> (Rect, Rect) {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }

        // Captured through `RefCell`s because they are read while the shared
        // `show` closure (which re-captures them every frame) is still in use:
        let top_rect = core::cell::RefCell::new(Rect::NOTHING);
        // The "Sub" row button's id and rect, captured on parent-menu show frames:
        let sub = core::cell::RefCell::new(None);

        let mut show = |ui: &mut Ui| {
            ui.vertical_centered(|ui| {
                *top_rect.borrow_mut() = ui
                    .menu_button("Top", |ui| {
                        let response = ui
                            .menu_button("Sub", |ui| {
                                _ = ui.button("Item in submenu");
                            })
                            .response;
                        *sub.borrow_mut() = Some((response.id, response.rect));
                    })
                    .response
                    .rect;
            });
        };

        // One frame to lay out the closed menu and find the "Top" button:
        run(&ctx, input_at(SCREEN, 0.0, vec![]), &mut show);
        let top_center = top_rect.borrow().center();

        // The dance, one `run` frame each: hover Top, press Top, release Top (the
        // release click opens the parent menu — that frame is its invisible sizing
        // pass), settle (the parent menu re-runs at its real position, so the Sub
        // row is captured from a parent show frame), hover Sub, press Sub, release
        // Sub (opens the submenu), settle:
        run(
            &ctx,
            input_at(SCREEN, 1.0, vec![Event::PointerMoved(top_center)]),
            &mut show,
        );
        run(
            &ctx,
            input_at(
                SCREEN,
                1.1,
                vec![button_event(top_center, PointerButton::Primary, true)],
            ),
            &mut show,
        );
        run(
            &ctx,
            input_at(
                SCREEN,
                1.2,
                vec![button_event(top_center, PointerButton::Primary, false)],
            ),
            &mut show,
        );
        run(&ctx, input_at(SCREEN, 1.3, vec![]), &mut show);

        let (sub_id, sub_rect) = sub
            .borrow()
            .expect("the parent menu should be open with its Sub row");
        let sub_center = sub_rect.center();

        run(
            &ctx,
            input_at(SCREEN, 1.4, vec![Event::PointerMoved(sub_center)]),
            &mut show,
        );
        run(
            &ctx,
            input_at(
                SCREEN,
                1.5,
                vec![button_event(sub_center, PointerButton::Primary, true)],
            ),
            &mut show,
        );
        run(
            &ctx,
            input_at(
                SCREEN,
                1.6,
                vec![button_event(sub_center, PointerButton::Primary, false)],
            ),
            &mut show,
        );
        run(&ctx, input_at(SCREEN, 1.7, vec![]), &mut show);

        let popup_id = SubMenu::id_from_widget_id(sub_id);
        let popup = popup_area_rect(&ctx, popup_id);
        (popup, sub_rect)
    }

    let (rtl_popup, rtl_sub) = submenu_rects(true);
    assert!(
        rtl_popup.max.x <= rtl_sub.min.x + 1.0,
        "RTL: the submenu should open on the reading-end (left) side of its row \
         (popup {rtl_popup:?}, row {rtl_sub:?})"
    );

    let (ltr_popup, ltr_sub) = submenu_rects(false);
    assert!(
        ltr_popup.min.x >= ltr_sub.max.x - 1.0,
        "LTR: the submenu should open on the reading-direction (right) side of \
         its row (popup {ltr_popup:?}, row {ltr_sub:?})"
    );
}

/// A5.4: a submenu whose rows are drawn inside a vertical `ScrollArea` (how apps
/// bound tall menus to the window) still opens on the reading-direction side of
/// its row and hugs its content width. The scroll area must place and measure
/// itself from the layout cursor on its non-scrolling cross axis: right-anchoring
/// it under RTL leaves an empty span between the parent layout's min-rect seed
/// point and the rect, ballooning the popup's measured width to the whole
/// available span — and a popup that wide can no longer fit on the reading-end
/// side of its row, so it fell back to another alignment.
#[test]
fn submenu_with_a_scroll_area_body_still_opens_on_the_reading_direction_side() {
    fn submenu_rects(rtl: bool) -> (Rect, Rect) {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }

        let top_rect = core::cell::RefCell::new(Rect::NOTHING);
        let sub = core::cell::RefCell::new(None);

        let mut show = |ui: &mut Ui| {
            ui.vertical_centered(|ui| {
                *top_rect.borrow_mut() = ui
                    .menu_button("Top", |ui| {
                        let response = ui
                            .menu_button("Sub", |ui| {
                                // The vertical ScrollArea a menu body bounded by the
                                // window height is drawn in (vertical-only, and without
                                // a bar strip while the rows fit):
                                egui::ScrollArea::vertical()
                                    .id_salt("menu-level")
                                    .max_height(400.0)
                                    .min_scrolled_height(400.0)
                                    .show(ui, |ui| {
                                        _ = ui.button("Item in submenu");
                                    });
                            })
                            .response;
                        *sub.borrow_mut() = Some((response.id, response.rect));
                    })
                    .response
                    .rect;
            });
        };

        // One frame to lay out the closed menu and find the "Top" button:
        run(&ctx, input_at(SCREEN, 0.0, vec![]), &mut show);
        let top_center = top_rect.borrow().center();

        // The dance, one `run` frame each: hover Top, press Top, release Top (the
        // release click opens the parent menu — that frame is its invisible sizing
        // pass), settle (the parent menu re-runs at its real position, so the Sub
        // row is captured from a parent show frame), hover Sub, press Sub, release
        // Sub (opens the submenu), settle:
        run(
            &ctx,
            input_at(SCREEN, 1.0, vec![Event::PointerMoved(top_center)]),
            &mut show,
        );
        run(
            &ctx,
            input_at(
                SCREEN,
                1.1,
                vec![button_event(top_center, PointerButton::Primary, true)],
            ),
            &mut show,
        );
        run(
            &ctx,
            input_at(
                SCREEN,
                1.2,
                vec![button_event(top_center, PointerButton::Primary, false)],
            ),
            &mut show,
        );
        run(&ctx, input_at(SCREEN, 1.3, vec![]), &mut show);

        let (sub_id, sub_rect) = sub
            .borrow()
            .expect("the parent menu should be open with its Sub row");
        let sub_center = sub_rect.center();

        run(
            &ctx,
            input_at(SCREEN, 1.4, vec![Event::PointerMoved(sub_center)]),
            &mut show,
        );
        run(
            &ctx,
            input_at(
                SCREEN,
                1.5,
                vec![button_event(sub_center, PointerButton::Primary, true)],
            ),
            &mut show,
        );
        run(
            &ctx,
            input_at(
                SCREEN,
                1.6,
                vec![button_event(sub_center, PointerButton::Primary, false)],
            ),
            &mut show,
        );
        run(&ctx, input_at(SCREEN, 1.7, vec![]), &mut show);

        let popup_id = SubMenu::id_from_widget_id(sub_id);
        (popup_area_rect(&ctx, popup_id), sub_rect)
    }

    let (rtl_popup, rtl_sub) = submenu_rects(true);
    assert!(
        rtl_popup.max.x <= rtl_sub.min.x + 1.0,
        "RTL: the submenu should open on the reading-end (left) side of its row \
         (popup {rtl_popup:?}, row {rtl_sub:?})"
    );

    let (ltr_popup, ltr_sub) = submenu_rects(false);
    assert!(
        ltr_popup.min.x >= ltr_sub.max.x - 1.0,
        "LTR: the submenu should open on the reading-direction (right) side of \
         its row (popup {ltr_popup:?}, row {ltr_sub:?})"
    );
    // The popup hugs its content in both directions (the scroll area followed the
    // content instead of ballooning to the offered span):
    assert!(
        (rtl_popup.width() - ltr_popup.width()).abs() < 2.0,
        "the scroll-area submenu should measure the same in both directions \
         (RTL {rtl_popup:?}, LTR {ltr_popup:?})"
    );
}

/// Open a context menu by secondary-clicking a full-screen widget at `pointer`,
/// and return the popup's area rect. Frames: hover, secondary press, secondary
/// release, settle, settle (the release frame opens the menu as its invisible
/// sizing pass; the settles let it show at its real position). The 200x140 body
/// plus the popup frame's 6 px margin makes the outer popup ≈212x152, so the
/// interior anchors below fit their reading side unflipped and the edge anchors
/// do not fit their default side at all.
fn context_menu_rect(rtl: bool, pointer: Pos2) -> Rect {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }

    let screen = Rect::from_min_size(Pos2::ZERO, SCREEN);
    let id = Id::unique("rtl-context-menu-root");
    let mut popup_id = Id::NULL;

    let mut show = |ui: &mut Ui| {
        let response = ui.interact(screen, id, Sense::click());
        let popup = Popup::context_menu(&response);
        popup_id = popup.get_id();
        popup.show(|ui| {
            ui.set_min_size(vec2(200.0, 140.0));
        });
    };

    let frames: [Vec<Event>; 5] = [
        vec![Event::PointerMoved(pointer)],
        vec![button_event(pointer, PointerButton::Secondary, true)],
        vec![button_event(pointer, PointerButton::Secondary, false)],
        vec![],
        vec![],
    ];
    for (nr, events) in frames.into_iter().enumerate() {
        run(&ctx, input_at(SCREEN, nr as f64 * 0.1, events), &mut show);
    }

    popup_area_rect(&ctx, popup_id)
}

/// A5.3: a context menu grows from the pointer in the reading direction (to the
/// LEFT under RTL, to the RIGHT under LTR), hanging below the pointer. Each
/// direction gets an interior anchor with room on its reading side, so the
/// default align is used unflipped (a shared anchor near the screen edge would
/// make both directions converge on the same flipped align).
#[test]
fn context_menu_grows_leftward_from_pointer_under_rtl() {
    // RTL at (700, 300): BOTTOM_END fits unflipped, spanning x [488, 700]:
    let rtl = context_menu_rect(true, pos2(700.0, 300.0));
    assert!(
        (rtl.max.x - 700.0).abs() < 2.0,
        "RTL: the popup's reading-start (right) edge should sit at the pointer \
         (popup {rtl:?})"
    );
    assert!(
        (rtl.top() - 300.0).abs() < 2.0,
        "RTL: the popup should hang below the pointer (popup {rtl:?})"
    );

    // LTR at (100, 300): BOTTOM_START fits unflipped, spanning x [100, 312]:
    let ltr = context_menu_rect(false, pos2(100.0, 300.0));
    assert!(
        (ltr.min.x - 100.0).abs() < 2.0,
        "LTR: the popup's reading-start (left) edge should sit at the pointer \
         (popup {ltr:?})"
    );
    assert!(
        (ltr.top() - 300.0).abs() < 2.0,
        "LTR: the popup should hang below the pointer (popup {ltr:?})"
    );
}

/// A5.2: the menu popup's justified width is the rows' natural width, not the
/// offered area width (600x400 by default), in both directions — and identical
/// rows measure to the same width in both. This fails if anyone reintroduces
/// reading-end-anchored sizing on the sizing pass.
#[test]
fn menu_popup_sizing_pass_measures_natural_width_under_rtl() {
    let mut rows = |ui: &mut Ui| {
        let _ = ui.button("Alpha");
        let _ = ui.button("Beta");
    };

    let (rtl_popup, _) = shown_menu_popup(true, &mut rows);
    assert!(
        rtl_popup.width() < 150.0,
        "RTL: the popup should be as wide as its rows, not the offered area width \
         (popup {rtl_popup:?})"
    );

    let (ltr_popup, _) = shown_menu_popup(false, &mut rows);
    assert!(
        ltr_popup.width() < 150.0,
        "LTR: the popup should be as wide as its rows, not the offered area width \
         (popup {ltr_popup:?})"
    );
    assert!(
        (rtl_popup.width() - ltr_popup.width()).abs() <= 2.0,
        "identical rows must measure to the same width in both directions \
         (RTL {rtl_popup:?}, LTR {ltr_popup:?})"
    );
}

/// A5.3/A20.3: near the reading-start screen edge the default side does not fit,
/// so the best-align fallback flips the popup to grow towards the reading end;
/// either way the whole popup stays inside the screen, in both directions.
#[test]
fn context_menu_flips_at_screen_edge_in_both_directions() {
    let screen = Rect::from_min_size(Pos2::ZERO, SCREEN);

    // RTL at (50, 100): BOTTOM_END does not fit, so the fallback flips to
    // rightward growth:
    let rtl = context_menu_rect(true, pos2(50.0, 100.0));
    assert!(
        screen.contains_rect(rtl),
        "RTL: the flipped popup should stay inside the screen (popup {rtl:?})"
    );
    assert!(
        (rtl.min.x - 50.0).abs() < 2.0,
        "RTL: the popup should grow rightward from the reading-start edge \
         (popup {rtl:?})"
    );

    // LTR at (750, 100): BOTTOM_START does not fit, so the fallback flips to
    // leftward growth:
    let ltr = context_menu_rect(false, pos2(750.0, 100.0));
    assert!(
        screen.contains_rect(ltr),
        "LTR: the flipped popup should stay inside the screen (popup {ltr:?})"
    );
    assert!(
        (ltr.max.x - 750.0).abs() < 2.0,
        "LTR: the popup should grow leftward from the reading-end edge \
         (popup {ltr:?})"
    );
}
