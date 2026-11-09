use emath::{NumExt as _, Vec2, vec2};
use nohash_hasher::IntMap;
use skrifa::GlyphId;

use crate::{
    Color32, ImageDelta, TextOptions, TextureAtlas,
    text::{
        FontFamily, FontPriority, GlyphBitmap, GlyphRasterizer, GlyphRasterizerRequest,
        MAX_GLYPH_SIZE, RasterizedGlyph,
        face_store::FontFaceKey,
        font_face::{FontFace, ShapedGlyph},
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

/// Where a glyph is in the font atlas.
///
/// The default value is the invisible, zero-size glyph.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlyphAllocation {
    /// UV rectangle for drawing.
    pub uv_rect: UvRect,

    /// A color glyph (e.g. emoji), stored with its own colors in the atlas.
    /// Do not tint it with the text color.
    pub is_color: bool,
}

/// An outline glyph in the atlas, positioned for one [`ShapedGlyph`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct OutlineGlyph {
    pub allocation: GlyphAllocation,

    /// Left edge of the glyph's pixel grid, in physical pixels.
    ///
    /// This is [`ShapedGlyph::h_pos`] rounded to the pixel (or sub-pixel bin)
    /// the bitmap was rendered for. Draw the bitmap here, not at the
    /// un-rounded `h_pos`, or it will be blurry.
    pub x_px: i32,
}

