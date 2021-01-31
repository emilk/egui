use std::sync::Arc;

use {
    ahash::AHashMap,
    rusttype::{point, Scale},
};

use crate::{
    mutex::{Mutex, RwLock},
    text::galley::{Galley, Row},
    TextureAtlas,
};
use emath::{vec2, Vec2};

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct UvRect {
    /// X/Y offset for nice rendering (unit: points).
    pub offset: Vec2,
    pub size: Vec2,

    /// Top left corner UV in texture.
    pub min: (u16, u16),

    /// Bottom right corner (exclusive).
    pub max: (u16, u16),
}

#[derive(Clone, Copy, Debug)]
pub struct GlyphInfo {
    id: rusttype::GlyphId,

    /// Unit: points.
    pub advance_width: f32,

    /// Texture coordinates. None for space.
    pub uv_rect: Option<UvRect>,
}

impl Default for GlyphInfo {
    fn default() -> Self {
        Self {
            id: rusttype::GlyphId(0),
            advance_width: 0.0,
            uv_rect: None,
        }
    }
}

// ----------------------------------------------------------------------------

/// A specific font with a size.
/// The interface uses points as the unit for everything.
pub struct FontImpl {
    rusttype_font: Arc<rusttype::Font<'static>>,
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
        rusttype_font: Arc<rusttype::Font<'static>>,
        scale_in_points: f32,
        y_offset: f32,
    ) -> FontImpl {
        assert!(scale_in_points > 0.0);
        assert!(pixels_per_point > 0.0);

        let scale_in_pixels = pixels_per_point * scale_in_points;

        let height_in_points = scale_in_points;
        // TODO: use v_metrics for line spacing ?
        // let v = rusttype_font.v_metrics(Scale::uniform(scale_in_pixels));
        // let height_in_pixels = v.ascent - v.descent + v.line_gap;
        // let height_in_points = height_in_pixels / pixels_per_point;

        Self {
            rusttype_font,
            scale_in_pixels,
            height_in_points,
            y_offset,
            pixels_per_point,
            glyph_info_cache: Default::default(),
            atlas,
        }
    }

    /// `\n` will result in `None`
    fn glyph_info(&self, c: char) -> Option<GlyphInfo> {
        {
            if let Some(glyph_info) = self.glyph_info_cache.read().get(&c) {
                return Some(*glyph_info);
            }
        }

        // Add new character:
        let glyph = self.rusttype_font.glyph(c);
        if glyph.id().0 == 0 {
            None
        } else {
            let glyph_info = allocate_glyph(
                &mut self.atlas.lock(),
                glyph,
                self.scale_in_pixels,
                self.y_offset,
                self.pixels_per_point,
            );
            self.glyph_info_cache.write().insert(c, glyph_info);
            Some(glyph_info)
        }
    }

    pub fn pair_kerning(
        &self,
        last_glyph_id: rusttype::GlyphId,
        glyph_id: rusttype::GlyphId,
    ) -> f32 {
        let scale_in_pixels = Scale::uniform(self.scale_in_pixels);
        self.rusttype_font
            .pair_kerning(scale_in_pixels, last_glyph_id, glyph_id)
            / self.pixels_per_point
    }

    /// Height of one row of text. In points
    pub fn row_height(&self) -> f32 {
        self.height_in_points
    }

    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }
}

type FontIndex = usize;

// TODO: rename?
/// Wrapper over multiple `FontImpl` (e.g. a primary + fallbacks for emojis)
#[derive(Default)]
pub struct Font {
    fonts: Vec<Arc<FontImpl>>,
    replacement_glyph: (FontIndex, GlyphInfo),
    pixels_per_point: f32,
    row_height: f32,
    glyph_info_cache: RwLock<AHashMap<char, (FontIndex, GlyphInfo)>>,
}

