use crate::{
    mutex::{Mutex, RwLock},
    text::TextStyle,
    TextureAtlas,
};
use ahash::AHashMap;
use emath::{vec2, Vec2};
use std::collections::BTreeSet;
use std::sync::Arc;

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

#[derive(Clone, Copy, Debug)]
pub struct GlyphInfo {
    pub(crate) id: ab_glyph::GlyphId,

    /// Unit: points.
    pub advance_width: f32,

    /// Texture coordinates. None for space.
    pub uv_rect: UvRect,
}

impl Default for GlyphInfo {
    fn default() -> Self {
        Self {
            id: ab_glyph::GlyphId(0),
            advance_width: 0.0,
            uv_rect: Default::default(),
        }
    }
}

// ----------------------------------------------------------------------------

/// A specific font with a size.
/// The interface uses points as the unit for everything.
pub struct FontImpl {
    ab_glyph_font: ab_glyph::FontArc,
    /// Maximum character height
    scale_in_pixels: f32,
    height_in_points: f32,
    // move each character by this much (hack)
    y_offset: f32,
    pixels_per_point: f32,
    glyph_info_cache: RwLock<AHashMap<char, GlyphInfo>>, // TODO: standard Mutex
    atlas: Arc<Mutex<TextureAtlas>>,
}

impl FontImpl {
    pub fn new(
        atlas: Arc<Mutex<TextureAtlas>>,
        pixels_per_point: f32,
        ab_glyph_font: ab_glyph::FontArc,
        scale_in_points: f32,
        y_offset: f32,
    ) -> FontImpl {
        assert!(scale_in_points > 0.0);
        assert!(pixels_per_point > 0.0);

        let scale_in_pixels = pixels_per_point * scale_in_points;

        // Round to an even number of physical pixels to get even kerning.
        // See https://github.com/emilk/egui/issues/382
        let scale_in_pixels = scale_in_pixels.round();
        let scale_in_points = scale_in_pixels / pixels_per_point;

        let height_in_points = scale_in_points;

        // TODO: use v_metrics for line spacing ?
        // let v = rusttype_font.v_metrics(Scale::uniform(scale_in_pixels));
        // let height_in_pixels = v.ascent - v.descent + v.line_gap;
        // let height_in_points = height_in_pixels / pixels_per_point;

        // Round to closest pixel:
        let y_offset = (y_offset * pixels_per_point).round() / pixels_per_point;

        Self {
            ab_glyph_font,
            scale_in_pixels,
            height_in_points,
            y_offset,
            pixels_per_point,
            glyph_info_cache: Default::default(),
            atlas,
        }
    }

    /// An un-ordered iterator over all supported characters.
    fn characters(&self) -> impl Iterator<Item = char> + '_ {
        use ab_glyph::Font as _;
        self.ab_glyph_font
            .codepoint_ids()
            .map(|(_, chr)| chr)
            .filter(|chr| {
                !matches!(
                    chr,
                    // Strip out a religious symbol with secondary nefarious interpretation:
                    '\u{534d}' | '\u{5350}' |

                    // Ignore ubuntu-specific stuff in `Ubuntu-Light.ttf`:
                    '\u{E0FF}' | '\u{EFFD}' | '\u{F0FF}' | '\u{F200}'
                )
            })
    }

    /// `\n` will result in `None`
    fn glyph_info(&self, c: char) -> Option<GlyphInfo> {
        {
            if let Some(glyph_info) = self.glyph_info_cache.read().get(&c) {
                return Some(*glyph_info);
            }
        }

        // Add new character:
        use ab_glyph::Font as _;
        let glyph_id = self.ab_glyph_font.glyph_id(c);
        if glyph_id.0 == 0 {
            if invisible_char(c) {
                // hack
                let glyph_info = GlyphInfo::default();
                self.glyph_info_cache.write().insert(c, glyph_info);
                Some(glyph_info)
            } else {
                None
            }
        } else {
            let mut glyph_info = allocate_glyph(
                &mut self.atlas.lock(),
                &self.ab_glyph_font,
                glyph_id,
                self.scale_in_pixels,
                self.y_offset,
                self.pixels_per_point,
            );

            if c == '\t' {
                if let Some(space) = self.glyph_info(' ') {
                    glyph_info.advance_width = crate::text::TAB_SIZE as f32 * space.advance_width;
                }
            }

            self.glyph_info_cache.write().insert(c, glyph_info);
            Some(glyph_info)
        }
    }

    #[inline]
    pub fn pair_kerning(
        &self,
        last_glyph_id: ab_glyph::GlyphId,
        glyph_id: ab_glyph::GlyphId,
    ) -> f32 {
        use ab_glyph::{Font as _, ScaleFont};
        self.ab_glyph_font
            .as_scaled(self.scale_in_pixels)
            .kern(last_glyph_id, glyph_id)
            / self.pixels_per_point
    }

    /// Height of one row of text. In points
    #[inline(always)]
    pub fn row_height(&self) -> f32 {
        self.height_in_points
    }

    #[inline(always)]
    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }
}