/// A glyph from a [`GlyphRasterizer`], allocated in the atlas.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RasterGlyphAllocation {
    pub allocation: GlyphAllocation,
    pub advance_px: f32,
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
pub(crate) enum SubpixelBin {
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

/// Hash of `(priority, cluster, family, pixels_per_point, font_size)`,
/// so that cache lookups do not allocate.
#[derive(Hash, PartialEq, Eq)]
struct RasterGlyphKey(u64);

impl nohash_hasher::IsEnabled for RasterGlyphKey {}

impl RasterGlyphKey {
    fn new(
        priority: FontPriority,
        cluster: &str,
        family: &FontFamily,
        pixels_per_point: f32,
        font_size: f32,
    ) -> Self {
        Self(crate::util::hash((
            priority,
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
    outline_glyphs: IntMap<OutlineGlyphKey, GlyphAllocation>,

    /// Glyphs from the [`GlyphRasterizer`]s.
    ///
    /// `None` means no rasterizer of that priority could handle the cluster.
    raster_glyphs: IntMap<RasterGlyphKey, Option<RasterGlyphAllocation>>,
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

    /// Forget what the [`GlyphRasterizer`]s produced, e.g. because they were replaced.
    pub fn clear_raster_glyphs(&mut self) {
        self.raster_glyphs.clear();
    }

    /// Get or render the glyph of `face` for `shaped`.
    ///
    /// [`GlyphId::NOTDEF`] renders the face's `.notdef` glyph ("tofu"),
    /// which is what we show for characters no font has.
    ///
    /// The bitmap is rendered at the sub-pixel bin of [`ShapedGlyph::h_pos`]
    /// (if the face has sub-pixel binning on, and the glyph is not CJK),
    /// so the same glyph can occupy up to four slots in the atlas.
    ///
    /// Never fails: see [`Self::allocate_bitmap`] for what happens when the atlas is full.
    pub fn allocate_outline(
        &mut self,
        face_key: FontFaceKey,
        face: &mut FontFace,
        metrics: &StyledMetrics,
        shaped: &ShapedGlyph,
    ) -> OutlineGlyph {
        let ShapedGlyph {
            glyph_id,
            h_pos,
            is_cjk,
        } = *shaped;

        let subpixel_binning =
            face.subpixel_binning() && !is_cjk && !face.is_color_glyph(metrics, glyph_id);
        let (x_px, bin) = if subpixel_binning {
            SubpixelBin::new(h_pos)
        } else {
            // CJK scripts contain a lot of characters and could hog the glyph atlas
            // if we stored 4 subpixel offsets per glyph.
            // Color glyphs (emoji) are big, and gain nothing from subpixel positioning.
            (h_pos.round() as i32, SubpixelBin::Zero)
        };

        let key = OutlineGlyphKey::new(face_key, glyph_id, metrics, bin);

        let Self {
            atlas,
            outline_glyphs,
            ..
        } = self;
        let subpixel_order = atlas.options().subpixel_order;
        let allocation = *outline_glyphs.entry(key).or_insert_with(|| {
            face.rasterize_glyph(metrics, glyph_id, bin, subpixel_order)
                .and_then(|bitmap| {
                    let texel = Self::atlas_texel(atlas, bitmap.is_color, subpixel_order.is_some());
                    let mut uv_rect =
                        Self::allocate_bitmap(atlas, &bitmap, metrics.pixels_per_point, texel)?;
                    uv_rect.offset.y += metrics.y_offset_in_points;
                    Some(GlyphAllocation {
                        uv_rect,
                        is_color: bitmap.is_color,
                    })
                })
                .unwrap_or_default()
        });

        OutlineGlyph { allocation, x_px }
    }

    /// Get or rasterize `cluster` using the [`GlyphRasterizer`]s of the given `priority`.
    ///
    /// The rasterizers are asked in order, and the first to return `Some` wins.
    ///
    /// Failures are cached too, so the (potentially slow) rasterizers
    /// are asked at most once per cluster and size.
    ///
    /// See [`Self::allocate_bitmap`] for what happens when the atlas is full.
    pub fn allocate_raster<'r>(
        &mut self,
        rasterizers: impl IntoIterator<Item = &'r GlyphRasterizer>,
        priority: FontPriority,
        cluster: &str,
        family: &FontFamily,
        pixels_per_point: f32,
        font_size: f32,
    ) -> Option<RasterGlyphAllocation> {
        let key = RasterGlyphKey::new(priority, cluster, family, pixels_per_point, font_size);
        if let Some(allocation) = self.raster_glyphs.get(&key) {
            return *allocation;
        }
        let request = GlyphRasterizerRequest {
            cluster,
            family,
            font_size_px: font_size * pixels_per_point,
            subpixel_offset_px: 0.0,
        };
        let allocation = rasterizers
            .into_iter()
            .filter(|rasterizer| rasterizer.priority == priority)
            .find_map(|rasterizer| (rasterizer.rasterize)(&request))
            .and_then(|RasterizedGlyph { bitmap, advance_px }| {
                let is_color = bitmap.is_color;
                // Rasterizers hand over grayscale coverage, never subpixel coverage.
                let texel = Self::atlas_texel(&self.atlas, is_color, false);
                let uv_rect =
                    Self::allocate_bitmap(&mut self.atlas, &bitmap, pixels_per_point, texel)?;
                Some(RasterGlyphAllocation {
                    allocation: GlyphAllocation { uv_rect, is_color },
                    advance_px,
                })
            });
        self.raster_glyphs.insert(key, allocation);
        allocation
    }

    /// How a bitmap's pixels become atlas texels.
    ///
    /// The transfer function assumes white coverage glyphs and discards color,
    /// so color glyphs (e.g. emoji) skip it. Subpixel coverage goes through it per channel.
    ///
    /// In a subpixel atlas, a channel above alpha marks subpixel coverage,
    /// so color glyphs are clamped to valid premultiplied colors.
    fn atlas_texel(
        atlas: &TextureAtlas,
        is_color: bool,
        subpixel: bool,
    ) -> impl Fn(Color32) -> Color32 + use<> {
        let transfer = atlas.options().color_transfer_function;
        let subpixel_atlas = atlas.options().subpixel_order.is_some();
        move |px: Color32| {
            if is_color {
                if subpixel_atlas {
                    let [r, g, b, a] = px.to_array();
                    Color32::from_rgba_premultiplied(r.min(a), g.min(a), b.min(a), a)
                } else {
                    px
                }
            } else if subpixel {
                let channel = |coverage: u8| {
                    ecolor::linear_u8_from_linear_f32(
                        transfer.alpha_from_coverage(coverage as f32 / 255.0),
                    )
                };
                let [r, g, b, _] = px.to_array();
                let [r, g, b] = [channel(r), channel(g), channel(b)];
                // The transfer function is increasing, so the smallest channel stays alpha.
                Color32::from_rgba_premultiplied(r, g, b, r.min(g).min(b))
            } else {
                transfer.to_atlas_color(px)
            }
        }
    }

    /// Copy a bitmap into the atlas.
    ///
    /// Returns `None` for empty and oversized bitmaps.
    ///
    /// Never fails for lack of space: the [`TextureAtlas`] grows in height as needed,
    /// and once it hits its maximum it logs a warning and starts overwriting old glyphs
    /// (see [`TextureAtlas::allocate`]). We avoid getting there because `Fonts::begin_pass`
    /// calls [`Self::clear`] as soon as [`Self::fill_ratio`] passes 80%.
    fn allocate_bitmap(
        atlas: &mut TextureAtlas,
        bitmap: &GlyphBitmap,
        pixels_per_point: f32,
        texel: impl Fn(Color32) -> Color32,
    ) -> Option<UvRect> {
        let [width, height] = bitmap.image.resolution();
        if width == 0 || height == 0 || MAX_GLYPH_SIZE < width || MAX_GLYPH_SIZE < height {
            return None;
        }
        let (glyph_pos, image) = atlas.allocate((width, height));
        for y in 0..height {
            for x in 0..width {
                image[(glyph_pos.0 + x, glyph_pos.1 + y)] = texel(bitmap.image[(x, y)]);
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

#[cfg(test)]
mod tests {
    use super::GlyphAtlas;
    use crate::{Color32, TextOptions, TextureAtlas, text::SubpixelOrder};

    fn atlas(subpixel_order: Option<SubpixelOrder>) -> TextureAtlas {
        let options = TextOptions {
            subpixel_order,
            ..Default::default()
        };
        TextureAtlas::new([1024, 64], options)
    }

    /// A color glyph never has a channel above its alpha in a subpixel atlas,
    /// where that would mark subpixel coverage.
    #[test]
    fn color_glyphs_are_clamped_in_a_subpixel_atlas() {
        let invalid = Color32::from_rgba_premultiplied(255, 40, 0, 128);
        let valid = Color32::from_rgba_premultiplied(100, 40, 0, 128);

        let texel = GlyphAtlas::atlas_texel(&atlas(Some(SubpixelOrder::Rgb)), true, false);
        assert_eq!(texel(invalid).to_array(), [128, 40, 0, 128]);
        assert_eq!(texel(valid), valid);

        // Without subpixel text, color glyphs are copied as they are.
        let texel = GlyphAtlas::atlas_texel(&atlas(None), true, false);
        assert_eq!(texel(invalid), invalid);
    }
}
