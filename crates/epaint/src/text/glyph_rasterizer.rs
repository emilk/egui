use std::sync::Arc;

use crate::{
    ColorImage,
    text::{FontFamily, FontPriority},
};

/// Input to a [`GlyphRasterizer`].
pub struct GlyphRasterizerRequest<'a> {
    /// The grapheme cluster to rasterize.
    ///
    /// For a fallback rasterizer ([`FontPriority::Lowest`]) this is a cluster no installed font has.
    /// A priority rasterizer ([`FontPriority::Highest`]) is asked about every cluster.
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

/// A glyph rasterized by a [`GlyphRasterizer`].
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
/// e.g. the browser on web, or your own custom glyphs.
///
/// By default ([`FontPriority::Lowest`]) a rasterizer is a fallback:
/// it is only asked about clusters that no installed font can render,
/// after the [`FontProvider`](crate::text::FontProvider)s have been asked for a font for them.
///
/// With [`FontPriority::Highest`] it is instead asked about every cluster,
/// before any font. Use this to override how specific glyphs look,
/// e.g. to always render `…` your own way.
/// Return `None` quickly for everything you do not want to override.
///
/// Several rasterizers can be installed. Within a priority tier
/// they are asked in the order they were added, and the first to return `Some` wins.
///
/// Results (and failures) are cached per cluster, family, and size,
/// so each rasterizer is asked at most once per such combination.
///
/// Give a rasterizer a [`Self::key`] to make installing it idempotent:
/// adding one when a rasterizer with the same key is already installed
/// replaces that one in place instead of appending another.
#[derive(Clone)]
pub struct GlyphRasterizer {
    /// Rasterize one grapheme cluster.
    ///
    /// Return `None` if this rasterizer does not handle it.
    pub rasterize: Arc<RasterizeFn>,

    /// Is this a fallback ([`FontPriority::Lowest`], the default),
    /// or does it override the installed fonts ([`FontPriority::Highest`])?
    pub priority: FontPriority,

    /// Two rasterizers with the same key are the same rasterizer.
    ///
    /// Adding one when a rasterizer with this key is already installed
    /// replaces that one in place, so it is safe to add it every frame.
    /// Without a key, only adding the very same [`Self::rasterize`] again is a no-op.
    pub key: Option<Arc<str>>,
}

impl GlyphRasterizer {
    /// A fallback rasterizer ([`FontPriority::Lowest`]).
    ///
    /// See [`Self::with_priority`].
    pub fn new(
        rasterize: impl for<'a> Fn(&GlyphRasterizerRequest<'a>) -> Option<RasterizedGlyph>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            rasterize: Arc::new(rasterize),
            priority: FontPriority::Lowest,
            key: None,
        }
    }

    /// Should this rasterizer be asked before ([`FontPriority::Highest`])
    /// or after ([`FontPriority::Lowest`]) the installed fonts?
    #[inline]
    pub fn with_priority(mut self, priority: FontPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Identify this rasterizer, so that adding it again replaces it instead of appending.
    ///
    /// See [`Self::key`].
    #[inline]
    pub fn with_key(mut self, key: impl Into<Arc<str>>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Is this the same rasterizer as `other`, so that adding it would change nothing?
    fn is_same_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.rasterize, &other.rasterize) && self.priority == other.priority
    }

    /// Add `self` to `rasterizers`, replacing the one with the same [`Self::key`] if there is one.
    ///
    /// Returns `false` if nothing changed: the same rasterizer was already installed.
    #[doc(hidden)]
    pub fn insert_into(self, rasterizers: &mut Vec<Self>) -> bool {
        let existing = match &self.key {
            Some(key) => rasterizers
                .iter()
                .position(|r| r.key.as_deref() == Some(key.as_ref())),
            None => rasterizers.iter().position(|r| r.is_same_as(&self)),
        };
        if let Some(index) = existing {
            if rasterizers[index].is_same_as(&self) {
                false
            } else {
                rasterizers[index] = self;
                true
            }
        } else {
            rasterizers.push(self);
            true
        }
    }
}

impl core::fmt::Debug for GlyphRasterizer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GlyphRasterizer")
            .field("priority", &self.priority)
            .field("key", &self.key)
            .finish_non_exhaustive()
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
mod insert_tests {
    use super::*;

    fn rasterizer() -> GlyphRasterizer {
        GlyphRasterizer::new(|_: &GlyphRasterizerRequest<'_>| None)
    }

    #[test]
    fn same_key_replaces_in_place() {
        let mut list = Vec::new();
        assert!(rasterizer().with_key("a").insert_into(&mut list));
        assert!(rasterizer().with_key("b").insert_into(&mut list));
        let replacement = rasterizer().with_key("a");
        let replacement_fn = Arc::clone(&replacement.rasterize);
        assert!(replacement.insert_into(&mut list));
        assert_eq!(list.len(), 2);
        assert!(Arc::ptr_eq(&list[0].rasterize, &replacement_fn));
        assert_eq!(list[0].key.as_deref(), Some("a"));
        assert_eq!(list[1].key.as_deref(), Some("b"));
    }

    #[test]
    fn same_rasterizer_is_a_no_op() {
        let mut list = Vec::new();
        let keyed = rasterizer().with_key("a");
        assert!(keyed.clone().insert_into(&mut list));
        assert!(!keyed.insert_into(&mut list));

        let unkeyed = rasterizer();
        assert!(unkeyed.clone().insert_into(&mut list));
        assert!(!unkeyed.clone().insert_into(&mut list));
        // A different priority makes it a different rasterizer:
        assert!(
            unkeyed
                .with_priority(FontPriority::Highest)
                .insert_into(&mut list)
        );
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn unkeyed_rasterizers_append() {
        let mut list = Vec::new();
        assert!(rasterizer().insert_into(&mut list));
        assert!(rasterizer().insert_into(&mut list));
        assert_eq!(list.len(), 2);
    }
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
