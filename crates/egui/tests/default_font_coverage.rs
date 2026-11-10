//! Intentional pin of the documented default font coverage.
//!
//! This file pins the default that the crate docs describe in the
//! "Fonts and scripts" section of `crates/egui/src/lib.rs`: the bundled
//! `default_fonts` stack covers Latin and Greek but has no Arabic or Hebrew
//! glyphs, nothing discovers a font on demand (no [`egui::FontProvider`] is
//! wired by default, so `discovered_fonts()` stays empty on every machine),
//! and a cluster nothing can draw renders as tofu unless the missing-glyph
//! policy is [`egui::MissingGlyphPolicy::Panic`], which panics naming the
//! codepoint and family.
//!
//! **This test passes by design** — it is a regression
//! guard for the documented default, not a probe of an old defect.
//!
//! **Flip rule:** a future change that ships real default coverage (bundled
//! glyphs or a default font provider) must flip this test and the
//! "Fonts and scripts" docs section in the same commit — the first person to
//! add default coverage otherwise meets a failing negative assertion here
//! with no explanation.

use egui::{CentralPanel, Context, FontId, MissingGlyphPolicy, Pos2, RawInput, Rect, Ui, Vec2};

/// A small screen so the labels have room to lay out.
const SIZE: Vec2 = Vec2::new(400.0, 100.0);

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// Run one ui pass in a central panel and discard the texture deltas
/// (these tests never render).
fn run(ctx: &Context, mut body: impl FnMut(&mut Ui)) {
    let mut output = ctx.run_ui(input(SIZE), |ui| {
        CentralPanel::default().show(ui, |ui| body(ui));
    });
    output.textures_delta.clear();
}

/// With [`MissingGlyphPolicy::Panic`] the FIRST cluster nothing can draw
/// panics, naming its codepoint and family. The label is exactly the one
/// character `ة` (U+0629) — the same character that panics the i18n demo
/// app's English test — because a longer string would panic on an earlier
/// codepoint and never reach this one.
#[test]
#[should_panic(expected = "U+0629")]
fn missing_glyph_policy_panic_names_the_char_and_family() {
    let ctx = Context::default();
    ctx.set_missing_glyph_policy(MissingGlyphPolicy::Panic);
    run(&ctx, |ui| {
        ui.label("ة"); // U+0629, Arabic Teh Marbuta
    });
}

/// The `egui` crate's default features are empty (`default = []` in
/// `crates/egui/Cargo.toml`), so a plain `cargo test -p egui` compiles only
/// this module: a bare `egui::Context::default()` has zero fonts and zero
/// providers — even Latin renders as tofu.
#[cfg(not(feature = "default_fonts"))]
mod default_features_bundle_no_fonts {
    use super::*;

    #[test]
    fn bare_context_has_no_fonts_not_even_latin() {
        let ctx = Context::default();
        ctx.set_missing_glyph_policy(MissingGlyphPolicy::Tofu);

        run(&ctx, |ui| {
            ui.fonts_mut(|fonts| {
                assert!(
                    !fonts.has_glyph(&FontId::proportional(14.0), 'A'),
                    "without the `default_fonts` feature even Latin 'A' must have no glyph"
                );
                assert!(
                    !fonts.has_glyph(&FontId::monospace(14.0), 'A'),
                    "without the `default_fonts` feature even Latin 'A' must have no glyph in Monospace"
                );
            });
            assert!(
                ui.fonts(|fonts| fonts.discovered_fonts().is_empty()),
                "no FontProvider is wired by default, so none can have discovered a font"
            );
        });
    }

    #[test]
    fn layout_of_latin_arabic_and_hebrew_as_tofu_without_panicking() {
        let ctx = Context::default();
        ctx.set_missing_glyph_policy(MissingGlyphPolicy::Tofu); // the default policy

        run(&ctx, |ui| {
            ui.label("A مرحبا שלום");
        });
    }
}

/// With the `default_fonts` feature the bundled stack covers Latin and Greek,
/// but no Arabic or Hebrew: run with
/// `cargo test -p egui --features default_fonts`.
#[cfg(feature = "default_fonts")]
mod default_fonts_cover_latin_but_not_arabic_or_hebrew {
    use super::*;

    #[test]
    fn bundled_fonts_cover_latin_and_greek_but_not_arabic_or_hebrew() {
        let ctx = Context::default();
        ctx.set_missing_glyph_policy(MissingGlyphPolicy::Tofu);

        run(&ctx, |ui| {
            ui.fonts_mut(|fonts| {
                // The Greek assertions verify the "Fonts and scripts" docs
                // section's Greek claim: if they fail, fix the docs wording,
                // not this test.
                assert!(fonts.has_glyph(&FontId::proportional(14.0), 'A'));
                assert!(fonts.has_glyph(&FontId::proportional(14.0), 'Ω'));

                assert!(!fonts.has_glyph(&FontId::proportional(14.0), 'ا')); // U+0627
                assert!(!fonts.has_glyph(&FontId::proportional(14.0), 'ة')); // U+0629
                assert!(!fonts.has_glyph(&FontId::proportional(14.0), 'ب')); // U+0628
                assert!(!fonts.has_glyph(&FontId::proportional(14.0), 'א')); // U+05D0

                assert!(fonts.has_glyph(&FontId::monospace(14.0), 'A'));
                assert!(fonts.has_glyph(&FontId::monospace(14.0), 'Ω'));

                assert!(!fonts.has_glyph(&FontId::monospace(14.0), 'ا')); // U+0627
                assert!(!fonts.has_glyph(&FontId::monospace(14.0), 'ة')); // U+0629
                assert!(!fonts.has_glyph(&FontId::monospace(14.0), 'ب')); // U+0628
                assert!(!fonts.has_glyph(&FontId::monospace(14.0), 'א')); // U+05D0
            });
            assert!(
                ui.fonts(|fonts| fonts.discovered_fonts().is_empty()),
                "no FontProvider is wired by default, so none can have discovered a font"
            );
        });
    }

    #[test]
    fn arabic_and_hebrew_layout_as_tofu_without_panicking() {
        let ctx = Context::default();
        ctx.set_missing_glyph_policy(MissingGlyphPolicy::Tofu); // the default policy

        run(&ctx, |ui| {
            ui.label("مرحبا שלום");
        });
    }
}
