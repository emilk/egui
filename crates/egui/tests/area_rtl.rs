//! The default [`egui::Area`] pivot follows the reading direction
//! ([`egui::Style::right_to_left`]): in a right-to-left interface a raw area is
//! rooted at its reading-start edge (`RIGHT_TOP`) and widens leftward, while in
//! a left-to-right interface it is rooted at its left edge (`LEFT_TOP`) and
//! widens rightward, exactly as before. An explicitly set pivot is never
//! second-guessed in either direction, popups and menus keep their own
//! direction-aware placement, and windows stay put frame over frame.
//!
//! Every RTL case has an LTR twin asserting the unchanged behavior. No
//! snapshot assertions; geometry only.

use egui::{
    Align2, Area, Context, Event, Id, Modifiers, PointerButton, Popup, Pos2, RawInput, Rect, Sense,
    Ui, Window, pos2, vec2,
};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))
}

fn input(events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(screen()),
        events,
        ..Default::default()
    }
}

/// Run one ui pass and discard the texture deltas (these tests never render).
fn run(ctx: &Context, events: Vec<Event>, show: &mut dyn FnMut(&mut Ui)) {
    let mut output = ctx.run_ui(input(events), show);
    output.textures_delta.clear();
}

fn pointer_move(pos: Pos2) -> Event {
    Event::PointerMoved(pos)
}

fn pointer_button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// Show a raw area rooted at `root` three times: a discarded settle frame (the
/// sizing pass guesses the size, which may be constrained), then 100 wide, then
/// 200 wide. Returns the settled rect of each recorded frame (`Area::end`
/// corrects the response rect to the measured size in the same frame).
fn growing_area_rects(rtl: bool, root: Pos2) -> [Rect; 2] {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let id = Id::unique("area-rtl-grow");
    let mut rects: [Option<Rect>; 2] = [None, None];
    for (frame, width) in (0..3).zip([None, Some(100.0), Some(200.0)]) {
        run(&ctx, vec![], &mut |ui| {
            let response = Area::new(id).fixed_pos(root).show(ui.ctx(), |ui| {
                if let Some(width) = width {
                    ui.set_min_size(vec2(width, 50.0));
                }
            });
            if frame > 0 {
                rects[frame - 1] = Some(response.response.rect);
            }
        });
    }
    rects.map(|rect| rect.expect("every recorded frame should have run"))
}

/// T1: under RTL the raw area is rooted at its reading-start (right) edge and
/// widens leftward when its content grows.
#[test]
fn rtl_default_area_widens_leftward_from_the_reading_start_edge() {
    let root = pos2(500.0, 100.0);
    let [narrow, wide] = growing_area_rects(true, root);
    assert!(
        (narrow.right() - root.x).abs() < 0.5 && (wide.right() - root.x).abs() < 0.5,
        "the reading-start (right) edge should stay pinned at the root: {narrow:?} {wide:?}"
    );
    assert!(
        (narrow.left() - (root.x - 100.0)).abs() < 0.5,
        "narrow rect: {narrow:?}"
    );
    assert!(
        (wide.left() - (root.x - 200.0)).abs() < 0.5,
        "the area should widen leftward: {wide:?}"
    );
}

/// T1 twin: under LTR the raw area keeps its left edge pinned and widens
/// rightward, byte-identical to the pre-mirror behavior.
#[test]
fn ltr_default_area_widens_rightward_from_the_left_edge() {
    let root = pos2(500.0, 100.0);
    let [narrow, wide] = growing_area_rects(false, root);
    assert!(
        (narrow.left() - root.x).abs() < 0.5 && (wide.left() - root.x).abs() < 0.5,
        "the left edge should stay pinned at the root: {narrow:?} {wide:?}"
    );
    assert!(
        (wide.right() - (root.x + 200.0)).abs() < 0.5,
        "the area should widen rightward: {wide:?}"
    );
}

/// Show a raw area with an explicit pivot rooted at `root` three times (a
/// discarded settle frame, then 100 wide, then 200 wide) and return the
/// settled rect of each recorded frame.
fn explicit_pivot_rects(rtl: bool, pivot: Align2, root: Pos2) -> [Rect; 2] {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let id = Id::unique("area-rtl-explicit-pivot");
    let mut rects: [Option<Rect>; 2] = [None, None];
    for (frame, width) in (0..3).zip([None, Some(100.0), Some(200.0)]) {
        run(&ctx, vec![], &mut |ui| {
            let response = Area::new(id)
                .pivot(pivot)
                .fixed_pos(root)
                .show(ui.ctx(), |ui| {
                    if let Some(width) = width {
                        ui.set_min_size(vec2(width, 50.0));
                    }
                });
            if frame > 0 {
                rects[frame - 1] = Some(response.response.rect);
            }
        });
    }
    rects.map(|rect| rect.expect("every recorded frame should have run"))
}

