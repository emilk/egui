//! Wrapping-horizontal labels continue from the reading start under RTL: the
//! first row starts after the previous widget on the line (indented by the
//! leftover space, exactly like the long-standing LTR fast path), later rows
//! hug the container's reading-start edge, and labels whose resolved base
//! direction disagrees with the layout's main direction stay on the plain
//! (else) branch.
//!
//! All assertions measure painted-ink geometry from `glyph.logical_rect()`
//! translated by `text.pos + row.pos` (exact, no raster wobble) and rects
//! captured inside the ui closure — never pixels, never absolute font
//! metrics. Test roles (fail-before vs characterization) are stated on each
//! test so a green-both-sides test is never mistaken for a fail-before.

use egui::epaint::TextShape;
use egui::{CentralPanel, Context, Label, Pos2, RawInput, Rect, Shape, Ui, vec2};

/// The screen size of every test frame.
fn input() -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
        ..Default::default()
    }
}

/// The painted ink rect of every row of a text shape (non-whitespace glyphs),
/// in screen space.
fn row_inks(text: &TextShape) -> Vec<Rect> {
    text.galley
        .rows
        .iter()
        .map(|row| {
            let row_pos = text.pos.to_vec2() + row.pos.to_vec2();
            let mut ink = Rect::NOTHING;
            for glyph in &row.glyphs {
                if glyph.chr.is_whitespace() {
                    continue;
                }
                ink = ink.union(glyph.logical_rect().translate(row_pos));
            }
            ink
        })
        .collect()
}

/// One wrapping-horizontal scenario: a short first label, then a long
/// continuation label inside `allocate_ui` + `horizontal_wrapped`. Returns
/// the container's max rect, the cursor's reading-start-side edge captured
/// after the first label and before the continuation label (exactly what
/// `label.rs`'s own `first_row_indentation` measures against), the
/// continuation label's response rect, and its painted text shape.
fn wrap_case(rtl: bool, first_text: &str, long_text: &str) -> (Rect, Rect, Rect, TextShape) {
    let ctx = Context::default();
    ctx.set_right_to_left(rtl);
    let mut max_rect = Rect::NOTHING;
    let mut cursor = Rect::NOTHING;
    let mut response = Rect::NOTHING;
    let mut text = None;
    // Two passes: the first sizes, the second settles the geometry.
    for pass in 0..2 {
        let mut output = ctx.run_ui(input(), |ui: &mut Ui| {
            CentralPanel::default().show(ui, |ui| {
                ui.allocate_ui(vec2(240.0, 300.0), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        max_rect = ui.max_rect();
                        ui.add(Label::new(first_text));
                        cursor = ui.cursor();
                        response = ui.add(Label::new(long_text)).rect;
                    });
                });
            });
        });
        output.textures_delta.clear();
        if pass == 1 {
            let text_shapes: Vec<&TextShape> = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    Shape::Text(text) => Some(text),
                    _ => None,
                })
                .collect();
            assert!(
                text_shapes.len() >= 2,
                "the first and the continuation label must both paint text"
            );
            text = Some(text_shapes[1].clone());
        }
    }
    (
        max_rect,
        cursor,
        response,
        text.expect("the continuation label must paint text"),
    )
}

/// The space the first label used on line 0: `label.rs`'s own
/// `first_row_indentation` is `available_width −
/// available_size_before_wrap().x`, which under right-to-left equals
/// `max_rect.right() − cursor.right()` and under left-to-right
/// `cursor.left() − max_rect.left()` (the cursor's finite edge is the
/// reading-start side; the other is infinite). The cursor is captured after
/// the first label, before the continuation label.
fn indent_of(rtl: bool, max_rect: Rect, cursor: Rect) -> f32 {
    if rtl {
        max_rect.right() - cursor.right()
    } else {
        cursor.left() - max_rect.left()
    }
}

