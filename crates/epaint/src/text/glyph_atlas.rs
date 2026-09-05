use emath::{NumExt as _, Vec2, vec2};
use skrifa::GlyphId;

use crate::{
    ColorImage, FontColorTransferFunction, ImageDelta, TextOptions, TextureAtlas,
    text::{
        FontFamily, GlyphRasterizer, GlyphRasterizerRequest, MAX_GLYPH_SIZE,
        font::{FontFace, ShapedGlyph},
        fonts::FontFaceKey,
        styled_metrics::StyledMetrics,
    },
};

/// Where a glyph ended up in the font atlas, in UV coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct UvRect {
    /// X/Y offset for nice rendering (unit: points).
    pub offset: Vec2,

    /// Screen size (in points) of this glyph.
    /// Note that the height is different from the font height.
    pub size: Vec2,

    /// Top left corner UV in texture.
    pub min: [u16; 2],

    /// Bottom right corner (exclusive).
    pub max: [u16; 2],
}

impl UvRect {
    pub fn is_nothing(&self) -> bool {
        self.min == self.max
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlyphAllocation {
    /// UV rectangle for drawing.
    pub uv_rect: UvRect,
}

/// A glyph from the [`GlyphRasterizer`], allocated in the atlas.
#[derive(Clone, Copy)]
pub(crate) struct RasterGlyphAllocation {
    pub allocation: GlyphAllocation,
    pub advance_px: f32,
    pub is_color: bool,
}

/// A glyph bitmap, ready to be copied into the atlas.
pub(crate) struct GlyphBitmap {
    /// Physical pixels. Coverage glyphs are white with alpha; color glyphs keep their colors.
    pub image: ColorImage,

    /// Offset from the glyph origin to the top-left of the image, in physical pixels.
    pub offset_px: Vec2,
}

// ----------------------------------------------------------------------------

// Subpixel binning, taken from cosmic-text:
// https://github.com/pop-os/cosmic-text/blob/974ddaed96b334f560b606ebe5d2ca2d2f9f23ef/src/glyph_cache.rs

/// Bin for subpixel positioning of glyphs.
///
/// For accurate glyph positioning, we want to render each glyph at a subpixel coordinate. However, we also want to
/// cache each glyph's bitmap. As a compromise, we bin each subpixel offset into one of four fractional values. This
/// means one glyph can have up to four subpixel-positioned bitmaps in the cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(super) enum SubpixelBin {
    #[default]
    Zero,
    One,
    Two,
    Three,
}

impl SubpixelBin {
    /// Bin the given position and return the new integral coordinate.
    fn new(pos: f32) -> (i32, Self) {
        let trunc = pos as i32;
        let fract = pos - trunc as f32;

        if pos.is_sign_negative() {
            if fract > -0.125 {
                (trunc, Self::Zero)
            } else if fract > -0.375 {
                (trunc - 1, Self::Three)
            } else if fract > -0.625 {
                (trunc - 1, Self::Two)
            } else if fract > -0.875 {
                (trunc - 1, Self::One)
            } else {
                (trunc - 1, Self::Zero)
            }
        } else {
            if fract < 0.125 {
                (trunc, Self::Zero)
            } else if fract < 0.375 {
                (trunc, Self::One)
            } else if fract < 0.625 {
                (trunc, Self::Two)
            } else if fract < 0.875 {
                (trunc, Self::Three)
            } else {
                (trunc + 1, Self::Zero)
            }
        }
    }

