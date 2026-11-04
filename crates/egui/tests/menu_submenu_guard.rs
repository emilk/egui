//! Regression tests for the hidden-row submenu guard: a menu row scrolled out of the
//! visible part of its menu popup (outside its `Ui::clip_rect`) must not open its submenu
//! on hover, while an already-open submenu is never force-closed by the guard.
//!
//! Every test runs in both directions ([`egui::Style::right_to_left`] on and off); the
//! guard is direction-blind (clip rects are physical), so both must behave identically.
//!
//! The open-state probe is [`egui::containers::menu::MenuState::open_item`], captured at
//! the end of the parent popup body via `MenuState::from_ui` — the exact store
//! `SubMenu::show` reads and writes. `Popup::is_id_open` is deliberately not used: the
//! harness's popups are flag-driven (`Popup::open(bool)`), not Memory-driven, so the
//! Memory popup registry is never written and `is_id_open` would always read false.

use core::cell::{Cell, RefCell};
use std::rc::Rc;

use egui::{
    Context, Event, FullOutput, Id, Modifiers, PointerButton, Popup, Pos2, RawInput, Rect,
    ScrollArea, Sense, Ui, Vec2,
    containers::menu::{MenuState, SubMenu},
    pos2, vec2,
};

/// All tests run on this screen.
const SCREEN: Vec2 = vec2(800.0, 600.0);

/// The anchor "title" widget every menu hangs from.
fn title_rect() -> Rect {
    Rect::from_min_size(pos2(40.0, 40.0), vec2(40.0, 20.0))
}

/// Rows in the scrolled menu body.
const ROW_COUNT: usize = 30;

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

/// The rect of the popup's [`egui::Area`], read from memory after the frame that showed
/// it. A popup's first frame is its invisible sizing pass, so geometry must never be read
/// from the frame that opened it. Only used for geometry preconditions; open/closed state
/// is read from the `open_item`/`body_ran` probes.
fn popup_area_rect(ctx: &Context, popup_id: Id) -> Rect {
    ctx.memory(|memory| memory.area_rect(popup_id)).expect(
        "the popup should have been shown; geometry is read after a show pass, \
         never from a sizing pass",
    )
}

/// One menu row as captured on the last body pass. `rect` is the raw (unclipped) widget
/// rect; `interact_rect` is the clip-aware one (`clip_rect ∩ rect`), i.e. the row's
/// intersection with the scroll band.
#[derive(Clone, Copy)]
struct CapturedRow {
    id: Id,
    rect: Rect,
    interact_rect: Rect,
}

/// Headless menu harness: a title widget with a flag-driven `Popup::menu`, whose body is
/// a fixed (optional) header row followed by a `ScrollArea` of [`ROW_COUNT`] menu rows.
struct Harness {
    ctx: Context,
    with_header: bool,
    next_time: Cell<f64>,
    /// Drives `Popup::open` (read at construction each pass). Flipped to `false` by the
    /// harness's emulation of the title's toggle when the title is clicked.
    open_flag: Rc<Cell<bool>>,
    /// Set to true at the top of the parent popup body each pass; cleared by the harness
    /// before every pass, so it reads "did the body run on the last pass".
    body_ran: Rc<Cell<bool>>,
    /// `MenuState::open_item` of the parent menu, captured at the end of the body.
    open_item: Rc<RefCell<Option<Id>>>,
    /// The rows as laid out on the last pass.
    rows: Rc<RefCell<Vec<CapturedRow>>>,
    /// The `ScrollArea`'s vertical scroll offset, read at construction each pass.
    scroll_offset: Rc<Cell<f32>>,
    /// The body Ui's `item_spacing` (the y axis is zeroed after capture, so rows touch).
    item_spacing: Rc<RefCell<Vec2>>,
    popup_id: Rc<RefCell<Option<Id>>>,
}

impl Harness {
    fn new(rtl: bool, with_header: bool) -> Self {
        let ctx = Context::default();
        if rtl {
            ctx.set_right_to_left(true);
        }
        Self {
            ctx,
            with_header,
            next_time: Cell::new(0.0),
            open_flag: Rc::new(Cell::new(false)),
            body_ran: Rc::new(Cell::new(false)),
            open_item: Rc::new(RefCell::new(None)),
            rows: Rc::new(RefCell::new(Vec::new())),
            scroll_offset: Rc::new(Cell::new(0.0)),
            item_spacing: Rc::new(RefCell::new(Vec2::ZERO)),
            popup_id: Rc::new(RefCell::new(None)),
        }
    }

