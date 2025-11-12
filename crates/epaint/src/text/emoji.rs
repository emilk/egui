//! Storage for vibrant emoji sprites that we can replay into the font atlas.
//!
//! The default emoji bundle is packed into atlas PNG files that we decode at startup.
//! Smaller curated sprites (like Ferris) still use raw RGBA blobs.

use std::{convert::TryInto, sync::Arc};

use crate::{Color32, ColorImage};

struct AtlasBytes {
    png: &'static [u8],
    meta: &'static [u8],
}

#[cfg(all(feature = "emoji_low_res", feature = "emoji_high_res"))]
compile_error!("`emoji_low_res` and `emoji_high_res` are mutually exclusive");

#[cfg(all(feature = "emoji_low_res", not(feature = "emoji_high_res")))]
const NOTO_ATLAS: AtlasBytes = AtlasBytes {
    png: include_bytes!("../../assets/emoji/noto_low.atlas"),
    meta: include_bytes!("../../assets/emoji/noto_low.bin"),
};

#[cfg(all(feature = "emoji_high_res", not(feature = "emoji_low_res")))]
const NOTO_ATLAS: AtlasBytes = AtlasBytes {
    png: include_bytes!("../../assets/emoji/noto_high.atlas"),
    meta: include_bytes!("../../assets/emoji/noto_high.bin"),
};

#[cfg(all(not(feature = "emoji_low_res"), not(feature = "emoji_high_res")))]
const NOTO_ATLAS: AtlasBytes = AtlasBytes {
    png: include_bytes!("../../assets/emoji/noto_mid.atlas"),
    meta: include_bytes!("../../assets/emoji/noto_mid.bin"),
};

/// Definition of a single emoji sprite.
#[derive(Clone)]
pub(crate) struct EmojiEntry {
    pub(crate) ch: char,
    pub(crate) image: Arc<ColorImage>,
}

impl EmojiEntry {
    #[inline]
    pub(crate) fn image(&self) -> &ColorImage {
        self.image.as_ref()
    }

    #[inline]
    pub(crate) fn image_arc(&self) -> Arc<ColorImage> {
        self.image.clone()
    }
}

/// Simple container that keeps the built-in emoji bitmaps alive so we can re-upload them whenever
/// the atlas is recreated.
#[derive(Default)]
pub(crate) struct EmojiStore {
    entries: Vec<EmojiEntry>,
}

impl EmojiStore {
    /// Load the bundled sprites for the current feature configuration.
    pub(crate) fn builtin() -> Self {
        Self {
            entries: load_builtin_emojis(),
        }
    }