    pub fn as_float(&self) -> f32 {
        match self {
            Self::Zero => 0.0,
            Self::One => 0.25,
            Self::Two => 0.5,
            Self::Three => 0.75,
        }
    }
}

// ----------------------------------------------------------------------------

/// Hash of everything that decides what an outline glyph looks like in the atlas.
#[derive(Hash, PartialEq, Eq)]
struct OutlineGlyphKey(u64);

impl nohash_hasher::IsEnabled for OutlineGlyphKey {}

impl OutlineGlyphKey {
    #[inline]
    fn new(
        face_key: FontFaceKey,
        glyph_id: GlyphId,
        metrics: &StyledMetrics,
        bin: SubpixelBin,
    ) -> Self {
        let StyledMetrics {
            pixels_per_point,
            px_scale_factor,
            location_hash,
            ..
        } = *metrics;
        debug_assert!(
            0.0 < pixels_per_point && pixels_per_point.is_finite(),
            "Bad pixels_per_point {pixels_per_point}"
        );
        debug_assert!(
            0.0 < px_scale_factor && px_scale_factor.is_finite(),
            "Bad px_scale_factor: {px_scale_factor}"
        );
        Self(crate::util::hash((
            face_key,
            glyph_id,
            pixels_per_point.to_bits(),
            px_scale_factor.to_bits(),
            bin,
            location_hash,
        )))
    }
}

/// Hash of `(cluster, family, pixels_per_point, font_size)`,
/// so that cache lookups do not allocate.
#[derive(Hash, PartialEq, Eq)]
struct RasterGlyphKey(u64);

impl nohash_hasher::IsEnabled for RasterGlyphKey {}

impl RasterGlyphKey {
    fn new(cluster: &str, family: &FontFamily, pixels_per_point: f32, font_size: f32) -> Self {
        Self(crate::util::hash((
            cluster,
            family,
            pixels_per_point.to_bits(),
            font_size.to_bits(),
        )))
    }
}

// ----------------------------------------------------------------------------

/// The font atlas, plus every cache that points into it.
///
/// Cleared as a unit when the atlas is getting full.
/// Knows nothing about families or fallback chains.
pub(crate) struct GlyphAtlas {
    atlas: TextureAtlas,

    /// Glyphs rendered from font outlines.
    outline_glyphs: nohash_hasher::IntMap<OutlineGlyphKey, GlyphAllocation>,

    /// Glyphs from the [`GlyphRasterizer`].
    ///
    /// `None` means the rasterizer could not handle the cluster.
    raster_glyphs: nohash_hasher::IntMap<RasterGlyphKey, Option<RasterGlyphAllocation>>,
}

impl GlyphAtlas {
    pub fn new(options: TextOptions) -> Self {
        let texture_width = options.max_texture_side.at_most(16 * 1024);
        let initial_height = 32; // Keep initial font atlas small, so it is fast to upload to GPU. This will expand as needed anyways.
        Self {
            atlas: TextureAtlas::new([texture_width, initial_height], options),
            outline_glyphs: Default::default(),
            raster_glyphs: Default::default(),
        }
    }

    #[inline]
    pub fn options(&self) -> &TextOptions {
        self.atlas.options()
    }

    #[inline]
    pub fn atlas(&self) -> &TextureAtlas {
        &self.atlas
    }

    #[inline]
    pub fn take_delta(&mut self) -> Option<ImageDelta> {
        self.atlas.take_delta()
    }

    /// How full is the atlas?
    #[inline]
    pub fn fill_ratio(&self) -> f32 {
        self.atlas.fill_ratio()
    }

    /// Forget every glyph and start over with an empty atlas.
    ///
    /// Anything holding a [`UvRect`] into the old atlas (e.g. a cached galley) is now stale.
    pub fn clear(&mut self) {
        *self = Self::new(*self.atlas.options());
    }

    /// Forget what the [`GlyphRasterizer`] produced, e.g. because it was replaced.
    pub fn clear_raster_glyphs(&mut self) {
        self.raster_glyphs.clear();
    }