    /// Run one pass: clear the `body_ran` flag, then show the title + popup with the
    /// given input events.
    fn pass(&self, events: Vec<Event>) {
        self.body_ran.set(false);
        let time = self.next_time.get() + 0.1;
        self.next_time.set(time);
        let open_flag = Rc::clone(&self.open_flag);
        let body_ran = Rc::clone(&self.body_ran);
        let open_item = Rc::clone(&self.open_item);
        let rows = Rc::clone(&self.rows);
        let scroll_offset = Rc::clone(&self.scroll_offset);
        let item_spacing = Rc::clone(&self.item_spacing);
        let popup_id = Rc::clone(&self.popup_id);
        let with_header = self.with_header;
        let input = input_at(SCREEN, time, events);
        run(&self.ctx, input, move |ui| {
            let title = ui.allocate_rect(title_rect(), Sense::click());
            // Emulate the title's toggle (the harness's `.open(bool)` shape bypasses
            // `from_toggle_button_response`'s click wiring): a click on the title closes
            // the menu for the following passes.
            if title.clicked() {
                open_flag.set(false);
            }
            let popup = Popup::menu(&title).open(open_flag.get());
            *popup_id.borrow_mut() = Some(popup.get_id());
            popup.show(|ui| {
                body_ran.set(true);
                ui.spacing_mut().item_spacing.y = 0.0;
                // Captured after zeroing, so the guard's `button_rect` expansion
                // (`item_spacing / 2.0`) can be reproduced exactly in preconditions.
                *item_spacing.borrow_mut() = ui.style().spacing.item_spacing;
                if with_header {
                    let _ = ui.button("Header");
                }
                ScrollArea::vertical()
                    .max_height(100.0)
                    .id_salt("submenu-guard-scroll")
                    .vertical_scroll_offset(scroll_offset.get())
                    .show(ui, |ui| {
                        let mut captured = Vec::with_capacity(ROW_COUNT);
                        for i in 0..ROW_COUNT {
                            let response = ui
                                .menu_button(format!("Sub {i}"), |ui| {
                                    let _ = ui.button("Item");
                                })
                                .response;
                            captured.push(CapturedRow {
                                id: response.id,
                                rect: response.rect,
                                interact_rect: response.interact_rect,
                            });
                        }
                        *rows.borrow_mut() = captured;
                    });
                // The capture runs after the rows in the same body pass, so it reads the
                // exact store the rows' `SubMenu::show` calls wrote this pass.
                *open_item.borrow_mut() = MenuState::from_ui(ui, |state, _| state.open_item);
            });
        });
    }

    fn open_flag(&self) -> &Rc<Cell<bool>> {
        &self.open_flag
    }

    fn scroll_offset(&self) -> &Rc<Cell<f32>> {
        &self.scroll_offset
    }

    fn rows(&self) -> Vec<CapturedRow> {
        self.rows.borrow().clone()
    }

    fn open_item(&self) -> Option<Id> {
        *self.open_item.borrow()
    }

    fn item_spacing(&self) -> Vec2 {
        *self.item_spacing.borrow()
    }

    /// The parent popup's area rect (geometry only; see [`popup_area_rect`]).
    fn popup_rect(&self) -> Rect {
        let id = self
            .popup_id
            .borrow()
            .expect("the popup must have been shown at least once");
        popup_area_rect(&self.ctx, id)
    }
}

/// The row straddling the clip band's bottom edge, and the band's bottom. The band edges
/// are derived from the rows' clip-aware `interact_rect`s — never from layout constants:
/// `interact_rect = clip_rect ∩ rect`, and a row's clip rect is the scroll band, so the
/// bottom-straddling row's `interact_rect.bottom()` IS the band's bottom edge.
fn band_bottom_row(rows: &[CapturedRow]) -> (f32, &CapturedRow) {
    let row = rows
        .iter()
        .find(|row| {
            !row.interact_rect.is_negative()
                && row.interact_rect.top() == row.rect.top()
                && row.interact_rect.bottom() < row.rect.bottom()
        })
        .expect(
            "the menu must overflow its scroll band with a row straddling the band's bottom \
         edge for this test to mean anything",
        );
    (row.interact_rect.bottom(), row)
}