/// T6 (fail-before): an RTL interface's wrapping-horizontal label continues
/// from the reading start — row 0 starts on line 0 indented by the leftover
/// space, later rows hug the container's reading-start (right) edge, and the
/// response covers the inks without poking past the edge. Red at base: the
/// old LTR-only gate sent the label to the else branch, where no continuation
/// indent exists (clause (ii) measures ≈ 0 ≠ indent).
#[test]
fn rtl_wrap_continues_from_the_reading_start_with_the_first_row_indented() {
    // Chosen widths (recorded per the plan): a 240 px container; the short
    // first label "سلام" leaves most of line 0 free — enough for the
    // continuation's first word, so its row 0 starts on line 0 with the
    // leftover space as its indent; `كلمة كثيرة طويلة ` x 6 wraps well past
    // 3 rows at that width (asserted below). Arabic rides the logical wrap
    // path (recorded bidi runs) — the unit pin is T5b in epaint.
    let long = "كلمة كثيرة طويلة ".repeat(6);
    let (max_rect, cursor, response, text) = wrap_case(true, "سلام", &long);
    let indent = indent_of(true, max_rect, cursor);
    assert!(
        indent > 10.0,
        "the first label must occupy line 0 space, got indent {indent}"
    );
    let inks = row_inks(&text);
    assert!(
        inks.len() >= 3,
        "the continuation label must wrap to at least 3 rows, got {}",
        inks.len()
    );

    let row0_ink_right = inks[0].right();
    for (i, ink) in inks.iter().enumerate() {
        if i == 0 {
            continue;
        }
        // (i) later rows hug the reading-start edge: the ink sits at the
        // galley origin, which the box wobble displaces from the container
        // edge — a wrapped row keeps its break space in its box (its advance
        // is part of the row's kept size), so `galley.rect.right()` pokes
        // that far past the ink and displaces ALL rows equally (the §9
        // trailing-whitespace class; 9 px for the default font's space at
        // 14 px). The ink never pokes PAST the edge:
        let wobble = text.galley.rect.right();
        assert!(
            ink.right() <= max_rect.right() + 2.0,
            "row {i} must not poke past the reading-start edge: ink right {} vs edge {}",
            ink.right(),
            max_rect.right()
        );
        assert!(
            ink.right() >= max_rect.right() - wobble - 2.0,
            "row {i} must hug the reading-start edge (up to the {wobble} px box wobble): ink right {} vs edge {}",
            ink.right(),
            max_rect.right()
        );
        // (ii) THE RED CLAUSE: the indent. The shared paint translation
        // cancels the anchor wobble, so this difference is the indent up to
        // per-row pixel rounding:
        assert!(
            (ink.right() - row0_ink_right - indent).abs() <= 3.0,
            "row {i}: later_row_ink_right {} − row0_ink_right {row0_ink_right} must equal the indent {indent}",
            ink.right()
        );
        // (iii) the response covers the inks and never pokes past the edge:
        assert!(
            response.expand2(vec2(2.0, 2.0)).contains_rect(*ink),
            "row {i} ink {ink:?} must sit inside the response {response:?}"
        );
    }
    assert!(
        response.right() <= max_rect.right() + 2.0,
        "the response must not poke past the reading-start edge: {} vs {}",
        response.right(),
        max_rect.right()
    );
}

/// T7 (LTR twin — byte-identity pin, NOT a fail-before): the LTR fast path
/// is verbatim unchanged — the first row's ink starts `indent` px in from the
/// left edge, later rows hug it. Green at base by construction; any failure
/// is a regression.
#[test]
fn ltr_wrap_is_unchanged() {
    let long = "lorem ipsum dolor ".repeat(5);
    let (max_rect, cursor, response, text) = wrap_case(false, "ol", &long);
    let indent = indent_of(false, max_rect, cursor);
    assert!(indent > 10.0, "the first label must occupy line 0 space");
    let inks = row_inks(&text);
    assert!(inks.len() >= 3, "the continuation label must wrap");
    let row0_ink_left = inks[0].left();
    assert!(
        (row0_ink_left - (max_rect.left() + indent)).abs() <= 2.0,
        "row 0 must start {indent} px in from the left edge: {} vs {}",
        row0_ink_left,
        max_rect.left() + indent
    );
    for (i, ink) in inks.iter().enumerate().skip(1) {
        assert!(
            (ink.left() - max_rect.left()).abs() <= 2.0,
            "row {i} must hug the left edge: {} vs {}",
            ink.left(),
            max_rect.left()
        );
        assert!(
            response.expand2(vec2(2.0, 2.0)).contains_rect(*ink),
            "row {i} ink {ink:?} must sit inside the response {response:?}"
        );
    }
}

/// T8 (fail-before): a pure-Latin label in an RTL interface is stamped RTL
/// (first-strong) and takes the RTL arm — it gains the reading-start indent.
/// Latin records no bidi runs, so this rides the LEGACY wrap path (the unit
/// pin is T5 in epaint). Red at base: the else branch has no indent.
#[test]
fn latin_label_in_an_rtl_interface_takes_the_reading_start_indent() {
    let long = "lorem ipsum dolor ".repeat(5);
    let (max_rect, cursor, _response, text) = wrap_case(true, "ol", &long);
    let indent = indent_of(true, max_rect, cursor);
    assert!(indent > 10.0, "the first label must occupy line 0 space");
    let inks = row_inks(&text);
    assert!(inks.len() >= 2, "the continuation label must wrap");
    let row0_ink_right = inks[0].right();
    for (i, ink) in inks.iter().enumerate().skip(1) {
        assert!(
            (ink.right() - row0_ink_right - indent).abs() <= 3.0,
            "row {i}: later_row_ink_right {} − row0_ink_right {row0_ink_right} must equal the indent {indent}",
            ink.right()
        );
    }
}

