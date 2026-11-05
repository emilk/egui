//! RTL interaction tests for [`egui::TextEdit`], driven through the headless
//! [`egui_kittest::Harness`] with the RTL harness knob
//! ([`egui_kittest::HarnessBuilder::with_right_to_left`]).
//!
//! The caret and selection assertions are structural (cursor indices from the
//! stored [`egui::widgets::text_edit::TextEditState`], x positions from the
//! painted cursor line segments, and glyph positions from the painted text
//! shapes), so they survive engine tweaks and renderers.
//!
//! On this base the cursor indices index the row's visually-ordered glyphs,
//! so Home/End are the visual left/right edges of the text and the arrow keys
//! move the caret visually; the bidi *rendering* order is asserted separately
//! from the painted glyphs (Arabic run right of the Latin run, mirrored
//! brackets, digits reading left-to-right).
//!
//! Assertions that need actual Arabic glyphs are gated on the RTL font fixture
//! ([`egui_kittest::HarnessBuilder::with_rtl_font`], env var `KITTEST_RTL_FONT`)
//! and are skipped with a printed reason when it is not configured — never
//! panicking and never producing a committed snapshot.
//!
//! The bidi-boundary cases port the test checklist of
//! <https://github.com/emilk/egui/pull/8822> to the harness level (see also
//! <https://github.com/emilk/egui/issues/1016>): mixed Arabic/Latin text, Arabic
//! with Arabic-Indic digits and mirrored brackets, digits outside RTL text, hit
//! tests across a direction boundary, and the end caret.

use core::ops::Range;

use egui::accesskit::Role;
use egui_kittest::{Harness, rtl_font_path};
use kittest::Queryable as _;

const EDIT_ID: &str = "rtl_edit";

fn edit_id() -> egui::Id {
    egui::Id::unique(EDIT_ID)
}

/// An RTL harness around a focused singleline `TextEdit` over `text`.
fn focused_rtl_edit(text: &mut String) -> Harness<'_, ()> {
    let mut harness = Harness::builder()
        .with_right_to_left(true)
        .with_rtl_font()
        .with_size(egui::vec2(300.0, 100.0))
        .build_ui(|ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .id(edit_id())
                    .hint_text("Text")
                    .desired_width(240.0),
            );
        });
    harness.get_by_role(Role::TextInput).focus();
    harness.run();
    harness
}

/// Why glyph-dependent assertions were skipped.
#[expect(clippy::print_stderr)] // Tests may run without a logger installed.
fn skipped_without_rtl_font(test: &str) {
    eprintln!(
        "skipped the Arabic-glyph-dependent part of {test}: {KITTEST_RTL_FONT_ENV} is not set. \
         Point it at an Arabic-capable TTF (e.g. Noto Naskh Arabic, Noto Sans Arabic or Amiri, \
         all OFL-licensed) to run them.",
        KITTEST_RTL_FONT_ENV = egui_kittest::RTL_FONT_ENV_VAR,
    );
}

/// The character index (NOT byte index) of the primary cursor.
fn cursor_char_index(harness: &Harness<'_, ()>) -> usize {
    let state = egui::widgets::text_edit::TextEditState::load(&harness.ctx, edit_id())
        .expect("the TextEdit should have stored its state");
    let range = state
        .cursor
        .char_range()
        .expect("the focused TextEdit should have a cursor range");
    range.primary.index.0
}

/// The sorted character range of the current selection.
fn selection_char_range(harness: &Harness<'_, ()>) -> (usize, usize) {
    let state = egui::widgets::text_edit::TextEditState::load(&harness.ctx, edit_id())
        .expect("the TextEdit should have stored its state");
    let range = state
        .cursor
        .char_range()
        .expect("the focused TextEdit should have a cursor range");
    let primary = range.primary.index.0;
    let secondary = range.secondary.index.0;
    (primary.min(secondary), primary.max(secondary))
}

