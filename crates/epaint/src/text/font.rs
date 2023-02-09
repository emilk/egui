use crate::{
    mutex::{Mutex, RwLock},
    TextureAtlas,
};
use emath::{vec2, Vec2};
use std::collections::BTreeSet;
use std::sync::Arc;

// ----------------------------------------------------------------------------

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphInfo {
    /// Used for pair-kerning.
    ///
    /// Doesn't need to be unique.
    /// Use `ab_glyph::GlyphId(0)` if you just want to have an id, and don't care.
    pub(crate) id: ab_glyph::GlyphId,

    /// Unit: points.
    pub advance_width: f32,

    /// Texture coordinates.
    pub uv_rect: UvRect,
}

impl Default for GlyphInfo {
    /// Basically a zero-width space.
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
    name: String,
    ab_glyph_font: ab_glyph::FontArc,
    /// Maximum character height
    scale_in_pixels: u32,
    height_in_points: f32,
    // move each character by this much (hack)
    y_offset: f32,
    pixels_per_point: f32,
    glyph_info_cache: RwLock<ahash::HashMap<char, GlyphInfo>>, // TODO(emilk): standard Mutex
    atlas: Arc<Mutex<TextureAtlas>>,
}

impl FontImpl {
    pub fn new(
        atlas: Arc<Mutex<TextureAtlas>>,
        pixels_per_point: f32,
        name: String,
        ab_glyph_font: ab_glyph::FontArc,
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
            ab_glyph_font,
            scale_in_pixels,
            height_in_points,
            y_offset,
            pixels_per_point,
            glyph_info_cache: Default::default(),
            atlas,
        }
    }

    /// Code points that will always be replaced by the replacement character.
    ///
    /// See also [`invisible_char`].
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
        use ab_glyph::Font as _;
        self.ab_glyph_font
            .codepoint_ids()
            .map(|(_, chr)| chr)
            .filter(|&chr| !self.ignore_character(chr))
    }

    /// `\n` will result in `None`
    fn glyph_info(&self, c: char) -> Option<GlyphInfo> {
        {
            if let Some(glyph_info) = self.glyph_info_cache.read().get(&c) {
                return Some(*glyph_info);
            }
        }

        if self.ignore_character(c) {
            return None; // these will result in the replacement character when rendering
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

        if invisible_char(c) {
            let glyph_info = GlyphInfo::default();
            self.glyph_info_cache.write().insert(c, glyph_info);
            return Some(glyph_info);
        }

        // Add new character:
        use ab_glyph::Font as _;
        let glyph_id = self.ab_glyph_font.glyph_id(c);

        if glyph_id.0 == 0 {
            None // unsupported character
        } else {
            let glyph_info = allocate_glyph(
                &mut self.atlas.lock(),
                &self.ab_glyph_font,
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
    pub fn pair_kerning(
        &self,
        last_glyph_id: ab_glyph::GlyphId,
        glyph_id: ab_glyph::GlyphId,
    ) -> f32 {
        use ab_glyph::{Font as _, ScaleFont};
        self.ab_glyph_font
            .as_scaled(self.scale_in_pixels as f32)
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

/// Code points that will always be invisible (zero width).
///
/// See also [`FontImpl::ignore_character`].
#[inline]
fn invisible_char(c: char) -> bool {
    if c == '\r' {
        // A character most vile and pernicious. Don't display it.
        return true;
    }

    // See https://github.com/emilk/egui/issues/336

    // From https://www.fileformat.info/info/unicode/category/Cf/list.htm

    // TODO(emilk): heed bidi characters

    matches!(
        c,
        '\u{200B}' // ZERO WIDTH SPACE
            | '\u{200C}' // ZERO WIDTH NON-JOINER
            | '\u{200D}' // ZERO WIDTH JOINER
            | '\u{200E}' // LEFT-TO-RIGHT MARK
            | '\u{200F}' // RIGHT-TO-LEFT MARK
            | '\u{202A}' // LEFT-TO-RIGHT EMBEDDING
            | '\u{202B}' // RIGHT-TO-LEFT EMBEDDING
            | '\u{202C}' // POP DIRECTIONAL FORMATTING
            | '\u{202D}' // LEFT-TO-RIGHT OVERRIDE
            | '\u{202E}' // RIGHT-TO-LEFT OVERRIDE
            | '\u{2060}' // WORD JOINER
            | '\u{2061}' // FUNCTION APPLICATION
            | '\u{2062}' // INVISIBLE TIMES
            | '\u{2063}' // INVISIBLE SEPARATOR
            | '\u{2064}' // INVISIBLE PLUS
            | '\u{2066}' // LEFT-TO-RIGHT ISOLATE
            | '\u{2067}' // RIGHT-TO-LEFT ISOLATE
            | '\u{2068}' // FIRST STRONG ISOLATE
            | '\u{2069}' // POP DIRECTIONAL ISOLATE
            | '\u{206A}' // INHIBIT SYMMETRIC SWAPPING
            | '\u{206B}' // ACTIVATE SYMMETRIC SWAPPING
            | '\u{206C}' // INHIBIT ARABIC FORM SHAPING
            | '\u{206D}' // ACTIVATE ARABIC FORM SHAPING
            | '\u{206E}' // NATIONAL DIGIT SHAPES
            | '\u{206F}' // NOMINAL DIGIT SHAPES
            | '\u{FEFF}' // ZERO WIDTH NO-BREAK SPACE
    )
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
            let (glyph_pos, image) = atlas.allocate((glyph_width, glyph_height));
            glyph.draw(|x, y, v| {
                if v > 0.0 {
                    let px = glyph_pos.0 + x as usize;
                    let py = glyph_pos.1 + y as usize;
                    image[(px, py)] = v;
                }
            });

            let offset_in_pixels = vec2(bb.min.x, scale_in_pixels + bb.min.y);
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
