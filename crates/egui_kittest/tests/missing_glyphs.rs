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

/// Font changes made before any text is laid out in a pass apply to that same pass,
/// even when fonts were already loaded (here: the bundled defaults).
#[test]
fn font_changes_before_any_text_apply_to_the_same_pass() {
    let _harness = Harness::new_ui(|ui| {
        ui.ctx().set_fonts(FontDefinitions::empty());
        assert!(
            ui.ctx()
                .fonts(|fonts| fonts.definitions().font_data.is_empty()),
            "Nothing was laid out yet, so the new fonts should be in use already"
        );
    });
}

/// Once text has been laid out, font changes wait for the next pass.
#[test]
fn font_changes_after_text_wait_for_the_next_pass() {
    let mut pass = 0;
    let _harness = Harness::new_ui(move |ui| {
        pass += 1;
        if pass == 1 {
            ui.label("Laid out with the bundled fonts");
            ui.ctx().set_fonts(FontDefinitions::empty());
            assert!(
                !ui.ctx()
                    .fonts(|fonts| fonts.definitions().font_data.is_empty()),
                "Text was laid out with the current fonts, so they must stay for this pass"
            );
        }
    });
}