/// The x position of the painted caret (the primary text cursor), if visible.
///
/// The caret is painted as a vertical line segment
/// (see `egui::text_selection::visuals::paint_cursor_end`).
fn caret_x(harness: &Harness<'_, ()>) -> Option<f32> {
    for clipped in &harness.output().shapes {
        if let egui::epaint::Shape::LineSegment { points, .. } = &clipped.shape {
            let [a, b] = points;
            if (a.x - b.x).abs() < 0.5 && (a.y - b.y).abs() > 2.0 {
                return Some(a.x.min(b.x));
            }
        }
    }
    None
}

fn caret_x_expect(harness: &Harness<'_, ()>, what: &str) -> f32 {
    caret_x(harness).unwrap_or_else(|| panic!("the caret should be painted {what}"))
}

/// Press `key` with shift, to move the selection end.
fn shift_press(harness: &mut Harness<'_, ()>, key: egui::Key) {
    harness.key_press_modifiers(egui::Modifiers::SHIFT, key);
    harness.run();
}

/// `caret_x` after pressing `key`.
fn caret_after_key(harness: &mut Harness<'_, ()>, key: egui::Key) -> f32 {
    harness.key_press(key);
    harness.run();
    caret_x_expect(harness, &format!("after pressing {key:?}"))
}

/// Latin caret/selection math under RTL chrome: runs without any Arabic font
/// (the [`with_rtl_font`] degradation protocol).
///
/// [`with_rtl_font`]: egui_kittest::HarnessBuilder::with_rtl_font
#[test]
fn test_text_edit_rtl_caret_and_selection() {
    let mut text = "Hello".to_owned();
    let mut harness = focused_rtl_edit(&mut text);

    assert!(harness.right_to_left(), "the harness should be in RTL mode");
    assert_eq!(
        cursor_char_index(&harness),
        5,
        "gaining focus starts with the cursor at the end of the text"
    );

    // Home/End jump to the two reading edges:
    harness.key_press(egui::Key::Home);
    harness.run();
    assert_eq!(cursor_char_index(&harness), 0, "Home jumps to the start");
    harness.key_press(egui::Key::End);
    harness.run();
    assert_eq!(cursor_char_index(&harness), 5, "End jumps to the end");

    // ArrowRight/ArrowLeft advance/retreat one character, from the end:
    harness.key_press(egui::Key::End);
    harness.run();
    harness.key_press(egui::Key::ArrowLeft);
    harness.run();
    assert_eq!(
        cursor_char_index(&harness),
        4,
        "ArrowLeft retreats one character from the end"
    );
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    assert_eq!(
        cursor_char_index(&harness),
        5,
        "ArrowRight advances one character"
    );

    // Shift+arrows extend the selection in reading order:
    harness.key_press(egui::Key::Home);
    harness.run();
    shift_press(&mut harness, egui::Key::ArrowRight);
    shift_press(&mut harness, egui::Key::ArrowRight);
    assert_eq!(
        selection_char_range(&harness),
        (0, 2),
        "two shift+ArrowRight presses should select the first two characters"
    );
    shift_press(&mut harness, egui::Key::ArrowLeft);
    assert_eq!(
        selection_char_range(&harness),
        (0, 1),
        "shift+ArrowLeft should shrink the selection"
    );

    // Arabic caret math (only with the RTL font fixture installed).
    //
    // On this base the cursor indices index the row's visually-ordered glyphs,
    // so Home/End are the visual left/right edges of the text and the arrow
    // keys move the caret visually (ArrowRight x+, ArrowLeft x-), independent
    // of the reading direction of the runs. The bidi *rendering* order is
    // covered in `test_text_edit_bidi_boundaries`.
    if rtl_font_path().is_some() {
        let mut text = "مصر".to_owned();
        let mut harness = focused_rtl_edit(&mut text);

        // The end index is the row's glyph count, which depends on the fixture
        // font's shaping; only its stability is asserted.
        let end_index = cursor_char_index(&harness);
        assert!(end_index > 0, "the focused cursor should start at the end");

        harness.key_press(egui::Key::Home);
        harness.run();
        assert_eq!(cursor_char_index(&harness), 0, "Home jumps to index 0");
        let home_x = caret_x_expect(&harness, "at Home");

        harness.key_press(egui::Key::End);
        harness.run();
        assert_eq!(
            cursor_char_index(&harness),
            end_index,
            "End returns to the end"
        );
        let end_x = caret_x_expect(&harness, "at End");
        assert!(
            end_x > home_x,
            "End should sit at the visual right edge of the Arabic text: \
             home {home_x:.1}, end {end_x:.1}"
        );

        // The arrows move the caret visually:
        harness.key_press(egui::Key::Home);
        harness.run();
        let x0 = caret_x_expect(&harness, "at Home");
        let x1 = caret_after_key(&mut harness, egui::Key::ArrowRight);
        assert!(
            x1 > x0,
            "ArrowRight should move the caret visually right: {x0:.1} -> {x1:.1}"
        );
        let x0_again = caret_after_key(&mut harness, egui::Key::ArrowLeft);
        assert!(
            (x0_again - x0).abs() < 1.0,
            "ArrowLeft should move back visually left: {x1:.1} -> {x0_again:.1} (was {x0:.1})"
        );

        // The selection extends glyph by glyph from the visual left edge:
        harness.key_press(egui::Key::Home);
        harness.run();
        let home_x = caret_x_expect(&harness, "at Home");
        shift_press(&mut harness, egui::Key::ArrowRight);
        assert_eq!(
            selection_char_range(&harness),
            (0, 1),
            "shift+ArrowRight from Home should select the first glyph"
        );
        let selected_x = caret_x_expect(&harness, "at the selection end");
        assert!(
            selected_x > home_x,
            "the selection should extend visually right: {home_x:.1} -> {selected_x:.1}"
        );
    } else {
        skipped_without_rtl_font("test_text_edit_rtl_caret_and_selection");
    }
}

