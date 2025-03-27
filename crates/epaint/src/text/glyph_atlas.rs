use std::borrow::Cow;

use ahash::HashMapExt;
use ecolor::Color32;
use emath::{vec2, OrderedFloat, Vec2};
use parley::{Glyph, GlyphRun};
use swash::zeno;

use crate::TextureAtlas;

use super::FontTweak;

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

// Subpixel binning, taken from cosmic-text:
// https://github.com/pop-os/cosmic-text/blob/974ddaed96b334f560b606ebe5d2ca2d2f9f23ef/src/glyph_cache.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum SubpixelBin {
    Zero,
    One,
    Two,
    Three,
}

impl SubpixelBin {
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

    fn as_float(&self) -> f32 {
        match self {
            Self::Zero => 0.0,
            Self::One => 0.25,
            Self::Two => 0.5,
            Self::Three => 0.75,
        }
    }

    pub const SUBPIXEL_OFFSETS: [f32; 4] = [0.0, 0.25, 0.5, 0.75];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct GlyphKey {
    glyph_id: swash::GlyphId,
    // We don't store the y-position because it's always rounded to an integer coordinate
    x: SubpixelBin,
    style_id: u32,
}

impl GlyphKey {
    fn from_glyph(glyph: &Glyph, scale: f32, style_id: u32) -> (Self, i32) {
        let (x, x_bin) = SubpixelBin::new(glyph.x * scale);
        (
            Self {
                glyph_id: glyph.id,
                x: x_bin,
                style_id,
            },
            x,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct StyleKey<'a> {
    font_id: u64,
    font_size: OrderedFloat<f32>,
    skew: i8,
    hinting_enabled: bool,
    /// We want to avoid doing a bunch of allocations. When looking up this key in a map, this can be a borrowed slice.
    /// We only need to convert it to an owned [`Vec<i16>`] the first time we insert it into the map.
    normalized_coords: Cow<'a, [i16]>,
}

impl<'a> StyleKey<'a> {
    fn new(
        font_id: u64,
        font_size: f32,
        skew: i8,
        hinting_enabled: bool,
        normalized_coords: &'a [i16],
    ) -> Self {
        Self {
            font_id,
            font_size: font_size.into(),
            skew,
            hinting_enabled,
            normalized_coords: Cow::Borrowed(normalized_coords),
        }
    }

    fn to_static(&self) -> StyleKey<'static> {
        StyleKey {
            normalized_coords: self.normalized_coords.clone().into_owned().into(),
            ..*self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RenderedGlyph {
    rect: UvRect,
    is_color_glyph: bool,
}

pub(super) struct GlyphAtlas {
    scale_context: swash::scale::ScaleContext,
    /// Style-related properties (font, size, variation coordinates) are the same for each glyph run and don't need to
    /// be part of each glyph's cache key. Instead, we associate each style with its own compact ID, included in each
    /// glyph's cache key. Compared to having a nested hash map of one cache per style, this keeps things flat and
    /// avoids a bunch of gnarly lifetime issues.
    style_ids: ahash::HashMap<StyleKey<'static>, u32>,
    next_style_id: u32,
    rendered_glyphs: ahash::HashMap<GlyphKey, Option<RenderedGlyph>>,
    /// Map of [`parley::fontique::Blob`] ID + [`parley::Font`] indexes to [`swash::FontRef`] offsets and cache keys.
    swash_keys: ahash::HashMap<(u64, u32), (swash::CacheKey, u32)>,
}

impl GlyphAtlas {
    pub(super) fn new() -> Self {
        Self {
            scale_context: swash::scale::ScaleContext::new(),
            style_ids: ahash::HashMap::new(),
            next_style_id: 0,
            rendered_glyphs: ahash::HashMap::new(),
            swash_keys: ahash::HashMap::new(),
        }
    }

    /// Clears this glyph atlas, allowing it to be reused.
    /// This will not clear the associated texture atlas--you should do that yourself before calling this.
    pub fn clear(&mut self) {
        self.style_ids.clear();
        self.next_style_id = 0;
        self.rendered_glyphs.clear();
        self.swash_keys.clear();
    }

    pub fn render_glyph_run<'a: 'b, 'b, 'c>(
        &'a mut self,
        atlas: &'c mut TextureAtlas,
        glyph_run: &'b GlyphRun<'b, Color32>,
        offset: Vec2,
        hinting_enabled: bool,
        pixels_per_point: f32,
        font_tweaks: &ahash::HashMap<u64, FontTweak>,
    ) -> impl Iterator<Item = (Glyph, Option<UvRect>, (i32, i32), Color32)> + use<'a, 'b, 'c> {
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

        let size = font_size * pixels_per_point;
        let normalized_coords = run.normalized_coords();

        let font_tweak = font_tweaks.get(&font_id);
        let tweak_offset = font_tweak.map_or(0.0, |tweak| {
            (font_size * tweak.y_offset_factor) + tweak.y_offset
        });
        let hinting_enabled = font_tweak
            .and_then(|tweak| tweak.hinting_override)
            .unwrap_or(hinting_enabled);

        let mut scaler: swash::scale::Scaler<'b> = self
            .scale_context
            .builder(font_ref)
            .size(size)
            .normalized_coords(normalized_coords)
            .hint(hinting_enabled)
            .build();
        let rendered_glyphs = &mut self.rendered_glyphs;
        let color = glyph_run.style().brush;

        // TODO(valadaptive): there's also a faux embolden property, but it's always true with the defaut font because
        // it's "light" and we technically asked for "normal"
        let skew = run.synthesis().skew();

        let style_key = StyleKey::<'b>::new(
            font_id,
            size,
            skew.unwrap_or_default() as i8,
            hinting_enabled,
            normalized_coords,
        );

        let style_id = match self.style_ids.get(&style_key) {
            Some(key) => *key,
            None => *self
                .style_ids
                .entry(style_key.to_static())
                .or_insert_with(|| {
                    let id = self.next_style_id;
                    self.next_style_id += 1;
                    id
                }),
        };

        glyph_run.positioned_glyphs().map(move |mut glyph| {
            // The Y-position transform applies to the font *after* it's been hinted, making it blurry. (So does the
            // X-position transform, but the hinter doesn't change the X coordinates anymore.)
            glyph.x += offset.x;
            let y = ((glyph.y + offset.y + tweak_offset) * pixels_per_point).round();
            glyph.y = y / pixels_per_point;
            let y = y as i32;
            let (cache_key, x) = GlyphKey::from_glyph(&glyph, pixels_per_point, style_id);

            if let Some(rendered_glyph) = rendered_glyphs.get(&cache_key) {
                return (
                    glyph,
                    rendered_glyph.map(|r| r.rect),
                    (x, y),
                    rendered_glyph.map_or(color, |r| {
                        if r.is_color_glyph {
                            Color32::WHITE
                        } else {
                            color
                        }
                    }),
                );
            }
            let offset = zeno::Vector::new(cache_key.x.as_float(), 0.0);
            let Some(image) = swash::scale::Render::new(&[
                swash::scale::Source::ColorOutline(0),
                swash::scale::Source::ColorBitmap(swash::scale::StrikeWith::BestFit),
                swash::scale::Source::Outline,
            ])
            .transform(skew.map(|skew| {
                zeno::Transform::skew(zeno::Angle::from_degrees(skew), zeno::Angle::ZERO)
            }))
            .format(swash::zeno::Format::Alpha)
            .offset(offset)
            .render(&mut scaler, glyph.id) else {
                rendered_glyphs.insert(cache_key, None);
                return (glyph, None, (x, y), color);
            };

            // Some glyphs may have zero size (e.g. whitespace). Don't bother rendering them.
            if image.placement.width == 0 || image.placement.height == 0 {
                rendered_glyphs.insert(cache_key, None);
                return (glyph, None, (x, y), color);
            }

            let gamma = atlas.gamma;
            let (allocated_pos, font_image) = atlas.allocate((
                image.placement.width as usize,
                image.placement.height as usize,
            ));

            let is_color_glyph = match image.content {
                swash::scale::image::Content::Mask => {
                    let mut i = 0;
                    for y in 0..image.placement.height as usize {
                        for x in 0..image.placement.width as usize {
                            font_image[(x + allocated_pos.0, y + allocated_pos.1)] =
                                TextureAtlas::coverage_to_color(
                                    gamma,
                                    image.data[i] as f32 / 255.0,
                                );
                            i += 1;
                        }
                    }

                    false
                }
                swash::scale::image::Content::SubpixelMask => {
                    panic!("Got a subpixel glyph we didn't ask for")
                }
                swash::scale::image::Content::Color => {
                    let mut i = 0;
                    for y in 0..image.placement.height as usize {
                        for x in 0..image.placement.width as usize {
                            let [r, g, b, a] = image.data[i * 4..(i * 4) + 4].try_into().unwrap();
                            font_image[(x + allocated_pos.0, y + allocated_pos.1)] =
                                Color32::from_rgba_unmultiplied(r, g, b, a);
                            i += 1;
                        }
                    }

                    true
                }
            };

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

            rendered_glyphs.insert(
                cache_key,
                Some(RenderedGlyph {
                    rect: uv_rect,
                    is_color_glyph,
                }),
            );
            (
                glyph,
                Some(uv_rect),
                (x, y),
                if is_color_glyph {
                    Color32::WHITE
                } else {
                    color
                },
            )
        })
    }
}