/// T2: an explicitly set pivot wins over the direction mirror in both
/// directions. In particular, an explicit `LEFT_TOP` under RTL is honored —
/// popup screen-edge fallbacks rely on being able to pin the left edge.
#[test]
fn explicit_pivot_wins_over_the_direction_mirror() {
    let root = pos2(500.0, 100.0);

    let [narrow, wide] = explicit_pivot_rects(true, Align2::LEFT_TOP, root);
    assert!(
        (narrow.left() - root.x).abs() < 0.5 && (wide.left() - root.x).abs() < 0.5,
        "RTL: an explicit LEFT_TOP should keep the left edge pinned: {narrow:?} {wide:?}"
    );
    assert!(
        (wide.right() - (root.x + 200.0)).abs() < 0.5,
        "RTL: an explicit LEFT_TOP should widen rightward: {wide:?}"
    );

    let [narrow, wide] = explicit_pivot_rects(false, Align2::RIGHT_TOP, root);
    assert!(
        (narrow.right() - root.x).abs() < 0.5 && (wide.right() - root.x).abs() < 0.5,
        "LTR: an explicit RIGHT_TOP should keep the right edge pinned: {narrow:?} {wide:?}"
    );
    assert!(
        (wide.left() - (root.x - 200.0)).abs() < 0.5,
        "LTR: an explicit RIGHT_TOP should widen leftward: {wide:?}"
    );
}

/// The rect of a popup hanging from a widget at `button` (a 200x140 body, so
/// the outer popup is ≈212x152), settled after its invisible sizing pass.
fn popup_rect(rtl: bool, button: Rect) -> Rect {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let button_id = Id::unique("area-rtl-popup-button");
    let mut popup_id = Id::NULL;
    let mut show = |ui: &mut Ui| {
        let response = ui.interact(button, button_id, Sense::click());
        let popup = Popup::from_response(&response);
        popup_id = popup.get_id();
        popup.show(|ui| {
            ui.set_min_size(vec2(200.0, 140.0));
        });
    };
    for _ in 0..4 {
        run(&ctx, vec![], &mut show);
    }
    ctx.memory(|memory| memory.area_rect(popup_id))
        .expect("the popup should be shown")
}

/// T3: a popup keeps its direction-aware placement — hanging below the anchor,
/// at its reading-start edge, in both directions.
#[test]
fn popup_placement_is_drift_free_in_both_directions() {
    let button = Rect::from_min_size(pos2(300.0, 100.0), vec2(80.0, 24.0));

    let rtl = popup_rect(true, button);
    assert!(
        (rtl.right() - button.right()).abs() < 0.5,
        "RTL: the popup should hang from the reading-start (right) edge of the button: {rtl:?}"
    );
    assert!(
        (rtl.top() - button.bottom()).abs() < 0.5,
        "RTL: the popup should hang below the button: {rtl:?}"
    );

    let ltr = popup_rect(false, button);
    assert!(
        (ltr.left() - button.left()).abs() < 0.5,
        "LTR: the popup should hang from the reading-start (left) edge of the button: {ltr:?}"
    );
    assert!(
        (ltr.top() - button.bottom()).abs() < 0.5,
        "LTR: the popup should hang below the button: {ltr:?}"
    );
}

/// The rect of a menu opened by clicking a widget at `button`, settled after
/// its invisible sizing pass.
fn menu_rect(rtl: bool, button: Rect) -> Rect {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let button_id = Id::unique("area-rtl-menu-button");
    let mut popup_id = Id::NULL;
    let mut show = |ui: &mut Ui| {
        let response = ui.interact(button, button_id, Sense::click());
        let popup = Popup::menu(&response);
        popup_id = popup.get_id();
        popup.show(|ui| {
            ui.set_min_size(vec2(200.0, 140.0));
        });
    };

    let center = button.center();
    // Frames: hover (a press on the very first frame finds no hovered widget),
    // press, release (opens the menu as its invisible sizing pass), then two
    // settles so it shows at its real position.
    for events in [
        vec![pointer_move(center)],
        vec![pointer_move(center), pointer_button(center, true)],
        vec![pointer_move(center), pointer_button(center, false)],
        vec![],
        vec![],
    ] {
        run(&ctx, events, &mut show);
    }
    ctx.memory(|memory| memory.area_rect(popup_id))
        .expect("the menu should be shown")
}