/// The row straddling the clip band's top edge, and the band's top (symmetric).
fn band_top_row(rows: &[CapturedRow]) -> (f32, &CapturedRow) {
    let row = rows
        .iter()
        .find(|row| {
            !row.interact_rect.is_negative()
                && row.interact_rect.top() > row.rect.top()
                && row.interact_rect.bottom() == row.rect.bottom()
        })
        .expect(
            "the menu must overflow its scroll band with a row straddling the band's top edge \
         for this test to mean anything",
        );
    (row.interact_rect.top(), row)
}

// ---------------------------------------------------------------------------

/// AC1: a menu row scrolled entirely out of the visible part of its menu (its raw rect
/// lies above the popup) must not open its submenu when the pointer sits where the hidden
/// row would be. Fails pre-fix: the raw rect test opens the phantom submenu.
#[test]
fn hidden_menu_row_does_not_open_its_submenu() {
    for rtl in [false, true] {
        let dir = if rtl { "RTL" } else { "LTR" };
        let h = Harness::new(rtl, false);
        h.scroll_offset().set(300.0);
        h.open_flag().set(true);
        h.pass(vec![]); // invisible sizing pass
        h.pass(vec![]); // show pass

        let popup = h.popup_rect();
        let rows = h.rows();
        // Strict convention: "entirely above" means at least 1 px of clearance.
        let hidden = rows
            .iter()
            .find(|row| row.rect.max.y <= popup.min.y - 1.0)
            .unwrap_or_else(|| {
                panic!(
                    "{dir}: the menu must have a row scrolled entirely above the popup \
                        for this test to mean anything"
                )
            });
        let point = hidden.rect.center();
        assert!(
            hidden.rect.contains(point),
            "{dir}: the hover point must be inside the hidden row's raw rect"
        );
        assert!(
            hidden.interact_rect.is_negative(),
            "{dir}: the row must be entirely outside the clip band (hidden), got {:?}",
            hidden.interact_rect
        );

        h.pass(vec![Event::PointerMoved(point)]);
        // Precondition: the hover must actually register at the off-screen point. egui
        // stores any `PointerMoved` position verbatim (`hover_pos` = `latest_pos`), but if
        // pointer handling ever starts dropping or clamping off-screen positions, the
        // hover would never land on the hidden row and this test would pass vacuously.
        assert_eq!(
            h.ctx.pointer_hover_pos(),
            Some(point),
            "{dir}: the hover must register at the off-screen point"
        );
        for _ in 0..2 {
            h.pass(vec![Event::PointerMoved(point)]);
        }

        assert_eq!(
            h.open_item(),
            None,
            "{dir}: a hidden row must not open its submenu on hover"
        );
        assert!(
            h.body_ran.get(),
            "{dir}: the parent menu must still be open"
        );
    }
}

/// AC1+AC2: a row straddling the visible band edge opens its submenu from its visible
/// half, but not from its hidden half (inside the raw rect, outside the clip band). The
/// hidden half fails pre-fix.
#[test]
fn partially_clipped_menu_row_opens_only_on_its_visible_half() {
    for rtl in [false, true] {
        let dir = if rtl { "RTL" } else { "LTR" };
        // The visible half: the submenu opens (the guard must not over-suppress).
        let h = Harness::new(rtl, false);
        h.scroll_offset().set(250.0);
        h.open_flag().set(true);
        h.pass(vec![]); // invisible sizing pass
        h.pass(vec![]); // show pass

        let rows = h.rows();
        let (band_bottom, row) = band_bottom_row(&rows);
        assert!(
            row.rect.top() < band_bottom && band_bottom < row.rect.bottom(),
            "{dir}: the row must straddle the band's bottom edge"
        );
        let visible_point = pos2(row.rect.center().x, band_bottom - 1.0);
        assert!(
            row.rect.contains(visible_point),
            "{dir}: the visible-half point must be inside the row's raw rect"
        );
        assert!(
            row.interact_rect.contains(visible_point),
            "{dir}: the visible-half point must be inside the clip band"
        );

        for _ in 0..2 {
            h.pass(vec![Event::PointerMoved(visible_point)]);
        }

        assert_eq!(
            h.open_item(),
            Some(SubMenu::id_from_widget_id(row.id)),
            "{dir}: the visible half must still open the submenu"
        );

        // The hidden half: same setup in a fresh context, hovered just below the band.
        let h = Harness::new(rtl, false);
        h.scroll_offset().set(250.0);
        h.open_flag().set(true);
        h.pass(vec![]); // invisible sizing pass
        h.pass(vec![]); // show pass

        let rows = h.rows();
        let (band_bottom, row) = band_bottom_row(&rows);
        let hidden_point = pos2(row.rect.center().x, band_bottom + 1.0);
        assert!(
            row.rect.contains(hidden_point),
            "{dir}: the hidden-half point must be inside the row's raw rect (straddle must exceed 1 px)"
        );
        assert!(
            !row.interact_rect.contains(hidden_point),
            "{dir}: the hidden-half point must be outside the clip band"
        );

        for _ in 0..2 {
            h.pass(vec![Event::PointerMoved(hidden_point)]);
        }

        assert_eq!(
            h.open_item(),
            None,
            "{dir}: the hidden half must not open the submenu"
        );
    }
}

