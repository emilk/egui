use std::sync::Arc;

use ahash::HashMapExt;
use ecolor::Color32;
use emath::{vec2, GuiRounding};
use parley::{Glyph, GlyphRun};
use swash::zeno;

use crate::{mutex::Mutex, TextureAtlas};

use super::font::UvRect;

// Subpixel binning, taken from cosmic-text:
// https://github.com/pop-os/cosmic-text/blob/974ddaed96b334f560b606ebe5d2ca2d2f9f23ef/src/glyph_cache.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum SubpixelBin {
    Zero,
    One,
    Two,
    Three,
}

impl SubpixelBin {
    pub fn new(pos: f32) -> (i32, Self) {
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
            #[allow(clippy::collapsible_else_if)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CacheKey {
    // TODO(valadaptive): is this the right key?
    font_id: u64,
    glyph_id: swash::GlyphId,
    font_size_bits: u32,
    x: SubpixelBin,
    y: SubpixelBin,
}

impl CacheKey {
    fn from_glyph(
        glyph: &Glyph,
        font_id: u64,
        font_size: f32,
        offset: (f32, f32),
        scale: f32,
    ) -> (Self, i32, i32) {
        let (x, x_bin) = SubpixelBin::new((glyph.x + offset.0) * scale);
        let (y, y_bin) = SubpixelBin::new((glyph.y + offset.1) * scale);
        (
            Self {
                font_id,
                glyph_id: glyph.id,
                font_size_bits: (font_size * scale).to_bits(),
                x: x_bin,
                y: y_bin,
            },
            x,
            y,
        )
    }
}

pub(super) struct GlyphAtlas {
    scale_context: swash::scale::ScaleContext,
    // TODO: just pass this in from Fonts
    atlas: Arc<Mutex<TextureAtlas>>,
    rendered_glyphs: ahash::HashMap<CacheKey, Option<UvRect>>,
    /// Map of [`parley::fontique::Blob`] ID + [`parley::Font`] indexes to [`swash::FontRef`] offsets and cache keys.
    swash_keys: ahash::HashMap<(u64, u32), (swash::CacheKey, u32)>,
}

impl GlyphAtlas {
    pub(super) fn new(atlas: Arc<Mutex<TextureAtlas>>) -> Self {
        Self {
            scale_context: swash::scale::ScaleContext::new(),
            atlas,
            rendered_glyphs: ahash::HashMap::new(),
            swash_keys: ahash::HashMap::new(),
        }
    }

    pub fn render_glyph_run<'a: 'b, 'b>(
        &'a mut self,
        glyph_run: &'b GlyphRun<'b, Color32>,
        offset: (f32, f32),
        pixels_per_point: f32,
    ) -> impl Iterator<Item = (Glyph, Option<UvRect>, (i32, i32))> + use<'a, 'b> {
        let run = glyph_run.run();
        let font = run.font();
        let font_size = run.font_size();
        let font_id = font.data.id();

        let (swash_key, swash_offset) = *self
            .swash_keys
            .entry((font_id, font.index))
            .or_insert_with(|| {
                let font_ref =
                    swash::FontRef::from_index(font.data.data(), font.index as usize).unwrap();
                (font_ref.key, font_ref.offset)
            });
        let font_ref: swash::FontRef<'b> = swash::FontRef {
            data: font.data.data(),
            offset: swash_offset,
            key: swash_key,
        };
        let mut scaler: swash::scale::Scaler<'b> = self
            .scale_context
            .builder(font_ref)
            .size(font_size * pixels_per_point)
            .hint(true)
            .build();
        // TODO(valadaptive): is it fine to lock the mutex here?
        let mut atlas = self.atlas.lock();
        let rendered_glyphs = &mut self.rendered_glyphs;

        glyph_run.positioned_glyphs().map(move |mut glyph| {
            // The Y-position transform applies to the font *after* it's been hinted, making it blurry. (So does the
            // X-position transform, but the hinter doesn't change the X coordinates anymore.)
            // TODO(valadaptive): remove Y subpixel position from the cache key entirely
            glyph.y = glyph.y.round_to_pixels(pixels_per_point);
            let (cache_key, x, y) =
                CacheKey::from_glyph(&glyph, font_id, font_size, offset, pixels_per_point);

            if let Some(rendered_glyph) = rendered_glyphs.get(&cache_key) {
                return (glyph, *rendered_glyph, (x, y));
            }
            let offset = zeno::Vector::new(cache_key.x.as_float(), cache_key.y.as_float());
            let Some(image) = swash::scale::Render::new(&[
                //swash::scale::Source::ColorOutline(0),
                //swash::scale::Source::ColorBitmap(swash::scale::StrikeWith::BestFit),
                swash::scale::Source::Outline,
            ])
            .format(swash::zeno::Format::Alpha)
            .offset(offset)
            .render(&mut scaler, glyph.id) else {
                rendered_glyphs.insert(cache_key, None);
                return (glyph, None, (x, y));
            };

            let (allocated_pos, font_image) = atlas.allocate((
                image.placement.width as usize,
                image.placement.height as usize,
            ));

            match image.content {
                swash::scale::image::Content::Mask => {
                    let mut i = 0;
                    for y in 0..image.placement.height as usize {
                        for x in 0..image.placement.width as usize {
                            font_image[(x + allocated_pos.0, y + allocated_pos.1)] =
                                image.data[i] as f32 / 255.0;
                            i += 1;
                        }
                    }
                }
                // TODO(valadaptive): color emoji support
                swash::scale::image::Content::SubpixelMask => todo!(),
                swash::scale::image::Content::Color => todo!(),
            }

            let uv_rect = UvRect {
                offset: vec2(image.placement.left as f32, -image.placement.top as f32)
                    / pixels_per_point,
                size: vec2(image.placement.width as f32, image.placement.height as f32)
                    / pixels_per_point,
                min: [allocated_pos.0 as u16, allocated_pos.1 as u16],
                max: [
                    allocated_pos.0 as u16 + image.placement.width as u16,
                    allocated_pos.1 as u16 + image.placement.height as u16,
                ],
            };

            rendered_glyphs.insert(cache_key, Some(uv_rect));
            (glyph, Some(uv_rect), (x, y))
        })
    }
}