type FontIndex = usize;

// TODO: rename?
/// Wrapper over multiple `FontImpl` (e.g. a primary + fallbacks for emojis)
pub struct Font {
    text_style: TextStyle,
    fonts: Vec<Arc<FontImpl>>,
    /// Lazily calculated.
    characters: RwLock<Option<std::collections::BTreeSet<char>>>,
    replacement_glyph: (FontIndex, GlyphInfo),
    pixels_per_point: f32,
    row_height: f32,
    glyph_info_cache: RwLock<AHashMap<char, (FontIndex, GlyphInfo)>>,
}

impl Font {
    pub fn new(text_style: TextStyle, fonts: Vec<Arc<FontImpl>>) -> Self {
        if fonts.is_empty() {
            return Self {
                text_style,
                fonts,
                characters: RwLock::new(None),
                replacement_glyph: Default::default(),
                pixels_per_point: 0.0,
                row_height: 0.0,
                glyph_info_cache: Default::default(),
            };
        }

        let pixels_per_point = fonts[0].pixels_per_point();
        let row_height = fonts[0].row_height();

        let mut slf = Self {
            text_style,
            fonts,
            characters: RwLock::new(None),
            replacement_glyph: Default::default(),
            pixels_per_point,
            row_height,
            glyph_info_cache: Default::default(),
        };

        const PRIMARY_REPLACEMENT_CHAR: char = '◻'; // white medium square
        const FALLBACK_REPLACEMENT_CHAR: char = '?'; // fallback for the fallback

        let replacement_glyph = slf
            .glyph_info_no_cache_or_fallback(PRIMARY_REPLACEMENT_CHAR)
            .or_else(|| slf.glyph_info_no_cache_or_fallback(FALLBACK_REPLACEMENT_CHAR))
            .unwrap_or_else(|| {
                panic!(
                    "Failed to find replacement characters {:?} or {:?}",
                    PRIMARY_REPLACEMENT_CHAR, FALLBACK_REPLACEMENT_CHAR
                )
            });
        slf.replacement_glyph = replacement_glyph;

        // Preload the printable ASCII characters [32, 126] (which excludes control codes):
        const FIRST_ASCII: usize = 32; // 32 == space
        const LAST_ASCII: usize = 126;
        for c in (FIRST_ASCII..=LAST_ASCII).map(|c| c as u8 as char) {
            slf.glyph_info(c);
        }
        slf.glyph_info('°');
        slf.glyph_info(crate::text::PASSWORD_REPLACEMENT_CHAR); // password replacement character

        slf
    }

    /// All supported characters
    pub fn characters(&self) -> BTreeSet<char> {
        if self.characters.read().is_none() {
            let mut characters = BTreeSet::new();
            for font in &self.fonts {
                characters.extend(font.characters());
            }
            self.characters.write().replace(characters);
        }
        self.characters.read().clone().unwrap()
    }

    #[inline(always)]
    pub fn text_style(&self) -> TextStyle {
        self.text_style
    }

    #[inline(always)]
    pub fn round_to_pixel(&self, point: f32) -> f32 {
        (point * self.pixels_per_point).round() / self.pixels_per_point
    }

    /// Height of one row of text. In points
    #[inline(always)]
    pub fn row_height(&self) -> f32 {
        self.row_height
    }