/// AC4: with the menu open and scrolled so a hidden row lies over the title, pressing the
/// title must not open the hidden row's submenu, and the title's own toggle still closes
/// the menu. The press frame fails pre-fix (failure mode 2's flash-open).
#[test]
fn pressing_the_title_over_a_hidden_row_does_not_open_a_submenu() {
    for rtl in [false, true] {
        let dir = if rtl { "RTL" } else { "LTR" };
        let h = Harness::new(rtl, false);
        h.scroll_offset().set(305.0);
        h.open_flag().set(true);
        h.pass(vec![]); // invisible sizing pass
        h.pass(vec![]); // show pass

        let title_center = title_rect().center();
        let spacing = h.item_spacing();
        let rows = h.rows();
        let (band_top, _) = band_top_row(&rows);
        let hidden_rows: Vec<&CapturedRow> = rows
            .iter()
            .filter(|row| {
                // The guard hovers the row's raw rect expanded by half the item spacing.
                row.rect.expand2(spacing / 2.0).contains(title_center)
                    && row.rect.max.y < band_top
                    && row.interact_rect.is_negative()
            })
            .collect();
        assert_eq!(
            hidden_rows.len(),
            1,
            "{dir}: exactly one hidden row must lie over the title (got {})",
            hidden_rows.len()
        );

        h.pass(vec![Event::PointerMoved(title_center)]);
        h.pass(vec![button_event(
            title_center,
            PointerButton::Primary,
            true,
        )]);

        assert_eq!(
            h.open_item(),
            None,
            "{dir}: pressing the title over a hidden row must not open a submenu"
        );

        // Release + settle: the harness emulates the title's toggle on the release
        // (click) frame, so the parent menu is closed afterwards.
        h.pass(vec![button_event(
            title_center,
            PointerButton::Primary,
            false,
        )]);
        h.pass(vec![]);
        h.pass(vec![]);

        assert!(
            !h.body_ran.get(),
            "{dir}: the menu must be closed after the title click"
        );
    }
}

