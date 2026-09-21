use egui::FontDefinitions;
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
    let mut harness = Harness::builder().allow_missing_glyphs().build_ui(|ui| {
        ui.label(NO_FONT_HAS_THIS);
    });
    harness.run();
}

/// An app that starts without any font (egui without `default_fonts`) and installs its fonts
/// from inside its first pass, as one does in a harness, gets them for that same pass.
#[test]
fn fonts_installed_during_a_pass_apply_to_that_pass_when_none_were_loaded() {
    let mut pass = 0;
    let mut harness = Harness::new_ui(move |ui| {
        pass += 1;
        if pass == 1 {
            // Take away the bundled fonts; applies at the start of the next pass.
            ui.ctx().set_fonts(FontDefinitions::empty());
        } else {
            ui.ctx().set_fonts(FontDefinitions::default());
            ui.label("Laid out in the pass that installed the fonts");
        }
    });
    harness.step();
}