    pub fn uv_rect(&self, c: char) -> UvRect {
        self.glyph_info_cache
            .read()
            .get(&c)
            .map(|gi| gi.1.uv_rect)
            .unwrap_or_default()
    }

    /// Width of this character in points.
    pub fn glyph_width(&self, c: char) -> f32 {
        self.glyph_info(c).1.advance_width
    }

    /// `\n` will (intentionally) show up as the replacement character.
    fn glyph_info(&self, c: char) -> (FontIndex, GlyphInfo) {
        {
            if let Some(font_index_glyph_info) = self.glyph_info_cache.read().get(&c) {
                return *font_index_glyph_info;
            }
        }

        let font_index_glyph_info = self.glyph_info_no_cache_or_fallback(c);
        let font_index_glyph_info = font_index_glyph_info.unwrap_or(self.replacement_glyph);
        self.glyph_info_cache
            .write()
            .insert(c, font_index_glyph_info);
        font_index_glyph_info
    }

    #[inline]
    pub(crate) fn glyph_info_and_font_impl(&self, c: char) -> (&FontImpl, GlyphInfo) {
        let (font_index, glyph_info) = self.glyph_info(c);
        let font_impl = &self.fonts[font_index];
        (font_impl, glyph_info)
    }

    fn glyph_info_no_cache_or_fallback(&self, c: char) -> Option<(FontIndex, GlyphInfo)> {
        for (font_index, font_impl) in self.fonts.iter().enumerate() {
            if let Some(glyph_info) = font_impl.glyph_info(c) {
                self.glyph_info_cache
                    .write()
                    .insert(c, (font_index, glyph_info));
                return Some((font_index, glyph_info));
            }
        }
        None
    }
}

#[inline]
fn invisible_char(c: char) -> bool {
    // See https://github.com/emilk/egui/issues/336

    // From https://www.fileformat.info/info/unicode/category/Cf/list.htm
    ('\u{200B}'..='\u{206F}').contains(&c) // TODO: heed bidi characters
}

fn allocate_glyph(
    atlas: &mut TextureAtlas,
    font: &ab_glyph::FontArc,
    glyph_id: ab_glyph::GlyphId,
    scale_in_pixels: f32,
    y_offset: f32,
    pixels_per_point: f32,
) -> GlyphInfo {
    assert!(glyph_id.0 != 0);
    use ab_glyph::{Font as _, ScaleFont};

    let glyph =
        glyph_id.with_scale_and_position(scale_in_pixels, ab_glyph::Point { x: 0.0, y: 0.0 });

    let uv_rect = font.outline_glyph(glyph).map(|glyph| {
        let bb = glyph.px_bounds();
        let glyph_width = bb.width() as usize;
        let glyph_height = bb.height() as usize;
        if glyph_width == 0 || glyph_height == 0 {
            UvRect::default()
        } else {
            let glyph_pos = atlas.allocate((glyph_width, glyph_height));

            let texture = atlas.texture_mut();
            glyph.draw(|x, y, v| {
                if v > 0.0 {
                    let px = glyph_pos.0 + x as usize;
                    let py = glyph_pos.1 + y as usize;
                    texture[(px, py)] = (v * 255.0).round() as u8;
                }
            });

            let offset_in_pixels = vec2(bb.min.x as f32, scale_in_pixels as f32 + bb.min.y as f32);
            let offset = offset_in_pixels / pixels_per_point + y_offset * Vec2::Y;
            UvRect {
                offset,
                size: vec2(glyph_width as f32, glyph_height as f32) / pixels_per_point,
                min: [glyph_pos.0 as u16, glyph_pos.1 as u16],
                max: [
                    (glyph_pos.0 + glyph_width) as u16,
                    (glyph_pos.1 + glyph_height) as u16,
                ],
            }
        }
    });
    let uv_rect = uv_rect.unwrap_or_default();

    let advance_width_in_points =
        font.as_scaled(scale_in_pixels).h_advance(glyph_id) / pixels_per_point;

    GlyphInfo {
        id: glyph_id,
        advance_width: advance_width_in_points,
        uv_rect,
    }
}
