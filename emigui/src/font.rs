use std::{collections::BTreeMap, sync::Arc};

use parking_lot::Mutex;
use rusttype::{point, Scale};

use crate::{
    math::{vec2, Vec2},
    texture_atlas::TextureAtlas,
};

/// A typeset piece of text on a single line. Could be a whole line, or just a word.
pub struct Fragment {
    /// The start of each character, starting at zero.
    /// Unit: points.
    pub x_offsets: Vec<f32>,

    /// 0 for the first line, n * line_spacing for the rest
    /// Unit: points.
    pub y_offset: f32,

    /// The actual characters
    pub text: String,
}

impl Fragment {
    pub fn min_x(&self) -> f32 {
        *self.x_offsets.first().unwrap()
    }

    pub fn max_x(&self) -> f32 {
        *self.x_offsets.last().unwrap()
    }
}

// pub fn fn_text_width(fragmens: &[Fragment]) -> f32 {
//     if fragmens.is_empty() {
//         0.0
//     } else {
//         fragmens.last().unwrap().max_x() - fragmens.first().unwrap().min_x()
//     }
// }

/// A collection of text locked into place.
pub struct Galley {
    pub fragments: Vec<Fragment>,
    pub size: Vec2,
}

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

/// The interface uses points as the unit for everything.
#[derive(Clone)]
pub struct Font {
    font: rusttype::Font<'static>,
    /// Maximum character height
    scale_in_pixels: f32,
    pixels_per_point: f32,
    glyph_infos: BTreeMap<char, GlyphInfo>,
    atlas: Arc<Mutex<TextureAtlas>>,
}

impl Font {
    pub fn new(
        atlas: Arc<Mutex<TextureAtlas>>,
        font_data: &'static [u8],
        scale_in_points: f32,
        pixels_per_point: f32,
    ) -> Font {
        let font = rusttype::Font::try_from_bytes(font_data).expect("Error constructing Font");
        let scale_in_pixels = pixels_per_point * scale_in_points;

        let mut font = Font {
            font,
            scale_in_pixels,
            pixels_per_point,
            glyph_infos: Default::default(),
            atlas,
        };

        /// Printable ASCII characters [32, 126], which excludes control codes.
        const FIRST_ASCII: usize = 32; // 32 == space
        const LAST_ASCII: usize = 126;
        for c in (FIRST_ASCII..=LAST_ASCII).map(|c| c as u8 as char) {
            font.add_char(c);
        }
        font
    }

    pub fn round_to_pixel(&self, point: f32) -> f32 {
        (point * self.pixels_per_point).round() / self.pixels_per_point
    }

    /// Height of one line of text. In points
    /// TODO: rename height ?
    pub fn line_spacing(&self) -> f32 {
        self.scale_in_pixels / self.pixels_per_point
    }
    pub fn height(&self) -> f32 {
        self.scale_in_pixels / self.pixels_per_point
    }

    pub fn uv_rect(&self, c: char) -> Option<UvRect> {
        self.glyph_infos.get(&c).and_then(|gi| gi.uv_rect)
    }

    fn glyph_info(&self, c: char) -> Option<&GlyphInfo> {
        self.glyph_infos.get(&c)
    }

    fn add_char(&mut self, c: char) {
        let glyph = self.font.glyph(c);
        assert_ne!(
            glyph.id().0,
            0,
            "Failed to find a glyph for the character '{}'",
            c
        );
        let glyph = glyph.scaled(Scale::uniform(self.scale_in_pixels));
        let glyph = glyph.positioned(point(0.0, 0.0));

        let uv_rect = if let Some(bb) = glyph.pixel_bounding_box() {
            let glyph_width = bb.width() as usize;
            let glyph_height = bb.height() as usize;
            assert!(glyph_width >= 1);
            assert!(glyph_height >= 1);

            let mut atlas_lock = self.atlas.lock();
            let glyph_pos = atlas_lock.allocate((glyph_width, glyph_height));

            let texture = atlas_lock.texture_mut();
            glyph.draw(|x, y, v| {
                if v > 0.0 {
                    let px = glyph_pos.0 + x as usize;
                    let py = glyph_pos.1 + y as usize;
                    texture[(px, py)] = (v * 255.0).round() as u8;
                }
            });

            let offset_y_in_pixels =
                self.scale_in_pixels as f32 + bb.min.y as f32 - 4.0 * self.pixels_per_point; // TODO: use font.v_metrics
            Some(UvRect {
                offset: vec2(
                    bb.min.x as f32 / self.pixels_per_point,
                    offset_y_in_pixels / self.pixels_per_point,
                ),
                size: vec2(glyph_width as f32, glyph_height as f32) / self.pixels_per_point,
                min: (glyph_pos.0 as u16, glyph_pos.1 as u16),
                max: (
                    (glyph_pos.0 + glyph_width) as u16,
                    (glyph_pos.1 + glyph_height) as u16,
                ),
            })
        } else {
            // No bounding box. Maybe a space?
            None
        };

        let advance_width_in_points =
            glyph.unpositioned().h_metrics().advance_width / self.pixels_per_point;

        self.glyph_infos.insert(
            c,
            GlyphInfo {
                id: glyph.id(),
                advance_width: advance_width_in_points,
                uv_rect,
            },
        );
    }