    #[inline]
    pub(crate) fn entries(&self) -> &[EmojiEntry] {
        &self.entries
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(all(feature = "emoji_low_res", not(feature = "emoji_high_res")))]
const FERRIS_BYTES: &[u8] = include_bytes!("../../assets/emoji/ferris_low.rgba");
#[cfg(all(feature = "emoji_low_res", not(feature = "emoji_high_res")))]
const FERRIS_SIZE: [usize; 2] = [48, 32];

#[cfg(all(not(feature = "emoji_low_res"), not(feature = "emoji_high_res")))]
const FERRIS_BYTES: &[u8] = include_bytes!("../../assets/emoji/ferris_mid.rgba");
#[cfg(all(not(feature = "emoji_low_res"), not(feature = "emoji_high_res")))]
const FERRIS_SIZE: [usize; 2] = [72, 48];

#[cfg(all(feature = "emoji_high_res", not(feature = "emoji_low_res")))]
const FERRIS_BYTES: &[u8] = include_bytes!("../../assets/emoji/ferris_high.rgba");
#[cfg(all(feature = "emoji_high_res", not(feature = "emoji_low_res")))]
const FERRIS_SIZE: [usize; 2] = [144, 96];

struct RawEmoji {
    ch: char,
    size: [usize; 2],
    bytes: &'static [u8],
}

fn load_builtin_emojis() -> Vec<EmojiEntry> {
    let mut entries = load_noto_emojis().unwrap_or_else(|err| {
        #[cfg(feature = "log")]
        log::warn!("Failed to load Noto emoji atlas: {err}");
        Vec::new()
    });
    entries.extend(load_curated_emojis());
    entries
}

fn load_curated_emojis() -> Vec<EmojiEntry> {
    // Keeping the list data-driven makes it trivial to append more sprites later.
    const RAW_EMOJIS: &[RawEmoji] = &[RawEmoji {
        ch: '🦀',
        size: FERRIS_SIZE,
        bytes: FERRIS_BYTES,
    }];

    RAW_EMOJIS
        .iter()
        .map(|raw| EmojiEntry {
            ch: raw.ch,
            image: Arc::new(ColorImage::from_rgba_unmultiplied(raw.size, raw.bytes)),
        })
        .collect()
}

fn load_noto_emojis() -> Result<Vec<EmojiEntry>, String> {
    let atlas = decode_png(NOTO_ATLAS.png)?;
    let glyphs = parse_metadata(NOTO_ATLAS.meta)?;

    let mut entries = Vec::with_capacity(glyphs.len());
    for glyph in glyphs {
        let image = Arc::new(copy_sub_image(
            &atlas,
            glyph.x as usize,
            glyph.y as usize,
            glyph.width as usize,
            glyph.height as usize,
        ));
        entries.push(EmojiEntry {
            ch: glyph.ch,
            image,
        });
    }
    Ok(entries)
}

fn decode_png(bytes: &[u8]) -> Result<ColorImage, String> {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder.read_info().map_err(|err| err.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|err| err.to_string())?;
    if info.color_type != png::ColorType::Rgba {
        return Err(format!(
            "Unsupported color type {:?} in emoji atlas",
            info.color_type
        ));
    }
    let data = &buf[..info.buffer_size()];
    Ok(ColorImage::from_rgba_unmultiplied(
        [info.width as usize, info.height as usize],
        data,
    ))
}

#[derive(Clone, Copy)]
struct GlyphMetadata {
    ch: char,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

fn parse_metadata(bytes: &[u8]) -> Result<Vec<GlyphMetadata>, String> {
    if bytes.len() < 12 {
        return Err("Emoji metadata is truncated".to_owned());
    }
    let count_offset = 8;
    let glyph_count = u32::from_le_bytes(
        bytes[count_offset..count_offset + 4]
            .try_into()
            .expect("slice len checked"),
    ) as usize;
    let mut offset = 12;
    let mut glyphs = Vec::with_capacity(glyph_count);
    while offset + 12 <= bytes.len() {
        let codepoint = u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("slice len checked"),
        );
        let ch = char::from_u32(codepoint)
            .ok_or_else(|| format!("Invalid codepoint in emoji metadata: {codepoint:#x}"))?;
        let x = u16::from_le_bytes(bytes[offset + 4..offset + 6].try_into().unwrap());
        let y = u16::from_le_bytes(bytes[offset + 6..offset + 8].try_into().unwrap());
        let width = u16::from_le_bytes(bytes[offset + 8..offset + 10].try_into().unwrap());
        let height = u16::from_le_bytes(bytes[offset + 10..offset + 12].try_into().unwrap());
        glyphs.push(GlyphMetadata {
            ch,
            x,
            y,
            width,
            height,
        });
        offset += 12;
    }
    if glyphs.len() != glyph_count {
        return Err(format!(
            "Emoji metadata expected {glyph_count} glyphs but contained {}",
            glyphs.len()
        ));
    }
    Ok(glyphs)
}

fn copy_sub_image(
    source: &ColorImage,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> ColorImage {
    let mut out = ColorImage::filled([width, height], Color32::TRANSPARENT);
    let src_width = source.width();
    for row in 0..height {
        let src_start = (y + row) * src_width + x;
        let src_end = src_start + width;
        let dst_start = row * width;
        out.pixels[dst_start..dst_start + width]
            .copy_from_slice(&source.pixels[src_start..src_end]);
    }
    out
}
