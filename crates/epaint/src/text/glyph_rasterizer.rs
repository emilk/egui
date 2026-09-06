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

/// A glyph bitmap, ready to be copied into the glyph atlas.
#[derive(Clone)]
pub struct GlyphBitmap {
    /// Pixels in physical pixels. Coverage glyphs are white with alpha;
    /// color glyphs retain their original colors.
    pub image: ColorImage,

    /// Offset from the glyph origin to the image top-left, in physical pixels.
    pub offset_px: emath::Vec2,

    /// A color glyph (e.g. emoji) that must not be tinted with the text color.
    pub is_color: bool,
}

/// A glyph rasterized by a platform fallback.
#[derive(Clone)]
pub struct RasterizedGlyph {
    pub bitmap: GlyphBitmap,

    /// Horizontal advance, in physical pixels.
    pub advance_px: f32,
}

/// The callback of a [`GlyphRasterizer`].
type RasterizeFn =
    dyn for<'a> Fn(&GlyphRasterizerRequest<'a>) -> Option<RasterizedGlyph> + Send + Sync;

/// Rasterizes grapheme clusters using something other than the installed fonts,
/// e.g. the browser on web.
///
/// Used for clusters that no installed font can render,
/// after the [`FontProvider`](crate::text::FontProvider)s have been asked for a font for them.
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
/// Used by `eframe`'s web glyph rasterizer to guess whether a browser-drawn glyph is color.
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
