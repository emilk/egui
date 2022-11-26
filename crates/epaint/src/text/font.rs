use crate::{
    mutex::{Mutex, RwLock},
    Color32, TextureAtlas,
};
use emath::{vec2, Vec2};
use std::collections::BTreeSet;
use std::sync::Arc;

// ----------------------------------------------------------------------------
pub type GlyphId = u32;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphInfo {
    pub(crate) id: GlyphId,
    /// Unit: points.
    pub advance_width: f32,

    /// Texture coordinates. None for space.
    pub uv_rect: UvRect,
}

impl Default for GlyphInfo {
    fn default() -> Self {
        Self {
            id: 0,
            advance_width: 0.0,
            uv_rect: Default::default(),
        }
    }
}

// ----------------------------------------------------------------------------

/// A specific font with a size.
/// The interface uses points as the unit for everything.
pub struct FontImpl {
    name: String,
    freetype_face: freetype::Face,
    /// Maximum character height
    scale_in_pixels: u32,
    height_in_points: f32,
    // move each character by this much (hack)
    y_offset: f32,
    pixels_per_point: f32,
    glyph_info_cache: RwLock<ahash::HashMap<char, GlyphInfo>>, // TODO(emilk): standard Mutex
    atlas: Arc<Mutex<TextureAtlas>>,
}

struct CharacterIter<'a> {
    freetype_font: &'a freetype::Face,
    gindex: u32,
    charcode: Option<u32>,
}

impl<'a> CharacterIter<'a> {
    pub fn new(freetype_face: &'a freetype::Face) -> Self {
        Self {
            freetype_font: freetype_face,
            gindex: 0,
            charcode: None,
        }
    }
}

impl Iterator for CharacterIter<'_> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let raw_font = unsafe { std::mem::transmute(self.freetype_font.raw()) };
        match self.charcode {
            Some(charcode) => {
                self.charcode = Some(unsafe {
                    freetype::ffi::FT_Get_Next_Char(raw_font, charcode, &mut self.gindex)
                })
            }
            None => {
                self.charcode =
                    Some(unsafe { freetype::ffi::FT_Get_First_Char(raw_font, &mut self.gindex) })
            }
        };

        match (self.gindex, self.charcode) {
            (0, _) => None,
            (_, None) => None,
            (_, Some(charcode)) => char::from_u32(charcode),
        }
    }
}

impl FontImpl {
    pub fn new(
        atlas: Arc<Mutex<TextureAtlas>>,
        pixels_per_point: f32,
        name: String,
        freetype_face: freetype::Face,
        scale_in_pixels: u32,
        y_offset_points: f32,
    ) -> FontImpl {
        assert!(scale_in_pixels > 0);
        assert!(pixels_per_point > 0.0);

        let height_in_points = scale_in_pixels as f32 / pixels_per_point;

        // TODO(emilk): use these font metrics?
        // use ab_glyph::ScaleFont as _;
        // let scaled = ab_glyph_font.as_scaled(scale_in_pixels as f32);
        // dbg!(scaled.ascent());
        // dbg!(scaled.descent());
        // dbg!(scaled.line_gap());

        // Round to closest pixel:
        let y_offset = (y_offset_points * pixels_per_point).round() / pixels_per_point;

        Self {
            name,
            freetype_face,
            scale_in_pixels,
            height_in_points,
            y_offset,
            pixels_per_point,
            glyph_info_cache: Default::default(),
            atlas,
        }
    }

    fn ignore_character(&self, chr: char) -> bool {
        if self.name == "emoji-icon-font" {
            // HACK: https://github.com/emilk/egui/issues/1284 https://github.com/jslegers/emoji-icon-font/issues/18
            // Don't show the wrong fullwidth capital letters:
            if 'Ｓ' <= chr && chr <= 'Ｙ' {
                return true;
            }
        }

        matches!(
            chr,
            // Strip out a religious symbol with secondary nefarious interpretation:
            '\u{534d}' | '\u{5350}' |

            // Ignore ubuntu-specific stuff in `Ubuntu-Light.ttf`:
            '\u{E0FF}' | '\u{EFFD}' | '\u{F0FF}' | '\u{F200}'
        )
    }