/// Hit tests, selection across an Arabic↔Latin direction boundary, and the bidi
/// rendering order (Arabic with mirrored brackets and Arabic-Indic digits,
/// digits outside RTL text; the checklist of emilk/egui#8822 ported to the
/// harness level). All cases need Arabic glyphs, so they degrade to a printed
/// skip without the font fixture.
#[test]
fn test_text_edit_bidi_boundaries() {
    if rtl_font_path().is_none() {
        skipped_without_rtl_font("test_text_edit_bidi_boundaries");
        return;
    }

    // Mixed Arabic/Latin in one paragraph. The Latin run reads left-to-right
    // and is painted to the LEFT of the Arabic run, which reads right-to-left
    // from the paragraph's reading start (the visual right edge).
    let mut text = "ملف PDF".to_owned();
    let painted_text = text.clone();
    let mut harness = focused_rtl_edit(&mut text);

    // The end caret and Home at the two visual edges:
    let end_index = cursor_char_index(&harness);
    assert!(end_index > 0, "the focused cursor should start at the end");
    harness.key_press(egui::Key::Home);
    harness.run();
    assert_eq!(cursor_char_index(&harness), 0, "Home jumps to index 0");
    let home_x = caret_x_expect(&harness, "at Home");
    harness.key_press(egui::Key::End);
    harness.run();
    assert_eq!(
        cursor_char_index(&harness),
        end_index,
        "End returns to the end"
    );
    let end_x = caret_x_expect(&harness, "at the end caret");
    assert!(
        end_x > home_x,
        "the end caret should sit at the visual right edge: home {home_x:.1}, end {end_x:.1}"
    );

    // Bidi rendering: the Latin run left of the Arabic run, reading
    // left-to-right within the run:
    // The harness keeps `&mut text` borrowed for its whole lifetime, so the
    // glyph lookup needs its own copy, taken before the harness was built:
    let glyphs = painted_glyphs(&harness, &painted_text);
    let latin_xs: Vec<f32> = glyphs
        .iter()
        .filter(|(chr, _)| chr.is_ascii_alphabetic())
        .map(|(_, x)| *x)
        .collect();
    let arabic_xs: Vec<f32> = glyphs
        .iter()
        .filter(|(chr, _)| is_arabic_letter(*chr))
        .map(|(_, x)| *x)
        .collect();
    assert!(
        latin_xs.len() >= 3 && arabic_xs.len() >= 3,
        "both runs should be painted: latin {latin_xs:?}, arabic {arabic_xs:?}"
    );
    assert!(
        arabic_xs.iter().copied().fold(f32::INFINITY, f32::min)
            > latin_xs.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        "the Arabic run should be painted to the RIGHT of the Latin run in an \
         RTL paragraph: latin {latin_xs:?}, arabic {arabic_xs:?}"
    );
    for pair in latin_xs.windows(2) {
        assert!(
            pair[0] < pair[1],
            "the Latin run should read left-to-right: {latin_xs:?}"
        );
    }

    // Hit test: clicking at the visual LEFT edge lands in the Latin run
    // (the glyph indices of the first Latin characters):
    let edit_rect = harness.get_by_role(Role::TextInput).rect();
    let click = egui::pos2(edit_rect.left() + 6.0, edit_rect.center().y);
    harness.event(egui::Event::PointerMoved(click));
    harness.event(egui::Event::PointerButton {
        pos: click,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
    harness.event(egui::Event::PointerButton {
        pos: click,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
    let hit = cursor_char_index(&harness);
    assert!(
        hit <= 5,
        "clicking the left edge should land on the Latin side of the boundary, got index {hit}"
    );

    // A selection spanning the direction boundary:
    harness.key_press(egui::Key::Home);
    harness.run();
    for _ in 0..5 {
        shift_press(&mut harness, egui::Key::ArrowRight);
    }
    assert_eq!(
        selection_char_range(&harness),
        (0, 5),
        "five shift+ArrowRight presses from Home should span the Latin run \
         and cross into the Arabic run"
    );

    // Arabic with mirrored brackets and Arabic-Indic digits:
    // the opening bracket renders (mirrored) at the visual RIGHT reading
    // start, and the digits of ٢٠٢٦ read left-to-right inside the number.
    let mut text = "(ملف ٢٠٢٦)".to_owned();
    let painted_text = text.clone();
    let harness = focused_rtl_edit(&mut text);
    // The harness keeps `&mut text` borrowed for its whole lifetime, so the
    // glyph lookup needs its own copy, taken before the harness was built:
    let glyphs = painted_glyphs(&harness, &painted_text);
    let opening_x = max_x_of(&glyphs, |chr| chr == '(');
    let closing_x = max_x_of(&glyphs, |chr| chr == ')');
    assert!(
        opening_x > closing_x,
        "the opening bracket should be mirrored to the visual right in an RTL \
         paragraph: ( at {opening_x:.1}, ) at {closing_x:.1}"
    );
    let digit_xs: Vec<f32> = glyphs
        .iter()
        .filter(|(chr, _)| '٠' <= *chr && *chr <= '٩')
        .map(|(_, x)| *x)
        .collect();
    assert_eq!(
        digit_xs.len(),
        4,
        "the four digits of ٢٠٢٦ should be painted once each: {digit_xs:?}"
    );
    for pair in digit_xs.windows(2) {
        assert!(
            pair[0] < pair[1],
            "the digits of ٢٠٢٦ should read left-to-right: {digit_xs:?}"
        );
    }

    // Arabic-Indic digits outside RTL text (all-LTR content in an RTL UI):
    // painted in logical (left-to-right) order.
    let mut text = "Done ٢٠٢٦".to_owned();
    let painted_text = text.clone();
    let harness = focused_rtl_edit(&mut text);
    // The harness keeps `&mut text` borrowed for its whole lifetime, so the
    // glyph lookup needs its own copy, taken before the harness was built:
    let glyphs = painted_glyphs(&harness, &painted_text);
    let xs: Vec<f32> = glyphs.iter().map(|(_, x)| *x).collect();
    for pair in xs.windows(2) {
        assert!(
            pair[0] < pair[1],
            "LTR-only content should read left-to-right even in an RTL UI: {xs:?}"
        );
    }
}

/// The (chr, galley-local x) of the painted glyphs of `text`, in glyph order.
fn painted_glyphs(harness: &Harness<'_, ()>, text: &str) -> Vec<(char, f32)> {
    for clipped in &harness.output().shapes {
        if let egui::epaint::Shape::Text(text_shape) = &clipped.shape
            && text_shape.galley.text() == text
        {
            return text_shape
                .galley
                .rows
                .iter()
                .flat_map(|row| row.glyphs.iter().map(|glyph| (glyph.chr, glyph.pos.x)))
                .collect();
        }
    }
    Vec::new()
}

fn is_arabic_letter(chr: char) -> bool {
    ('\u{0621}'..='\u{064A}').contains(&chr)
}

fn max_x_of(glyphs: &[(char, f32)], predicate: impl Fn(char) -> bool) -> f32 {
    glyphs
        .iter()
        .filter(|glyph| predicate(glyph.0))
        .fold(f32::NEG_INFINITY, |acc, glyph| acc.max(glyph.1))
}

/// RTL mirror of `tests.rs::test_ime_composition_visuals`: Arabic preedit in a
/// focused RTL `TextEdit`, asserting the preedit render and the per-stretch
/// active-range underline geometry structurally (from paint shapes).
/// No snapshot is taken: the rendering depends on the `KITTEST_RTL_FONT` fixture.
#[test]
fn test_ime_composition_visuals_rtl() {
    // Latin preedit: structural, runs without the RTL font fixture.
    ime_preedit_case("Hello. ".to_owned(), "salam", Some(0..2), "latin");

    if rtl_font_path().is_some() {
        // Arabic preedit with an active range covering its first stretch:
        ime_preedit_case("مرحبا. ".to_owned(), "يا سلام", Some(0..2), "arabic");
    } else {
        skipped_without_rtl_font("test_ime_composition_visuals_rtl (Arabic part)");
    }
}

/// Send a preedit with `active_range` into a focused RTL [`egui::TextEdit`] holding `base`
/// and assert the structural visuals of the composition.
fn ime_preedit_case(base: String, preedit: &str, active_range: Option<Range<usize>>, what: &str) {
    let mut text = base;
    let mut harness = focused_rtl_edit(&mut text);

    harness.event(egui::Event::Ime(egui::ImeEvent::Preedit {
        text: preedit.to_owned(),
        active_range_chars: active_range.clone(),
    }));
    harness.run();

    let edit_rect = harness.get_by_role(Role::TextInput).rect();

    // The preedit text is painted inside the TextEdit:
    let mut preedit_painted = false;
    for clipped in &harness.output().shapes {
        if let egui::epaint::Shape::Text(text_shape) = &clipped.shape
            && text_shape.galley.text().contains(preedit)
        {
            preedit_painted = true;
        }
    }
    assert!(preedit_painted, "{what}: preedit text should be painted");

    // The underline stretches: horizontal line segments under the composition,
    // coloured with the active or the inactive IME underline stroke.
    let ime = harness.ctx.global_style().visuals.ime_composition;
    let mut active_underlines = Vec::new();
    let mut inactive_underlines = Vec::new();
    for clipped in &harness.output().shapes {
        if let egui::epaint::Shape::LineSegment { points, stroke } = &clipped.shape {
            let [a, b] = points;
            let horizontal = (a.y - b.y).abs() < 0.5 && (a.x - b.x).abs() > 1.0;
            let inside_edit = edit_rect.contains(*a) && edit_rect.contains(*b);
            if horizontal && inside_edit {
                if stroke.color == ime.active_underline_stroke.color {
                    active_underlines.push((a.x.min(b.x), a.x.max(b.x)));
                } else if stroke.color == ime.inactive_underline_stroke.color {
                    inactive_underlines.push((a.x.min(b.x), a.x.max(b.x)));
                }
            }
        }
    }

    match active_range {
        // A non-empty active range: the active stretch is underlined with the
        // active stroke and is narrower than the whole composition.
        Some(range) if range.start < range.end => {
            assert!(
                !active_underlines.is_empty(),
                "{what}: the active stretch should be underlined with the active stroke"
            );
            assert!(
                !inactive_underlines.is_empty(),
                "{what}: the inactive stretches should be underlined with the inactive stroke"
            );
        }
        // An empty active range: the composition caret is shown instead
        // (a vertical line segment inside the composition):
        _ => {
            assert!(
                caret_x(&harness).is_some(),
                "{what}: the composition caret should be painted for an empty active range"
            );
        }
    }
}
