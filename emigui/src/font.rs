use std::sync::{Arc, Mutex};

use rusttype::{point, Scale};

use crate::{
    math::{vec2, Vec2},
    texture_atlas::TextureAtlas,
};

pub struct TextFragment {
    /// The start of each character, starting at zero.
    pub x_offsets: Vec<f32>,
    /// 0 for the first line, n * line_spacing for the rest
    pub y_offset: f32,
    pub text: String,
}

impl TextFragment {
    pub fn min_x(&self) -> f32 {
        *self.x_offsets.first().unwrap()
    }

    pub fn max_x(&self) -> f32 {
        *self.x_offsets.last().unwrap()
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvRect {
    /// X/Y offset for nice rendering
    pub offset: (i16, i16),

    /// Top left corner.
    pub min: (u16, u16),

    /// Inclusive
    pub max: (u16, u16),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphInfo {
    id: rusttype::GlyphId,

    pub advance_width: f32,

    /// Texture coordinates. None for space.
    pub uv: Option<UvRect>,
}

/// Printable ASCII characters [32, 126], which excludes control codes.
const FIRST_ASCII: usize = 32; // 32 == space
const LAST_ASCII: usize = 126;

#[derive(Clone)]
pub struct Font {
    font: rusttype::Font<'static>,
    /// Maximum character height
    scale: usize,
    /// NUM_CHARS big
    glyph_infos: Vec<GlyphInfo>,
    atlas: Arc<Mutex<TextureAtlas>>,
}

impl Font {
    pub fn new(atlas: Arc<Mutex<TextureAtlas>>, font_data: &'static [u8], scale: usize) -> Font {
        let font = rusttype::Font::from_bytes(font_data).expect("Error constructing Font");

        // println!(
        //     "font.v_metrics: {:?}",
        //     font.v_metrics(Scale::uniform(scale as f32))
        // );

        let glyphs: Vec<_> = Self::supported_characters()
            .map(|c| {
                let glyph = font.glyph(c);
                assert_ne!(
                    glyph.id().0,
                    0,
                    "Failed to find a glyph for the character '{}'",
                    c
                );
                let glyph = glyph.scaled(Scale::uniform(scale as f32));
                glyph.positioned(point(0.0, 0.0))
            })
            .collect();

        let mut glyph_infos = vec![];
        let mut atlas_lock = atlas.lock().unwrap();

        for glyph in glyphs {
            if let Some(bb) = glyph.pixel_bounding_box() {
                let glyph_width = bb.width() as usize;
                let glyph_height = bb.height() as usize;
                assert!(glyph_width >= 1);
                assert!(glyph_height >= 1);

                let glyph_pos = atlas_lock.allocate((glyph_width, glyph_height));

                glyph.draw(|x, y, v| {
                    if v > 0.0 {
                        let px = glyph_pos.0 + x as usize;
                        let py = glyph_pos.1 + y as usize;
                        atlas_lock[(px, py)] = (v * 255.0).round() as u8;
                    }
                });

                let offset_y = scale as i16 + bb.min.y as i16 - 4; // TODO: use font.v_metrics
                glyph_infos.push(GlyphInfo {
                    id: glyph.id(),
                    advance_width: glyph.unpositioned().h_metrics().advance_width,
                    uv: Some(UvRect {
                        offset: (bb.min.x as i16, offset_y as i16),
                        min: (glyph_pos.0 as u16, glyph_pos.1 as u16),
                        max: (
                            (glyph_pos.0 + glyph_width - 1) as u16,
                            (glyph_pos.1 + glyph_height - 1) as u16,
                        ),
                    }),
                });
            } else {
                // No bounding box. Maybe a space?
                glyph_infos.push(GlyphInfo {
                    id: glyph.id(),
                    advance_width: glyph.unpositioned().h_metrics().advance_width,
                    uv: None,
                });
            }
        }

        drop(atlas_lock);

        Font {
            font,
            scale,
            glyph_infos,
            atlas,
        }
    }

    pub fn line_spacing(&self) -> f32 {
        self.scale as f32
    }

    pub fn supported_characters() -> impl Iterator<Item = char> {
        (FIRST_ASCII..=LAST_ASCII).map(|c| c as u8 as char)
    }

    pub fn uv_rect(&self, c: char) -> Option<UvRect> {
        let c = c as usize;
        if FIRST_ASCII <= c && c <= LAST_ASCII {
            self.glyph_infos[c - FIRST_ASCII].uv
        } else {
            None
        }
    }

    fn glyph_info(&self, c: char) -> Option<GlyphInfo> {
        let c = c as usize;
        if FIRST_ASCII <= c && c <= LAST_ASCII {
            Some(self.glyph_infos[c - FIRST_ASCII])
        } else {
            None
        }
    }

    /// Returns the a single line of characters separated into words
    pub fn layout_single_line(&self, text: &str) -> Vec<TextFragment> {
        let scale = Scale::uniform(self.scale as f32);

        let mut current_fragment = TextFragment {
            x_offsets: vec![0.0],
            y_offset: 0.0,
            text: String::new(),
        };
        let mut all_fragments = vec![];
        let mut cursor_x = 0.0f32;
        let mut last_glyph_id = None;

        for c in text.chars() {
            if let Some(glyph) = self.glyph_info(c) {
                if let Some(last_glyph_id) = last_glyph_id {
                    cursor_x += self.font.pair_kerning(scale, last_glyph_id, glyph.id)
                }
                cursor_x += glyph.advance_width;
                cursor_x = cursor_x.round();
                last_glyph_id = Some(glyph.id);

                let is_space = glyph.uv.is_none();
                if is_space {
                    // TODO: also break after hyphens etc
                    if !current_fragment.text.is_empty() {
                        all_fragments.push(current_fragment);
                        current_fragment = TextFragment {
                            x_offsets: vec![cursor_x],
                            y_offset: 0.0,
                            text: String::new(),
                        }
                    }
                } else {
                    current_fragment.text.push(c);
                    current_fragment.x_offsets.push(cursor_x);
                }
            } else {
                // Ignore unknown glyph
            }
        }

        if !current_fragment.text.is_empty() {
            all_fragments.push(current_fragment)
        }
        all_fragments
    }

    pub fn layout_single_line_max_width(&self, text: &str, max_width: f32) -> Vec<TextFragment> {
        let mut words = self.layout_single_line(text);
        if words.is_empty() || words.last().unwrap().max_x() <= max_width {
            return words; // Early-out
        }

        let line_spacing = self.line_spacing();

        // Break up lines:
        let mut line_start_x = 0.0;
        let mut cursor_y = 0.0;

        for word in words.iter_mut().skip(1) {
            if word.max_x() - line_start_x >= max_width {
                // Time for a new line:
                cursor_y += line_spacing;
                line_start_x = word.min_x();
            }

            word.y_offset += cursor_y;
            for x in &mut word.x_offsets {
                *x -= line_start_x;
            }
        }

        words
    }

    /// Returns each line + total bounding box size.
    pub fn layout_multiline(&self, text: &str, max_width: f32) -> (Vec<TextFragment>, Vec2) {
        let line_spacing = self.line_spacing();
        let mut cursor_y = 0.0;
        let mut text_fragments = Vec::new();
        for line in text.split('\n') {
            let mut line_fragments = self.layout_single_line_max_width(&line, max_width);
            if let Some(last_word) = line_fragments.last() {
                let line_height = last_word.y_offset + line_spacing;
                for fragment in &mut line_fragments {
                    fragment.y_offset += cursor_y;
                }
                text_fragments.append(&mut line_fragments);
                cursor_y += line_height; // TODO: add extra spacing between paragraphs
            } else {
                cursor_y += line_spacing;
            }
            cursor_y = cursor_y.round();
        }

        let mut widest_line = 0.0;
        for fragment in &text_fragments {
            widest_line = fragment.max_x().max(widest_line);
        }

        let bounding_size = vec2(widest_line, cursor_y);
        (text_fragments, bounding_size)
    }

    pub fn debug_print_all_chars(&self) {
        let atlas_lock = self.atlas.lock().unwrap();

        let max_width = 160;
        let scale = Scale::uniform(self.scale as f32);
        let mut pixel_rows = vec![vec![0; max_width]; self.scale];
        let mut cursor_x = 0.0;
        let cursor_y = 0;
        let mut last_glyph_id = None;
        for c in Self::supported_characters() {
            if let Some(glyph) = self.glyph_info(c) {
                if let Some(last_glyph_id) = last_glyph_id {
                    cursor_x += self.font.pair_kerning(scale, last_glyph_id, glyph.id)
                }
                if cursor_x + glyph.advance_width >= max_width as f32 {
                    println!("{}", (0..max_width).map(|_| "X").collect::<String>());
                    for row in pixel_rows {
                        println!("{}", as_ascii(&row));
                    }
                    pixel_rows = vec![vec![0; max_width]; self.scale];
                    cursor_x = 0.0;
                }
                if let Some(uv) = glyph.uv {
                    for x in uv.min.0..=uv.max.0 {
                        for y in uv.min.1..=uv.max.1 {
                            let pixel = atlas_lock[(x as usize, y as usize)];
                            let rx = uv.offset.0 + x as i16 - uv.min.0 as i16;
                            let ry = uv.offset.1 + y as i16 - uv.min.1 as i16;
                            let px = (cursor_x + rx as f32).round();
                            let py = cursor_y + ry;
                            if 0.0 <= px && 0 <= py {
                                pixel_rows[py as usize][px as usize] = pixel;
                            }
                        }
                    }
                }
                cursor_x += glyph.advance_width;
                last_glyph_id = Some(glyph.id);
            }
        }
        println!("{}", (0..max_width).map(|_| "X").collect::<String>());
    }
}

fn as_ascii(pixels: &[u8]) -> String {
    pixels
        .iter()
        .map(|pixel| {
            if *pixel == 0 {
                ' '
            } else if *pixel < 85 {
                '.'
            } else if *pixel < 170 {
                'o'
            } else if *pixel < 255 {
                'O'
            } else {
                'X'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn font_test() {
        let atlas = TextureAtlas::new(128, 8);
        let atlas = Arc::new(Mutex::new(atlas));
        let font_data = include_bytes!("../fonts/Roboto-Regular.ttf");
        let font = Font::new(atlas, font_data, 13);
        font.debug_print_all_chars();
    }
}