impl Font {
    pub fn new(fonts: Vec<Arc<FontImpl>>) -> Self {
        if fonts.is_empty() {
            return Default::default();
        }

        let pixels_per_point = fonts[0].pixels_per_point();
        let row_height = fonts[0].row_height();

        let mut slf = Self {
            fonts,
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

        slf
    }

    pub fn round_to_pixel(&self, point: f32) -> f32 {
        (point * self.pixels_per_point).round() / self.pixels_per_point
    }

    /// Height of one row of text. In points
    pub fn row_height(&self) -> f32 {
        self.row_height
    }

    pub fn uv_rect(&self, c: char) -> Option<UvRect> {
        self.glyph_info_cache
            .read()
            .get(&c)
            .and_then(|gi| gi.1.uv_rect)
    }

    pub fn glyph_width(&self, c: char) -> f32 {
        self.glyph_info(c).1.advance_width
    }

    /// `\n` will (intentionally) show up as the replacement character.
    fn glyph_info(&self, c: char) -> (FontIndex, GlyphInfo) {
        {
            if let Some(glyph_info) = self.glyph_info_cache.read().get(&c) {
                return *glyph_info;
            }
        }

        let font_index_glyph_info = self.glyph_info_no_cache_or_fallback(c);
        let font_index_glyph_info = font_index_glyph_info.unwrap_or(self.replacement_glyph);
        self.glyph_info_cache
            .write()
            .insert(c, font_index_glyph_info);
        font_index_glyph_info
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

    /// Typeset the given text onto one row.
    /// Assumes there are no `\n` in the text.
    /// Return `x_offsets`, one longer than the number of characters in the text.
    fn layout_single_row_fragment(&self, text: &str) -> Vec<f32> {
        let mut x_offsets = Vec::with_capacity(text.chars().count() + 1);
        x_offsets.push(0.0);

        let mut cursor_x_in_points = 0.0f32;
        let mut last_glyph_id = None;

        for c in text.chars() {
            if !self.fonts.is_empty() {
                let (font_index, glyph_info) = self.glyph_info(c);
                let font_impl = &self.fonts[font_index];

                if let Some(last_glyph_id) = last_glyph_id {
                    cursor_x_in_points += font_impl.pair_kerning(last_glyph_id, glyph_info.id)
                }
                cursor_x_in_points += glyph_info.advance_width;
                cursor_x_in_points = self.round_to_pixel(cursor_x_in_points);
                last_glyph_id = Some(glyph_info.id);
            }

            x_offsets.push(cursor_x_in_points);
        }

        x_offsets
    }

    /// Typeset the given text onto one row.
    /// Any `\n` will show up as the replacement character.
    /// Always returns exactly one `Row` in the `Galley`.
    pub fn layout_single_line(&self, text: String) -> Galley {
        let x_offsets = self.layout_single_row_fragment(&text);
        let row = Row {
            x_offsets,
            y_min: 0.0,
            y_max: self.row_height(),
            ends_with_newline: false,
        };
        let width = row.max_x();
        let size = vec2(width, self.row_height());
        let galley = Galley {
            text,
            rows: vec![row],
            size,
        };
        galley.sanity_check();
        galley
    }

    /// Always returns at least one row.
    pub fn layout_multiline(&self, text: String, max_width_in_points: f32) -> Galley {
        self.layout_multiline_with_indentation_and_max_width(text, 0.0, max_width_in_points)
    }

    /// * `first_row_indentation`: extra space before the very first character (in points).
    /// * `max_width_in_points`: wrapping width.
    /// Always returns at least one row.
    pub fn layout_multiline_with_indentation_and_max_width(
        &self,
        text: String,
        first_row_indentation: f32,
        max_width_in_points: f32,
    ) -> Galley {
        let row_height = self.row_height();
        let mut cursor_y = 0.0;
        let mut rows = Vec::new();

        let mut paragraph_start = 0;

        while paragraph_start < text.len() {
            let next_newline = text[paragraph_start..].find('\n');
            let paragraph_end = next_newline
                .map(|newline| paragraph_start + newline)
                .unwrap_or_else(|| text.len());

            assert!(paragraph_start <= paragraph_end);
            let paragraph_text = &text[paragraph_start..paragraph_end];
            let line_indentation = if rows.is_empty() {
                first_row_indentation
            } else {
                0.0
            };
            let mut paragraph_rows = self.layout_paragraph_max_width(
                paragraph_text,
                line_indentation,
                max_width_in_points,
            );
            assert!(!paragraph_rows.is_empty());
            paragraph_rows.last_mut().unwrap().ends_with_newline = next_newline.is_some();

            for row in &mut paragraph_rows {
                row.y_min += cursor_y;
                row.y_max += cursor_y;
            }
            cursor_y = paragraph_rows.last().unwrap().y_max;
            cursor_y += row_height * 0.2; // Extra spacing between paragraphs. TODO: less hacky

            rows.append(&mut paragraph_rows);

            paragraph_start = paragraph_end + 1;
        }

        if text.is_empty() {
            rows.push(Row {
                x_offsets: vec![first_row_indentation],
                y_min: cursor_y,
                y_max: cursor_y + row_height,
                ends_with_newline: false,
            });
        } else if text.ends_with('\n') {
            rows.push(Row {
                x_offsets: vec![0.0],
                y_min: cursor_y,
                y_max: cursor_y + row_height,
                ends_with_newline: false,
            });
        }

        let mut widest_row = 0.0;
        for row in &rows {
            widest_row = row.max_x().max(widest_row);
        }
        let size = vec2(widest_row, rows.last().unwrap().y_max);

        let galley = Galley { text, rows, size };
        galley.sanity_check();
        galley
    }

    /// A paragraph is text with no line break character in it.
    /// The text will be wrapped by the given `max_width_in_points`.
    /// Always returns at least one row.
    fn layout_paragraph_max_width(
        &self,
        text: &str,
        mut first_row_indentation: f32,
        max_width_in_points: f32,
    ) -> Vec<Row> {
        if text.is_empty() {
            return vec![Row {
                x_offsets: vec![first_row_indentation],
                y_min: 0.0,
                y_max: self.row_height(),
                ends_with_newline: false,
            }];
        }

        let full_x_offsets = self.layout_single_row_fragment(text);

        let mut row_start_x = 0.0; // NOTE: BEFORE the `first_row_indentation`.

        let mut cursor_y = 0.0;
        let mut row_start_idx = 0;

        // Keeps track of good places to insert row break if we exceed `max_width_in_points`.
        let mut row_break_candidates = RowBreakCandidates::default();

        let mut out_rows = vec![];

        for (i, (x, chr)) in full_x_offsets.iter().skip(1).zip(text.chars()).enumerate() {
            debug_assert!(chr != '\n');
            let potential_row_width = first_row_indentation + x - row_start_x;

            if potential_row_width > max_width_in_points {
                if let Some(last_kept_index) = row_break_candidates.get() {
                    let row = Row {
                        x_offsets: full_x_offsets[row_start_idx..=last_kept_index + 1]
                            .iter()
                            .map(|x| first_row_indentation + x - row_start_x)
                            .collect(),
                        y_min: cursor_y,
                        y_max: cursor_y + self.row_height(),
                        ends_with_newline: false,
                    };
                    row.sanity_check();
                    out_rows.push(row);

                    row_start_idx = last_kept_index + 1;
                    row_start_x = first_row_indentation + full_x_offsets[row_start_idx];
                    row_break_candidates = Default::default();
                    cursor_y = self.round_to_pixel(cursor_y + self.row_height());
                } else if out_rows.is_empty() && first_row_indentation > 0.0 {
                    assert_eq!(row_start_idx, 0);
                    // Allow the first row to be completely empty, because we know there will be more space on the next row:
                    let row = Row {
                        x_offsets: vec![first_row_indentation],
                        y_min: cursor_y,
                        y_max: cursor_y + self.row_height(),
                        ends_with_newline: false,
                    };
                    row.sanity_check();
                    out_rows.push(row);
                    cursor_y = self.round_to_pixel(cursor_y + self.row_height());
                    first_row_indentation = 0.0; // Continue all other rows as if there is no indentation
                }
            }

            row_break_candidates.add(i, chr);
        }

        if row_start_idx + 1 < full_x_offsets.len() {
            let row = Row {
                x_offsets: full_x_offsets[row_start_idx..]
                    .iter()
                    .map(|x| first_row_indentation + x - row_start_x)
                    .collect(),
                y_min: cursor_y,
                y_max: cursor_y + self.row_height(),
                ends_with_newline: false,
            };
            row.sanity_check();
            out_rows.push(row);
        }

        out_rows
    }
}

/// Keeps track of good places to break a long row of text.
/// Will focus primarily on spaces, secondarily on things like `-`
#[derive(Clone, Copy, Default)]
struct RowBreakCandidates {
    /// Breaking at ` ` or other whitespace
    /// is always the primary candidate.
    space: Option<usize>,
    /// Breaking at a dash is super-
    /// good idea.
    dash: Option<usize>,
    /// This is nicer for things like URLs, e.g. www.
    /// example.com.
    punctuation: Option<usize>,
    /// Breaking after just random character is some
    /// times necessary.
    any: Option<usize>,
}

impl RowBreakCandidates {
    fn add(&mut self, index: usize, chr: char) {
        const NON_BREAKING_SPACE: char = '\u{A0}';
        if chr.is_whitespace() && chr != NON_BREAKING_SPACE {
            self.space = Some(index);
        }
        if chr == '-' {
            self.dash = Some(index);
        } else if chr.is_ascii_punctuation() {
            self.punctuation = Some(index);
        }
        self.any = Some(index);
    }

    fn get(&self) -> Option<usize> {
        self.space.or(self.dash).or(self.punctuation).or(self.any)
    }
}

fn allocate_glyph(
    atlas: &mut TextureAtlas,
    glyph: rusttype::Glyph<'static>,
    scale_in_pixels: f32,
    y_offset: f32,
    pixels_per_point: f32,
) -> GlyphInfo {
    assert!(glyph.id().0 != 0);

    let glyph = glyph.scaled(Scale::uniform(scale_in_pixels));
    let glyph = glyph.positioned(point(0.0, 0.0));

    let uv_rect = if let Some(bb) = glyph.pixel_bounding_box() {
        let glyph_width = bb.width() as usize;
        let glyph_height = bb.height() as usize;

        if glyph_width == 0 || glyph_height == 0 {
            None
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
            Some(UvRect {
                offset,
                size: vec2(glyph_width as f32, glyph_height as f32) / pixels_per_point,
                min: (glyph_pos.0 as u16, glyph_pos.1 as u16),
                max: (
                    (glyph_pos.0 + glyph_width) as u16,
                    (glyph_pos.1 + glyph_height) as u16,
                ),
            })
        }
    } else {
        // No bounding box. Maybe a space?
        None
    };

    let advance_width_in_points = glyph.unpositioned().h_metrics().advance_width / pixels_per_point;

    GlyphInfo {
        id: glyph.id(),
        advance_width: advance_width_in_points,
        uv_rect,
    }
}