    /// Returns the a single line of characters separated into words
    /// Always returns at least one frament.
    fn layout_words(&self, text: &str) -> Galley {
        let scale_in_pixels = Scale::uniform(self.scale_in_pixels);

        let mut current_fragment = Fragment {
            x_offsets: vec![0.0],
            y_offset: 0.0,
            text: String::new(),
        };
        let mut fragments = vec![];
        let mut cursor_x_in_points = 0.0f32;
        let mut last_glyph_id = None;

        for c in text.chars() {
            if let Some(glyph) = self.glyph_info(c) {
                if let Some(last_glyph_id) = last_glyph_id {
                    cursor_x_in_points +=
                        self.font
                            .pair_kerning(scale_in_pixels, last_glyph_id, glyph.id)
                            / self.pixels_per_point
                }
                cursor_x_in_points += glyph.advance_width;
                cursor_x_in_points = self.round_to_pixel(cursor_x_in_points);
                last_glyph_id = Some(glyph.id);

                let is_space = glyph.uv_rect.is_none();
                if is_space {
                    // TODO: also break after hyphens etc
                    if !current_fragment.text.is_empty() {
                        fragments.push(current_fragment);
                        current_fragment = Fragment {
                            x_offsets: vec![cursor_x_in_points],
                            y_offset: 0.0,
                            text: String::new(),
                        }
                    }
                // TODO: add a fragment for the space aswell
                } else {
                    current_fragment.text.push(c);
                    current_fragment.x_offsets.push(cursor_x_in_points);
                }
            } else {
                // Ignore unknown glyph
            }
        }

        if !current_fragment.text.is_empty() {
            fragments.push(current_fragment)
        }

        let width = if fragments.is_empty() {
            0.0
        } else {
            fragments.last().unwrap().max_x()
        };

        let size = vec2(width, self.height());

        Galley { fragments, size }
    }

    /// Typeset the given text onto one line.
    /// Always returns at least one frament.
    pub fn layout_single_line(&self, text: &str) -> Galley {
        // TODO: return a single Fragment instead of calling layout_words
        // saves a lot of allocations
        self.layout_words(text)
    }

    /// A paragraph is text with no line break character in it.
    /// The text will be linebreaked by the given max_width_in_points.
    /// TODO: return Galley ?
    pub fn layout_paragraph_max_width(
        &self,
        text: &str,
        max_width_in_points: f32,
    ) -> Vec<Fragment> {
        let mut galley = self.layout_words(text);
        if galley.fragments.is_empty() || galley.size.x <= max_width_in_points {
            return galley.fragments; // Early-out
        }

        let line_spacing = self.line_spacing();

        // Break up lines:
        let mut line_start_x = 0.0;
        let mut cursor_y = 0.0;

        for word in galley.fragments.iter_mut().skip(1) {
            if word.max_x() - line_start_x >= max_width_in_points {
                // Time for a new line:
                cursor_y += line_spacing;
                line_start_x = word.min_x();
            }

            word.y_offset += cursor_y;
            for x in &mut word.x_offsets {
                *x -= line_start_x;
            }
        }

        galley.fragments
    }

    pub fn layout_multiline(&self, text: &str, max_width_in_points: f32) -> Galley {
        let line_spacing = self.line_spacing();
        let mut cursor_y = 0.0;
        let mut fragments = Vec::new();
        for line in text.split('\n') {
            let mut line_fragments = self.layout_paragraph_max_width(line, max_width_in_points);
            if let Some(last_word) = line_fragments.last() {
                let line_height = last_word.y_offset + line_spacing;
                for fragment in &mut line_fragments {
                    fragment.y_offset += cursor_y;
                }
                fragments.append(&mut line_fragments);
                cursor_y += line_height; // TODO: add extra spacing between paragraphs
            } else {
                cursor_y += line_spacing;
            }
            cursor_y = self.round_to_pixel(cursor_y);
        }

        let mut widest_line = 0.0;
        for fragment in &fragments {
            widest_line = fragment.max_x().max(widest_line);
        }

        Galley {
            fragments,
            size: vec2(widest_line, cursor_y),
        }
    }
}
