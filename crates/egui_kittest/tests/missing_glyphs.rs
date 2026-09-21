use egui_kittest::Harness;

/// A private-use character no bundled font has a glyph for.
const NO_FONT_HAS_THIS: &str = "\u{10FFFD}";

#[test]
#[should_panic(expected = "(U+10FFFD) in Proportional. Installed fonts: [\"Ubuntu-Light\"")]
fn missing_glyph_panics_by_default() {
    let mut harness = Harness::new_ui(|ui| {
        ui.label(NO_FONT_HAS_THIS);
    });
    harness.run();
}

#[test]
fn missing_glyph_is_allowed_on_request() {
    let mut harness = Harness::builder()
        .allow_missing_glyphs()
        .build_ui(|ui| {
            ui.label(NO_FONT_HAS_THIS);
        });
    harness.run();
}

/// Whitespace and control characters never panic, whatever the fonts have.
#[test]
fn whitespace_is_not_a_missing_glyph() {
    let mut harness = Harness::new_ui(|ui| {
        ui.label(" \t\n\u{00A0}\u{200B}");
    });
    harness.run();
}