    /// Get or render the glyph of `face` for `shaped`.
    ///
    /// Returns the allocation and the rounded horizontal position (in physical pixels).
    pub fn allocate_outline(
        &mut self,
        face_key: FontFaceKey,
        face: &mut FontFace,
        metrics: &StyledMetrics,
        shaped: &ShapedGlyph,
    ) -> (GlyphAllocation, i32) {
        let ShapedGlyph {
            glyph_id,
            h_pos,
            is_cjk,
        } = *shaped;

        if glyph_id == GlyphId::NOTDEF {
            // invisible
            return (GlyphAllocation::default(), h_pos.round() as i32);
        }

        let (h_pos_round, bin) = if face.subpixel_binning() && !is_cjk {
            SubpixelBin::new(h_pos)
        } else {
            // CJK scripts contain a lot of characters and could hog the glyph atlas
            // if we stored 4 subpixel offsets per glyph.
            (h_pos.round() as i32, SubpixelBin::Zero)
        };

        let key = OutlineGlyphKey::new(face_key, glyph_id, metrics, bin);

        let Self {
            atlas,
            outline_glyphs,
            ..
        } = self;
        let alloc = *outline_glyphs.entry(key).or_insert_with(|| {
            face.rasterize_outline(metrics, glyph_id, bin)
                .and_then(|bitmap| {
                    let transfer = atlas.options().color_transfer_function;
                    Self::allocate_bitmap(atlas, &bitmap, metrics.pixels_per_point, transfer)
                })
                .map(|mut uv_rect| {
                    uv_rect.offset.y += metrics.y_offset_in_points;
                    GlyphAllocation { uv_rect }
                })
                .unwrap_or_default()
        });

        (alloc, h_pos_round)
    }

    /// Get or rasterize `cluster` using the platform [`GlyphRasterizer`].
    ///
    /// Failures are cached too, so the (potentially slow) rasterizer
    /// is asked at most once per cluster and size.
    pub fn allocate_raster(
        &mut self,
        rasterizer: &GlyphRasterizer,
        cluster: &str,
        family: &FontFamily,
        pixels_per_point: f32,
        font_size: f32,
    ) -> Option<RasterGlyphAllocation> {
        let key = RasterGlyphKey::new(cluster, family, pixels_per_point, font_size);
        if let Some(allocation) = self.raster_glyphs.get(&key) {
            return *allocation;
        }
        let request = GlyphRasterizerRequest {
            cluster,
            family,
            font_size_px: font_size * pixels_per_point,
            subpixel_offset_px: 0.0,
        };
        let allocation = (rasterizer.rasterize)(&request).and_then(|glyph| {
            // The transfer function assumes white coverage glyphs and discards color,
            // so color glyphs (e.g. emoji) must skip it.
            let transfer = if glyph.is_color {
                FontColorTransferFunction::Off
            } else {
                self.atlas.options().color_transfer_function
            };
            let bitmap = GlyphBitmap {
                image: glyph.image,
                offset_px: glyph.offset_px,
            };
            let uv_rect =
                Self::allocate_bitmap(&mut self.atlas, &bitmap, pixels_per_point, transfer)?;
            Some(RasterGlyphAllocation {
                allocation: GlyphAllocation { uv_rect },
                advance_px: glyph.advance_px,
                is_color: glyph.is_color,
            })
        });
        self.raster_glyphs.insert(key, allocation);
        allocation
    }

    /// Copy a bitmap into the atlas.
    ///
    /// Returns `None` for empty and oversized bitmaps.
    fn allocate_bitmap(
        atlas: &mut TextureAtlas,
        bitmap: &GlyphBitmap,
        pixels_per_point: f32,
        transfer: FontColorTransferFunction,
    ) -> Option<UvRect> {
        let [width, height] = bitmap.image.size;
        if width == 0 || height == 0 || MAX_GLYPH_SIZE < width || MAX_GLYPH_SIZE < height {
            return None;
        }
        let (glyph_pos, image) = atlas.allocate((width, height));
        for y in 0..height {
            for x in 0..width {
                image[(glyph_pos.0 + x, glyph_pos.1 + y)] =
                    transfer.to_atlas_color(bitmap.image[(x, y)]);
            }
        }
        Some(UvRect {
            offset: bitmap.offset_px / pixels_per_point,
            size: vec2(width as f32, height as f32) / pixels_per_point,
            min: [glyph_pos.0 as u16, glyph_pos.1 as u16],
            max: [(glyph_pos.0 + width) as u16, (glyph_pos.1 + height) as u16],
        })
    }
}
