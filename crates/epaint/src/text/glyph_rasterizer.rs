use std::sync::Arc;

use crate::{ColorImage, text::FontFamily};

/// Input to a [`GlyphRasterizer`].
pub struct GlyphRasterizerRequest<'a> {
    /// An unsupported grapheme cluster.
    pub cluster: &'a str,

    /// The requested font family.
    pub family: &'a FontFamily,

    /// Requested font size in physical pixels.
    pub font_size_px: f32,

    /// Horizontal offset from the pixel grid in physical pixels.
    pub subpixel_offset_px: f32,
}

/// A glyph rasterized by a platform fallback.
#[derive(Clone)]
pub struct RasterizedGlyph {
    /// Pixels in physical pixels. Color glyphs retain their original colors.
    pub image: ColorImage,

    /// Offset from the baseline to the image top-left, in physical pixels.
    pub offset_px: emath::Vec2,

    /// Horizontal advance, in physical pixels.
    pub advance_px: f32,

    /// Do not tint this glyph with the text color.
    pub is_color: bool,
}

/// The callback of a [`GlyphRasterizer`].
type RasterizeFn =
    dyn for<'a> Fn(&GlyphRasterizerRequest<'a>) -> Option<RasterizedGlyph> + Send + Sync;

/// Where to look first for the glyphs of a grapheme cluster.
///
/// The other source is used as a fallback if the first one cannot render the cluster.
///
/// See [`GlyphSourcePreference`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GlyphSource {
    /// The fonts in [`FontDefinitions`](crate::text::FontDefinitions).
    ///
    /// Predictable: looks the same everywhere, both on native and on web.
    Fonts,

    /// What the platform offers: fonts from the [`FontProvider`](crate::text::FontProvider)s
    /// (e.g. the system fonts on native), and the [`GlyphRasterizer`] (e.g. the browser on web).
    ///
    /// Supports colored emojis.
    /// Unpredictable: may look different on different computers.
    Platform,
}

/// Decides where to look first for the glyphs of each grapheme cluster.
///
/// Default: [`default_glyph_source`], so that color emoji come from the platform,
/// while text-presentation symbols (e.g. ⏮︎) look the same on all platforms.
///
/// Use `|_| GlyphSource::Fonts` to only use the platform for clusters
/// that no font in [`FontDefinitions`](crate::text::FontDefinitions) can render.
///
/// Set with `egui::Context::set_glyph_source_preference` or [`Fonts::with_glyph_source_preference`](crate::text::Fonts::with_glyph_source_preference).
pub type GlyphSourcePreference = Arc<dyn Fn(&str) -> GlyphSource + Send + Sync>;

/// Rasterizes grapheme clusters using something other than the installed fonts,
/// e.g. the browser on web.
///
/// Used for clusters no installed font can render,
/// and for clusters where the [`GlyphSourcePreference`] says [`GlyphSource::Platform`].
#[derive(Clone)]
pub struct GlyphRasterizer {
    /// Rasterize one grapheme cluster.
    ///
    /// Return `None` if the platform cannot render it either.
    pub rasterize: Arc<RasterizeFn>,
}

impl GlyphRasterizer {
    pub fn new(
        rasterize: impl for<'a> Fn(&GlyphRasterizerRequest<'a>) -> Option<RasterizedGlyph>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            rasterize: Arc::new(rasterize),
        }
    }
}

/// The default [`GlyphSourcePreference`]:
/// [`GlyphSource::Platform`] for clusters with emoji presentation
/// (see [`has_emoji_presentation`]), [`GlyphSource::Fonts`] for everything else.
pub fn default_glyph_source(cluster: &str) -> GlyphSource {
    if has_emoji_presentation(cluster) {
        GlyphSource::Platform
    } else {
        GlyphSource::Fonts
    }
}

impl core::fmt::Debug for GlyphRasterizer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("GlyphRasterizer")
    }
}

/// Does this grapheme cluster have emoji presentation, per Unicode (UTS #51)?
///
/// True for clusters that default to a color glyph (😀, 🦀, 🇸🇪, 👨‍👩‍👧),
/// for clusters with an explicit emoji presentation selector (⏮️, U+FE0F),
/// and for emoji modifier (skin tone) sequences (☝🏻).
///
/// False for text-presentation symbols (⏮, ✔, ♥) and for clusters
/// with an explicit text presentation selector (⏮︎, U+FE0E).
///
/// Used by [`default_glyph_source`].
pub fn has_emoji_presentation(cluster: &str) -> bool {
    use unicode_properties::emoji::{
        EmojiStatus, UnicodeEmoji as _, is_emoji_presentation_selector,
        is_text_presentation_selector,
    };

    if cluster.is_ascii() {
        return false; // Fast path: no ASCII character has emoji presentation.
    }

    let mut has_emoji_presentation = false;
    for c in cluster.chars() {
        if is_text_presentation_selector(c) {
            return false; // Explicit text presentation wins.
        }
        has_emoji_presentation |= is_emoji_presentation_selector(c)
            || matches!(
                c.emoji_status(),
                EmojiStatus::EmojiPresentation
                    | EmojiStatus::EmojiPresentationAndModifierBase
                    | EmojiStatus::EmojiPresentationAndEmojiComponent
                    | EmojiStatus::EmojiPresentationAndModifierAndEmojiComponent
            );
    }
    has_emoji_presentation
}

#[cfg(test)]
mod has_emoji_presentation_tests {
    use super::has_emoji_presentation;

    #[test]
    fn emoji_presentation() {
        for cluster in [
            "😀",
            "🦀",
            "⏮\u{FE0F}",              // explicit emoji presentation
            "☝🏻",                     // skin tone modifier sequence
            "🇸🇪",                     // flag
            "👨\u{200D}👩\u{200D}👧", // ZWJ sequence
            "1\u{FE0F}\u{20E3}",      // keycap
        ] {
            assert!(has_emoji_presentation(cluster), "{cluster:?}");
        }
    }

    #[test]
    fn text_presentation() {
        for cluster in [
            "",
            "a",
            "1",
            "⏮",
            "⏮\u{FE0E}",  // explicit text presentation
            "😀\u{FE0E}", // explicit text presentation wins
            "✔",
            "♥",
            "©",
            "→",
            "√",
        ] {
            assert!(!has_emoji_presentation(cluster), "{cluster:?}");
        }
    }
}
