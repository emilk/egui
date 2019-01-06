use rusttype::{point, Scale};

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

// TODO: break out texture atlas into separate struct, and fill it dynamically, potentially from multiple fonts.
#[derive(Clone)]
pub struct Font {
    font: rusttype::Font<'static>,
    /// Maximum character height
    scale: usize,
    /// NUM_CHARS big
    glyph_infos: Vec<GlyphInfo>,
    atlas_width: usize,
    atlas_height: usize,
    atlas: Vec<u8>,
}

impl Font {
    pub fn new(scale: usize) -> Font {
        // TODO: figure out a way to make the wasm smaller despite including a font.
        // let font_data = include_bytes!("../fonts/ProggyClean.ttf"); // Use 13 for this. NOTHING ELSE.
        // let font_data = include_bytes!("../fonts/DejaVuSans.ttf");
        let font_data = include_bytes!("../fonts/Roboto-Regular.ttf");
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

        let mut glyph_infos = vec![];

        for glyph in glyphs {
            if let Some(bb) = glyph.pixel_bounding_box() {
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

                let offset_y = scale as i16 + bb.min.y as i16 - 4; // TODO: use font.v_metrics
                glyph_infos.push(GlyphInfo {
                    id: glyph.id(),
                    advance_width: glyph.unpositioned().h_metrics().advance_width,
                    uv: Some(UvRect {
                        offset: (bb.min.x as i16, offset_y as i16),
                        min: (cursor_x as u16, cursor_y as u16),
                        max: (
                            (cursor_x + glyph_width - 1) as u16,
                            (cursor_y + glyph_height - 1) as u16,
                        ),
                    }),
                });

                cursor_x += glyph_width;
            } else {
                // No bounding box. Maybe a space?
                glyph_infos.push(GlyphInfo {
                    id: glyph.id(),
                    advance_width: glyph.unpositioned().h_metrics().advance_width,
                    uv: None,
                });
            }
        }

        Font {
            font,
            scale,
            glyph_infos,
            atlas_width,
            atlas_height,
            atlas,
        }
    }

    pub fn line_spacing(&self) -> f32 {
        self.scale as f32
    }

    pub fn supported_characters() -> impl Iterator<Item = char> {
        (FIRST_ASCII..=LAST_ASCII).map(|c| c as u8 as char)
    }

    pub fn texture(&self) -> (u16, u16, &[u8]) {
        (
            self.atlas_width as u16,
            self.atlas_height as u16,
            &self.atlas,
        )
    }

    pub fn pixel(&self, x: u16, y: u16) -> u8 {
        let x = x as usize;
        let y = y as usize;
        assert!(x < self.atlas_width);
        assert!(y < self.atlas_height);
        self.atlas[y * self.atlas_width + x]
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

    /// Returns the start (X) of each character, starting at zero, plus the total width.
    /// i.e. returns text.chars().count() + 1 numbers.
    pub fn layout_single_line(&self, text: &str) -> Vec<f32> {
        let scale = Scale::uniform(self.scale as f32);

        let mut x_offsets = Vec::new();
        let mut cursor_x = 0.0f32;
        let mut last_glyph_id = None;
        for c in text.chars() {
            cursor_x = cursor_x.round();
            x_offsets.push(cursor_x);
            if let Some(glyph) = self.glyph_info(c) {
                if let Some(last_glyph_id) = last_glyph_id {
                    cursor_x += self.font.pair_kerning(scale, last_glyph_id, glyph.id)
                }
                cursor_x += glyph.advance_width;
                last_glyph_id = Some(glyph.id);
            }
        }
        x_offsets.push(cursor_x);
        x_offsets
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
                            let pixel = self.pixel(x as u16, y as u16);
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
        let font = Font::new(13);
        font.debug_print_atlas_ascii_art();
        font.debug_print_all_chars();
        panic!();
    }
}
