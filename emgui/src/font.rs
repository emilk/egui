#![allow(unused)] // TODO

use rusttype::{point, Scale};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphInfo {
    /// X offset for nice rendering
    offset_x: u16,

    /// Y offset for nice rendering
    offset_y: u16,

    min_x: u16,
    min_y: u16,

    /// Inclusive.
    max_x: u16,

    /// Inclusive
    max_y: u16,
}

/// Printable ascii characters [33, 126], which excludes 32 (space) and 127 (DEL)
const NUM_CHARS: usize = 94;
const FIRST_ASCII: usize = 33;
/// Inclusive
const LAST_ASCII: usize = 126;

#[derive(Clone)]
pub struct Font {
    /// Maximum character height
    scale: usize,
    /// NUM_CHARS big
    char_rects: Vec<GlyphInfo>,
    atlas_width: usize,
    atlas_height: usize,
    atlas: Vec<u8>,
}

impl Font {
    pub fn new(scale: usize) -> Font {
        let font_data = include_bytes!("../fonts/ProggyClean.ttf");
        let font = rusttype::Font::from_bytes(font_data as &[u8]).expect("Error constructing Font");

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

        // TODO: decide dynamically?
        let atlas_width = 128;

        let mut atlas_height = 8;
        let mut atlas = vec![0; atlas_width * atlas_height];

        // Make one white pixel for use for various stuff:
        atlas[0] = 255;

        let mut cursor_x = 1;
        let mut cursor_y = 0;
        let mut row_height = 1;

        let mut char_rects = vec![];

        for glyph in glyphs {
            let bb = glyph
                .pixel_bounding_box()
                .expect("Failed to get pixel bounding box");
            let glyph_width = bb.width() as usize;
            let glyph_height = bb.height() as usize;
            assert!(glyph_width >= 1);
            assert!(glyph_height >= 1);
            assert!(glyph_width <= atlas_width);
            if cursor_x + glyph_width > atlas_width {
                // New row:
                cursor_x = 0;
                cursor_y += row_height;
                row_height = 0;
            }

            row_height = row_height.max(glyph_height);
            while cursor_y + row_height >= atlas_height {
                atlas_height *= 2;
            }
            if atlas_width * atlas_height > atlas.len() {
                atlas.resize(atlas_width * atlas_height, 0);
            }

            glyph.draw(|x, y, v| {
                if v > 0.0 {
                    let x = x as usize;
                    let y = y as usize;
                    let px = cursor_x + x as usize;
                    let py = cursor_y + y as usize;
                    atlas[py * atlas_width + px] = (v * 255.0).round() as u8;
                }
            });

            let offset_y = scale as i32 + bb.min.y - 3; // TODO: use font.v_metrics
            assert!(0 <= bb.min.x);
            assert!(0 <= offset_y && offset_y < scale as i32);
            char_rects.push(GlyphInfo {
                offset_x: bb.min.x as u16,
                offset_y: offset_y as u16,
                min_x: cursor_x as u16,
                min_y: cursor_y as u16,
                max_x: (cursor_x + glyph_width - 1) as u16,
                max_y: (cursor_y + glyph_height - 1) as u16,
            });

            cursor_x += glyph_width;
        }

        Font {
            scale,
            char_rects,
            atlas_width,
            atlas_height,
            atlas,
        }
    }

    pub fn supported_characters() -> impl Iterator<Item = char> {
        (FIRST_ASCII..=LAST_ASCII).map(|c| c as u8 as char)
    }

    pub fn texture(&self) -> (usize, usize, &[u8]) {
        (self.atlas_width, self.atlas_height, &self.atlas)
    }

    pub fn pixel(&self, x: u16, y: u16) -> u8 {
        let x = x as usize;
        let y = y as usize;
        assert!(x < self.atlas_width);
        assert!(y < self.atlas_height);
        self.atlas[y * self.atlas_width + x]
    }

    pub fn glyph_info(&self, c: char) -> Option<GlyphInfo> {
        let c = c as usize;
        if FIRST_ASCII <= c && c <= LAST_ASCII {
            Some(self.char_rects[c - FIRST_ASCII])
        } else {
            None
        }
    }

    pub fn debug_print_atlas_ascii_art(&self) {
        for y in 0..self.atlas_height {
            println!(
                "{}",
                as_ascii(&self.atlas[y * self.atlas_width..(y + 1) * self.atlas_width])
            );
        }
    }

    pub fn debug_print_all_chars(&self) {
        let max_width = 160;
        let mut pixel_rows = vec![vec![0; max_width]; self.scale];
        let mut cursor_x = 0;
        let mut cursor_y = 0;
        for c in Self::supported_characters() {
            if let Some(glyph_info) = self.glyph_info(c) {
                for x in glyph_info.min_x..=glyph_info.max_x {
                    for y in glyph_info.min_y..=glyph_info.max_y {
                        let pixel = self.pixel(x, y);
                        let rx = glyph_info.offset_x + x - glyph_info.min_x;
                        let ry = glyph_info.offset_y + y - glyph_info.min_y;
                        pixel_rows[cursor_y + ry as usize][cursor_x + rx as usize] = pixel;
                    }
                }
                cursor_x += 7; // TODO
                if cursor_x + 7 >= max_width {
                    println!("{}", (0..max_width).map(|_| "X").collect::<String>());
                    for row in pixel_rows {
                        println!("{}", as_ascii(&row));
                    }
                    pixel_rows = vec![vec![0; max_width]; self.scale];
                    cursor_x = 0;
                }
            }
        }
        println!("{}", (0..max_width).map(|_| "X").collect::<String>());
    }
}

fn as_ascii(pixels: &[u8]) -> String {
    pixels
        .iter()
        .map(|pixel| if *pixel == 0 { ' ' } else { 'X' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn font_test() {
        let font = Font::new(13);
        font.debug_print_atlas_ascii_art();
        font.debug_print_all_chars();
        panic!();
    }
}
