//! Everything related to text, fonts, text layout, cursors etc.

mod configured_fonts;
pub mod cursor;
mod face_store;
mod family;
mod font_data;
mod font_definitions;
mod font_face;
mod font_id;
mod font_provider;
mod font_tweak;
mod fonts;
mod galley_cache;
mod glyph_atlas;
mod glyph_rasterizer;
mod index;
mod styled_metrics;
mod text_layout;
mod text_layout_types;
mod unicode;

pub use {
    configured_fonts::ConfiguredFonts,
    font_data::{Blob, FontData, FontVariationAxis},
    font_definitions::{FontDefinitions, FontInsert, FontPriority, InsertFontFamily},
    font_id::{FontFamily, FontId},
    font_provider::{FallbackRequest, FontProvider},
    font_tweak::{FontTweak, HintingTarget, SmoothHinting},
    fonts::{Fonts, FontsView, MAX_GLYPH_SIZE},
    glyph_rasterizer::{
        GlyphBitmap, GlyphRasterizer, GlyphRasterizerRequest, RasterizedGlyph,
        has_emoji_presentation,
    },
    index::{ByteIndex, ByteRange, ByteRangeExt, CharIndex, CharRange, CharRangeExt},
    text_layout_types::*,
};

/// Suggested character to use to replace those in password text fields.
pub const PASSWORD_REPLACEMENT_CHAR: char = '•';

/// Controls how we render text
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextOptions {
    /// Maximum size of the font texture.
    pub max_texture_side: usize,

    /// Controls how to convert glyph colors when writing to the font atlas.
    pub color_transfer_function: crate::FontColorTransferFunction,

    /// Whether to enable font hinting
    ///
    /// (round some font coordinates to pixels for sharper text).
    ///
    /// Default is `true`.
    pub font_hinting: bool,

    /// Enable sub-pixel binning for glyphs.
    ///
    /// Sub-pixel binning renders each glyph at up to four fractional horizontal offsets,
    /// giving more even kerning at the cost of more atlas space.
    ///
    /// It also lead to text looking more blurry.
    ///
    /// This is always disabled for CJK characters (which have too many unique glyphs).
    ///
    /// Can be overridden per font with [`FontTweak::subpixel_binning`].
    ///
    /// Default: `true`.
    pub subpixel_binning: bool,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            max_texture_side: 2048, // Small but portable
            color_transfer_function: crate::FontColorTransferFunction::default(),
            font_hinting: true,
            subpixel_binning: true,
        }
    }
}