/// T9 (characterization — green at base AND after; pins the `base_rtl` key):
/// a digits-only label in an RTL interface resolves no strong direction
/// (first-strong inference finds none), is NOT stamped, and stays on the else
/// branch — giving it the RTL arm would put the indent at its reading END.
/// All rows keep the else branch's uniform right-anchored ink: no indent.
#[test]
fn digits_only_label_in_an_rtl_interface_stays_out_of_the_rtl_arm() {
    let long = "3.14 5.67 8.90 ".repeat(4);
    let (_max_rect, _cursor, _response, text) = wrap_case(true, "ol", &long);
    let inks = row_inks(&text);
    assert!(inks.len() >= 2, "the continuation label must wrap");
    let row0_ink_right = inks[0].right();
    for (i, ink) in inks.iter().enumerate().skip(1) {
        assert!(
            (ink.right() - row0_ink_right).abs() <= 3.0,
            "row {i} must keep the uniform else-branch anchoring: {} vs {row0_ink_right}",
            ink.right()
        );
    }
}

/// T10 (characterization — green at base AND after; pins the reroute): an
/// Arabic label in an LTR interface resolves RTL but the layout's main
/// direction disagrees, so it must NOT take the fast path. It never gets a
/// continuation indent: the else branch never sets `leading_space`, so the
/// epaint threading cannot inset it, and the RIGHT-anchored else branch pins
/// every row's ink right to the same galley x. The rows' absolute x MAY shift
/// between the base and the reroute (the reroute's deliberate behavior
/// change); only the no-indent invariant is pinned here.
#[test]
fn arabic_label_in_an_ltr_interface_wraps_without_the_fast_path() {
    let long = "كلمة كثيرة طويلة ".repeat(4);
    let (max_rect, cursor, _response, text) = wrap_case(false, "ol", &long);
    let indent = indent_of(false, max_rect, cursor);
    assert!(indent > 3.0, "the first label must occupy line 0 space");
    let inks = row_inks(&text);
    assert!(inks.len() >= 2, "the continuation label must wrap");
    let row0_ink_right = inks[0].right();
    for (i, ink) in inks.iter().enumerate().skip(1) {
        assert!(
            (ink.right() - row0_ink_right - indent).abs() > 3.0,
            "row {i} must NOT gain the continuation indent: later − row0 = {} vs indent {indent}",
            ink.right() - row0_ink_right
        );
    }
}

/// T11 (fail-before in clause (ii); clause (i) is a green-both-sides
/// geometry pin): inside T6's setup, row 0's hit rect
/// (`rect_without_leading_space` translated by the paint pos) extends ≈ the
/// indent past row 0's ink on the reading-start side — the hit rect includes
/// the indent gap that the response gives back. Red at base: the else branch
/// anchors row 0 with no advance, so the gap collapses to ≈ the break space
/// (a few px), far below the indent. If a trailing break space rides row 0 in
/// practice it ADDS to the post-fix gap, so the assertion is the one-sided
/// `≥ indent − 3` form.
#[test]
fn rtl_wrap_row_response_rects_track_the_ink() {
    let long = "كلمة كثيرة طويلة ".repeat(6);
    let (max_rect, cursor, _response, text) = wrap_case(true, "سلام", &long);
    let indent = indent_of(true, max_rect, cursor);
    assert!(indent > 10.0, "the first label must occupy line 0 space");
    let inks = row_inks(&text);
    assert!(inks.len() >= 2, "the continuation label must wrap");

    // (i) green both sides (structural): every later row's box rect contains
    // that row's ink, up to 2 px of slack (negative-offset continuation
    // glyphs may poke a few px past the box edge):
    for (i, row) in text.galley.rows.iter().enumerate().skip(1) {
        let box_rect = row.rect().translate(text.pos.to_vec2());
        assert!(
            box_rect.expand2(vec2(2.0, 2.0)).contains_rect(inks[i]),
            "row {i}: the box {box_rect:?} must contain the ink {:?}",
            inks[i]
        );
    }

    // (ii) THE DISCRIMINATING CLAUSE: row 0's hit rect reaches ≈ the indent
    // past its ink on the reading-start side.
    let hit_rect = text.galley.rows[0]
        .rect_without_leading_space()
        .translate(text.pos.to_vec2());
    let gap = hit_rect.right() - inks[0].right();
    assert!(
        gap >= indent - 3.0,
        "row 0's hit rect must extend ≈ the indent ({indent}) past its ink, got gap {gap}"
    );
}