    /// An un-ordered iterator over all supported characters.
    fn characters(&self) -> impl Iterator<Item = char> + '_ {
        CharacterIter::new(&self.freetype_face)
    }

    /// `\n` will result in `None`
    fn glyph_info(&self, c: char) -> Option<GlyphInfo> {
        {
            if let Some(glyph_info) = self.glyph_info_cache.read().get(&c) {
                return Some(*glyph_info);
            }
        }

        if self.ignore_character(c) {
            return None;
        }

        if c == '\t' {
            if let Some(space) = self.glyph_info(' ') {
                let glyph_info = GlyphInfo {
                    advance_width: crate::text::TAB_SIZE as f32 * space.advance_width,
                    ..GlyphInfo::default()
                };
                self.glyph_info_cache.write().insert(c, glyph_info);
                return Some(glyph_info);
            }
        }

        if c == '\u{2009}' {
            // Thin space, often used as thousands deliminator: 1 234 567 890
            // https://www.compart.com/en/unicode/U+2009
            // https://en.wikipedia.org/wiki/Thin_space

            if let Some(space) = self.glyph_info(' ') {
                let em = self.height_in_points; // TODO(emilk): is this right?
                let advance_width = f32::min(em / 6.0, space.advance_width * 0.5);
                let glyph_info = GlyphInfo {
                    advance_width,
                    ..GlyphInfo::default()
                };
                self.glyph_info_cache.write().insert(c, glyph_info);
                return Some(glyph_info);
            }
        }

        // Add new character:
        let glyph_id = self.freetype_face.get_char_index(c as usize);

        if glyph_id == 0 {
            if invisible_char(c) {
                // hack
                let glyph_info = GlyphInfo::default();
                self.glyph_info_cache.write().insert(c, glyph_info);
                Some(glyph_info)
            } else {
                None // unsupported character
            }
        } else {
            let glyph_info = allocate_glyph(
                &mut self.atlas.lock(),
                &self.freetype_face,
                glyph_id,
                self.scale_in_pixels as f32,
                self.y_offset,
                self.pixels_per_point,
            );

            self.glyph_info_cache.write().insert(c, glyph_info);
            Some(glyph_info)
        }
    }

    #[inline]
    pub fn pair_kerning(&self, last_glyph_id: GlyphId, glyph_id: GlyphId) -> f32 {
        let result = self
            .freetype_face
            .get_kerning(
                last_glyph_id,
                glyph_id,
                freetype::face::KerningMode::KerningUnfitted,
            )
            .unwrap();
        result.x as f32 / (1 << 6) as f32 / self.pixels_per_point
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

// TODO(emilk): rename?
/// Wrapper over multiple [`FontImpl`] (e.g. a primary + fallbacks for emojis)
pub struct Font {
    fonts: Vec<Arc<FontImpl>>,
    /// Lazily calculated.
    characters: Option<BTreeSet<char>>,
    replacement_glyph: (FontIndex, GlyphInfo),
    pixels_per_point: f32,
    row_height: f32,
    glyph_info_cache: ahash::HashMap<char, (FontIndex, GlyphInfo)>,
}

impl Font {
    pub fn new(fonts: Vec<Arc<FontImpl>>) -> Self {
        if fonts.is_empty() {
            return Self {
                fonts,
                characters: None,
                replacement_glyph: Default::default(),
                pixels_per_point: 1.0,
                row_height: 0.0,
                glyph_info_cache: Default::default(),
            };
        }

        let pixels_per_point = fonts[0].pixels_per_point();
        let row_height = fonts[0].row_height();

        let mut slf = Self {
            fonts,
            characters: None,
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

        slf
    }

    pub fn preload_characters(&mut self, s: &str) {
        for c in s.chars() {
            self.glyph_info(c);
        }
    }

    pub fn preload_common_characters(&mut self) {
        // Preload the printable ASCII characters [32, 126] (which excludes control codes):
        const FIRST_ASCII: usize = 32; // 32 == space
        const LAST_ASCII: usize = 126;
        for c in (FIRST_ASCII..=LAST_ASCII).map(|c| c as u8 as char) {
            self.glyph_info(c);
        }
        self.glyph_info('°');
        self.glyph_info(crate::text::PASSWORD_REPLACEMENT_CHAR);
    }

    /// All supported characters.
    pub fn characters(&mut self) -> &BTreeSet<char> {
        self.characters.get_or_insert_with(|| {
            let mut characters = BTreeSet::new();
            for font in &self.fonts {
                characters.extend(font.characters());
            }
            characters
        })
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
            .get(&c)
            .map(|gi| gi.1.uv_rect)
            .unwrap_or_default()
    }

    /// Width of this character in points.
    pub fn glyph_width(&mut self, c: char) -> f32 {
        self.glyph_info(c).1.advance_width
    }

    /// Can we display this glyph?
    pub fn has_glyph(&mut self, c: char) -> bool {
        self.glyph_info(c) != self.replacement_glyph // TODO(emilk): this is a false negative if the user asks about the replacement character itself 🤦‍♂️
    }

    /// Can we display all the glyphs in this text?
    pub fn has_glyphs(&mut self, s: &str) -> bool {
        s.chars().all(|c| self.has_glyph(c))
    }

    /// `\n` will (intentionally) show up as the replacement character.
    fn glyph_info(&mut self, c: char) -> (FontIndex, GlyphInfo) {
        if let Some(font_index_glyph_info) = self.glyph_info_cache.get(&c) {
            return *font_index_glyph_info;
        }

        let font_index_glyph_info = self.glyph_info_no_cache_or_fallback(c);
        let font_index_glyph_info = font_index_glyph_info.unwrap_or(self.replacement_glyph);
        self.glyph_info_cache.insert(c, font_index_glyph_info);
        font_index_glyph_info
    }

    #[inline]
    pub(crate) fn glyph_info_and_font_impl(&mut self, c: char) -> (Option<&FontImpl>, GlyphInfo) {
        if self.fonts.is_empty() {
            return (None, self.replacement_glyph.1);
        }
        let (font_index, glyph_info) = self.glyph_info(c);
        let font_impl = &self.fonts[font_index];
        (Some(font_impl), glyph_info)
    }

    fn glyph_info_no_cache_or_fallback(&mut self, c: char) -> Option<(FontIndex, GlyphInfo)> {
        for (font_index, font_impl) in self.fonts.iter().enumerate() {
            if let Some(glyph_info) = font_impl.glyph_info(c) {
                self.glyph_info_cache.insert(c, (font_index, glyph_info));
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
    ('\u{200B}'..='\u{206F}').contains(&c) // TODO(emilk): heed bidi characters
}

fn allocate_glyph(
    atlas: &mut TextureAtlas,
    font: &freetype::Face,
    glyph_id: GlyphId,
    scale_in_pixels: f32,
    y_offset: f32,
    pixels_per_point: f32,
) -> GlyphInfo {
    assert!(glyph_id != 0);
    use freetype::face::LoadFlag;

    let mut advance_width_in_points = 0.0;
    font.set_pixel_sizes(0, scale_in_pixels as u32).unwrap();
    let uv_rect = || -> Result<UvRect, freetype::Error> {
        font.load_glyph(glyph_id, LoadFlag::RENDER | LoadFlag::TARGET_LCD)?;
        let glyph = font.glyph();
        let bitmap = glyph.bitmap();
        let glyph_width = bitmap.width() / 3;
        let glyph_height = bitmap.rows();

        // freetype-rs's Glyph type will call `FT_Done_Glyph` and `FT_Done_Library` when dropping, so we use a named variable to prevent it from dropping in this scope
        let rendered_glyph = glyph.get_glyph()?;
        advance_width_in_points =
            rendered_glyph.advance_x() as f32 / (1 << 16) as f32 / pixels_per_point;

        if glyph_width == 0
            || glyph_height == 0
            || bitmap.pitch() < 3
            || bitmap.buffer().len() < (bitmap.pitch() as usize * glyph_height as usize)
        {
            return Ok(UvRect::default());
        }

        let (glyph_pos, image) = atlas.allocate((glyph_width as usize, glyph_height as usize));
        let mut buffer_cursor = 0;
        for i in 0..glyph_height {
            for j in 0..glyph_width {
                let idx = (j * 3 + buffer_cursor) as usize;
                let r = bitmap.buffer()[idx];
                let g = bitmap.buffer()[idx + 1];
                let b = bitmap.buffer()[idx + 2];
                let px = glyph_pos.0 + j as usize;
                let py = glyph_pos.1 + i as usize;

                // Luminance Y is defined by the CIE 1931 XYZ color space. Linear RGB to Y is a weighted average based on factors from the color conversion matrix:
                // Y = 0.2126*R + 0.7152*G + 0.0722*B. Computed on the integer pipe.
                let a = (4732 * r as usize + 46871 * g as usize + 13933 * b as usize) >> 16;
                image[(px, py)] = Color32::from_rgba_premultiplied(r, g, b, a as u8);
            }
            buffer_cursor += bitmap.pitch();
        }

        // Note that bitmap_left is the horizontal distance from the current pen position to the left-most border of the glyph bitmap, while bitmap_top is the vertical distance from the pen position (on the baseline) to the top-most border of the glyph bitmap. It is positive to indicate an upwards distance.
        let offset_in_pixels = vec2(
            glyph.bitmap_left() as f32,
            scale_in_pixels - glyph.bitmap_top() as f32,
        );

        let offset = offset_in_pixels / pixels_per_point + y_offset * Vec2::Y;
        Ok(UvRect {
            offset,
            size: vec2(glyph_width as f32, glyph_height as f32) / pixels_per_point,
            min: [glyph_pos.0 as u16, glyph_pos.1 as u16],
            max: [
                (glyph_pos.0 + glyph_width as usize) as u16,
                (glyph_pos.1 + glyph_height as usize) as u16,
            ],
        })
    }()
    .unwrap_or_default();

    GlyphInfo {
        id: glyph_id,
        advance_width: advance_width_in_points,
        uv_rect,
    }
}