/// AC3 pin: an already-open submenu is never force-closed when its row scrolls out of the
/// band (passes pre- and post-fix; the guard's `was_open` clause keeps `is_hovered` raw
/// for the currently-open row).
#[test]
fn an_open_submenu_is_not_closed_by_the_guard() {
    for rtl in [false, true] {
        let dir = if rtl { "RTL" } else { "LTR" };
        let h = Harness::new(rtl, false);
        h.open_flag().set(true);
        h.pass(vec![]); // invisible sizing pass
        h.pass(vec![]); // show pass

        let rows = h.rows();
        let row = &rows[0];
        assert!(
            !row.interact_rect.is_negative() && row.interact_rect == row.rect,
            "{dir}: row 0 must be fully visible before scrolling"
        );
        let point = row.rect.center();
        for _ in 0..2 {
            h.pass(vec![Event::PointerMoved(point)]);
        }
        let expected = SubMenu::id_from_widget_id(row.id);
        assert_eq!(
            h.open_item(),
            Some(expected),
            "{dir}: the submenu must open on hover"
        );

        // Move the pointer out of the menu before scrolling: while the rows shift, the
        // pointer must not sit over another (visible) row, or egui's own hover-switch
        // would close this submenu and open that row's — stock behaviour, not the guard.
        let park = pos2(600.0, 300.0);
        assert!(
            !h.popup_rect().contains(park),
            "{dir}: the parking spot must be outside the popup"
        );
        h.pass(vec![Event::PointerMoved(park)]);

        // Scroll the open row out of the band entirely.
        h.scroll_offset().set(300.0);
        h.pass(vec![]);
        h.pass(vec![]);

        let rows = h.rows();
        let row = &rows[0];
        assert!(
            row.interact_rect.is_negative(),
            "{dir}: row 0 must be entirely outside the clip band after scrolling"
        );

        let point = row.rect.center();
        h.pass(vec![Event::PointerMoved(point)]);
        // Precondition: the hover must actually register at the off-screen point, or the
        // pin below would pass vacuously if pointer handling ever dropped off-screen
        // positions (the guard's `was_open` clause would simply never be exercised).
        assert_eq!(
            h.ctx.pointer_hover_pos(),
            Some(point),
            "{dir}: the hover must register at the off-screen point"
        );
        h.pass(vec![Event::PointerMoved(point)]);

        assert_eq!(
            h.open_item(),
            Some(expected),
            "{dir}: an open submenu must not be closed by the guard when its row scrolls out"
        );
    }
}

/// AC3 clause pin: with a fixed header row above the `ScrollArea`, the clip band is inset
/// within the menu rect, so the hidden open row's rect lies over the header — inside the
/// popup. Hovering it must not close the open submenu. Passes pre- and post-fix, and
/// fails if the `was_open ||` clause is ever dropped: without it, `is_hovered` drops and
/// `hovering_other_menu_entry` closes the submenu.
#[test]
fn an_open_submenus_hidden_row_survives_hovering_over_its_rect_inside_the_menu() {
    for rtl in [false, true] {
        let dir = if rtl { "RTL" } else { "LTR" };
        let h = Harness::new(rtl, true);
        h.open_flag().set(true);
        h.pass(vec![]); // invisible sizing pass
        h.pass(vec![]); // show pass

        let rows = h.rows();
        let row = &rows[0];
        assert!(
            !row.interact_rect.is_negative() && row.interact_rect == row.rect,
            "{dir}: row 0 must be fully visible before scrolling"
        );
        let point = row.rect.center();
        for _ in 0..2 {
            h.pass(vec![Event::PointerMoved(point)]);
        }
        let expected = SubMenu::id_from_widget_id(row.id);
        assert_eq!(
            h.open_item(),
            Some(expected),
            "{dir}: the submenu must open on hover"
        );

        // Move the pointer out of the menu before scrolling: while the rows shift, the
        // pointer must not sit over another (visible) row, or egui's own hover-switch
        // would close this submenu and open that row's — stock behaviour, not the guard.
        let park = pos2(600.0, 300.0);
        assert!(
            !h.popup_rect().contains(park),
            "{dir}: the parking spot must be outside the popup"
        );
        h.pass(vec![Event::PointerMoved(park)]);

        // Scroll just past the row height: row 0 now sits over the header strip, inside
        // the popup but above the clip band.
        h.scroll_offset().set(25.0);
        h.pass(vec![]);
        h.pass(vec![]);

        let rows = h.rows();
        let (band_top, _) = band_top_row(&rows);
        let row = &rows[0];
        assert!(
            row.rect.max.y < band_top,
            "{dir}: row 0 must be entirely above the band"
        );
        assert!(
            row.interact_rect.is_negative(),
            "{dir}: row 0 must be outside the clip band"
        );
        let point = row.rect.center();
        assert!(
            row.rect.contains(point),
            "{dir}: the hover point must be inside row 0's raw rect"
        );
        assert!(
            h.popup_rect().contains(point),
            "{dir}: the hover point must be inside the popup (over the header)"
        );

        for _ in 0..2 {
            h.pass(vec![Event::PointerMoved(point)]);
        }

        assert_eq!(
            h.open_item(),
            Some(expected),
            "{dir}: hovering the open row's hidden rect inside the menu must not close its submenu"
        );
    }
}
