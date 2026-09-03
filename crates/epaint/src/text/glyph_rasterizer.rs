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