/// T3: a menu keeps its direction-aware placement — hanging below the button,
/// at its reading-start edge, in both directions.
#[test]
fn menu_placement_is_drift_free_in_both_directions() {
    let button = Rect::from_min_size(pos2(300.0, 100.0), vec2(80.0, 24.0));

    let rtl = menu_rect(true, button);
    assert!(
        (rtl.right() - button.right()).abs() < 0.5,
        "RTL: the menu should hang from the reading-start (right) edge of the button: {rtl:?}"
    );
    assert!(
        (rtl.top() - button.bottom()).abs() < 0.5,
        "RTL: the menu should hang below the button: {rtl:?}"
    );

    let ltr = menu_rect(false, button);
    assert!(
        (ltr.left() - button.left()).abs() < 0.5,
        "LTR: the menu should hang from the reading-start (left) edge of the button: {ltr:?}"
    );
    assert!(
        (ltr.top() - button.bottom()).abs() < 0.5,
        "LTR: the menu should hang below the button: {ltr:?}"
    );
}

/// The rects of a window at `root` over three frames (resizable with a fixed
/// content size, so the window hugs its content instead of keeping the frame-0
/// sizing guess; a `resizable(false)` window keeps the guessed width —
/// pre-existing behavior, not the pivot mirror's).
fn window_rects(rtl: bool, root: Pos2) -> [Rect; 3] {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let mut rects = [Rect::NOTHING; 3];
    for rect in &mut rects {
        run(&ctx, vec![], &mut |ui| {
            let response = Window::new("area-rtl-window")
                .current_pos(root)
                .resizable(true)
                .default_size(vec2(200.0, 100.0))
                .show(ui.ctx(), |ui| {
                    ui.set_min_size(vec2(200.0, 100.0));
                });
            *rect = response.expect("the window should be shown").response.rect;
        });
    }
    rects
}

/// T3: a window is drift-free frame over frame in both directions, rooted at
/// its reading-start edge under RTL and at its left edge under LTR.
#[test]
fn window_placement_is_drift_free_in_both_directions() {
    let root = pos2(500.0, 100.0);

    let [first, second, third] = window_rects(true, root);
    assert_eq!(
        second, third,
        "RTL: the window should not drift between frames: {second:?} {third:?}"
    );
    assert!(
        (second.right() - root.x).abs() < 0.5,
        "RTL: the window should be rooted at its reading-start (right) edge: {second:?}"
    );
    let _ = first; // the sizing pass guesses the size; only frames 2+ are settled

    let [first, second, third] = window_rects(false, root);
    assert_eq!(
        second, third,
        "LTR: the window should not drift between frames: {second:?} {third:?}"
    );
    assert!(
        (second.left() - root.x).abs() < 0.5,
        "LTR: the window should be rooted at its left edge: {second:?}"
    );
    let _ = first;
}

/// T4: two simultaneously-visible raw areas cascade without overlapping, and
/// the second one aligns with the first one's reading-start edge.
#[test]
fn two_automatic_areas_cascade_without_overlap_in_both_directions() {
    fn rects(rtl: bool) -> (Rect, Rect) {
        let ctx = Context::default();
        ctx.set_right_to_left(rtl);
        let first_id = Id::unique("area-rtl-cascade-first");
        let second_id = Id::unique("area-rtl-cascade-second");
        let mut first = Rect::NOTHING;
        let mut second = Rect::NOTHING;
        run(&ctx, vec![], &mut |ui| {
            let response = Area::new(first_id).show(ui.ctx(), |ui| {
                ui.set_min_size(vec2(100.0, 50.0));
            });
            first = response.response.rect;
        });
        run(&ctx, vec![], &mut |ui| {
            let response = Area::new(first_id).show(ui.ctx(), |ui| {
                ui.set_min_size(vec2(100.0, 50.0));
            });
            first = response.response.rect;
            let response = Area::new(second_id).show(ui.ctx(), |ui| {
                ui.set_min_size(vec2(100.0, 50.0));
            });
            second = response.response.rect;
        });
        (first, second)
    }

    let (first, second) = rects(false);
    assert!(
        !second.intersects(first),
        "LTR: the areas should not overlap: {first:?} {second:?}"
    );
    assert!(
        (second.top() - (first.bottom() + 16.0)).abs() < 0.5,
        "LTR: the second area should stack below the first: {first:?} {second:?}"
    );

    let (first, second) = rects(true);
    assert!(
        !second.intersects(first),
        "RTL: the areas should not overlap: {first:?} {second:?}"
    );
    assert!(
        (second.top() - (first.bottom() + 16.0)).abs() < 0.5,
        "RTL: the second area should stack below the first: {first:?} {second:?}"
    );
    assert!(
        (second.right() - first.right()).abs() < 0.5,
        "RTL: the second area should align with the first one's reading-start (right) edge: \
         {first:?} {second:?}"
    );
}
