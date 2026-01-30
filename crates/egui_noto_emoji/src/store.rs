//! Storage for vibrant emoji sprites that we can replay into the font atlas.
//!
//! The default emoji bundle is packed into atlas PNG files that we decode at startup.
//! Smaller curated sprites (like Ferris) still use raw RGBA blobs.
//!
//! When `emoji_high_res` feature is enabled, all three resolutions (low/mid/high) are loaded
//! for better rendering at all font sizes.

use std::{collections::HashMap, convert::TryInto as _, sync::Arc};

use egui::{Color32, ColorImage};

struct AtlasBytes {
    png: &'static [u8],
    meta: &'static [u8],
}

// Load all three atlases when high_res is enabled for multi-resolution support.
#[expect(
    clippy::large_include_file,
    reason = "emoji atlas is intentionally large"
)]
#[cfg(feature = "emoji_high_res")]
const NOTO_ATLAS_LOW: AtlasBytes = AtlasBytes {
    png: include_bytes!("../assets/emoji/noto_low.atlas"),
    meta: include_bytes!("../assets/emoji/noto_low.bin"),
};

#[expect(
    clippy::large_include_file,
    reason = "emoji atlas is intentionally large"
)]
#[cfg(feature = "emoji_high_res")]
const NOTO_ATLAS_MID: AtlasBytes = AtlasBytes {
    png: include_bytes!("../assets/emoji/noto_mid.atlas"),
    meta: include_bytes!("../assets/emoji/noto_mid.bin"),
};

#[expect(
    clippy::large_include_file,
    reason = "emoji atlas is intentionally large"
)]
#[cfg(feature = "emoji_high_res")]
const NOTO_ATLAS_HIGH: AtlasBytes = AtlasBytes {
    png: include_bytes!("../assets/emoji/noto_high.atlas"),
    meta: include_bytes!("../assets/emoji/noto_high.bin"),
};

// Single atlas when not using high_res
#[expect(
    clippy::large_include_file,
    reason = "emoji atlas is intentionally large"
)]
#[cfg(all(feature = "emoji_low_res", not(feature = "emoji_high_res")))]
const NOTO_ATLAS: AtlasBytes = AtlasBytes {
    png: include_bytes!("../assets/emoji/noto_low.atlas"),
    meta: include_bytes!("../assets/emoji/noto_low.bin"),
};

#[expect(
    clippy::large_include_file,
    reason = "emoji atlas is intentionally large"
)]
#[cfg(all(not(feature = "emoji_low_res"), not(feature = "emoji_high_res")))]
const NOTO_ATLAS: AtlasBytes = AtlasBytes {
    png: include_bytes!("../assets/emoji/noto_mid.atlas"),
    meta: include_bytes!("../assets/emoji/noto_mid.bin"),
};

/// A single resolution image for an emoji.
#[derive(Clone)]
pub struct EmojiResolution {
    /// The native pixel height of this image.
    pub size_px: u16,

    /// The image data.
    pub image: Arc<ColorImage>,
}

/// Definition of a single emoji with potentially multiple resolutions.
#[derive(Clone)]
pub struct EmojiEntry {
    pub(crate) ch: char,

    /// Available resolutions, sorted by size (smallest first).
    pub(crate) resolutions: Vec<EmojiResolution>,
}

impl EmojiEntry {
    /// The character (codepoint) this emoji represents.
    #[inline]
    pub fn ch(&self) -> char {
        self.ch
    }

    /// Returns the resolutions as (`size_px`, image) tuples for registration.
    #[inline]
    pub fn resolutions(&self) -> Vec<(u16, Arc<ColorImage>)> {
        self.resolutions
            .iter()
            .map(|r| (r.size_px, Arc::clone(&r.image)))
            .collect()
    }

    /// Returns a reference to the largest resolution emoji image.
    /// For backwards compatibility.
    #[inline]
    pub fn image(&self) -> &ColorImage {
        self.resolutions
            .last()
            .map(|r| r.image.as_ref())
            .unwrap_or_else(|| self.resolutions[0].image.as_ref())
    }

    /// Returns a shared reference to the largest resolution emoji image.
    /// For backwards compatibility.
    #[inline]
    pub fn image_arc(&self) -> Arc<ColorImage> {
        self.resolutions
            .last()
            .map(|r| Arc::clone(&r.image))
            .unwrap_or_else(|| Arc::clone(&self.resolutions[0].image))
    }
}

/// Simple container that keeps the built-in emoji bitmaps alive so we can re-upload them whenever
/// the atlas is recreated.
#[derive(Default)]
pub struct EmojiStore {
    entries: Vec<EmojiEntry>,
}

impl EmojiStore {
    /// Load the bundled sprites for the current feature configuration.
    pub fn builtin() -> Self {
        Self {
            entries: load_builtin_emojis(),
        }
    }

    /// Returns all emoji entries in this store.
    #[inline]
    pub fn entries(&self) -> &[EmojiEntry] {
        &self.entries
    }

    /// Returns `true` if this store contains no emoji entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// Ferris emoji at all resolutions (when high_res enabled)
#[cfg(feature = "emoji_high_res")]
const FERRIS_LOW: (&[u8], [usize; 2]) =
    (include_bytes!("../assets/emoji/ferris_low.rgba"), [48, 32]);
#[cfg(feature = "emoji_high_res")]
const FERRIS_MID: (&[u8], [usize; 2]) =
    (include_bytes!("../assets/emoji/ferris_mid.rgba"), [72, 48]);
#[cfg(feature = "emoji_high_res")]
const FERRIS_HIGH: (&[u8], [usize; 2]) = (
    include_bytes!("../assets/emoji/ferris_high.rgba"),
    [144, 96],
);

// Single Ferris resolution when not using high_res
#[cfg(all(feature = "emoji_low_res", not(feature = "emoji_high_res")))]
const FERRIS_SINGLE: (&[u8], [usize; 2]) =
    (include_bytes!("../assets/emoji/ferris_low.rgba"), [48, 32]);

#[cfg(all(not(feature = "emoji_low_res"), not(feature = "emoji_high_res")))]
const FERRIS_SINGLE: (&[u8], [usize; 2]) =
    (include_bytes!("../assets/emoji/ferris_mid.rgba"), [72, 48]);

fn load_builtin_emojis() -> Vec<EmojiEntry> {
    let mut entries = load_noto_emojis().unwrap_or_else(|err| {
        #[cfg(feature = "log")]
        log::warn!("Failed to load Noto emoji atlas: {err}");
        #[cfg(not(feature = "log"))]
        let _ = err;
        Vec::new()
    });
    entries.extend(load_curated_emojis());
    entries
}

#[cfg(feature = "emoji_high_res")]
fn load_curated_emojis() -> Vec<EmojiEntry> {
    // Ferris with all three resolutions for sharp rendering at all sizes
    let ferris_resolutions = vec![
        EmojiResolution {
            size_px: FERRIS_LOW.1[1] as u16,
            image: Arc::new(ColorImage::from_rgba_unmultiplied(
                FERRIS_LOW.1,
                FERRIS_LOW.0,
            )),
        },
        EmojiResolution {
            size_px: FERRIS_MID.1[1] as u16,
            image: Arc::new(ColorImage::from_rgba_unmultiplied(
                FERRIS_MID.1,
                FERRIS_MID.0,
            )),
        },
        EmojiResolution {
            size_px: FERRIS_HIGH.1[1] as u16,
            image: Arc::new(ColorImage::from_rgba_unmultiplied(
                FERRIS_HIGH.1,
                FERRIS_HIGH.0,
            )),
        },
    ];

    vec![EmojiEntry {
        ch: '🦀',
        resolutions: ferris_resolutions,
    }]
}

#[cfg(not(feature = "emoji_high_res"))]
fn load_curated_emojis() -> Vec<EmojiEntry> {
    // Single resolution Ferris
    vec![EmojiEntry {
        ch: '🦀',
        resolutions: vec![EmojiResolution {
            size_px: FERRIS_SINGLE.1[1] as u16,
            image: Arc::new(ColorImage::from_rgba_unmultiplied(
                FERRIS_SINGLE.1,
                FERRIS_SINGLE.0,
            )),
        }],
    }]
}

/// Load Noto emojis with multi-resolution support when `emoji_high_res` is enabled.
#[cfg(feature = "emoji_high_res")]
#[expect(
    clippy::iter_over_hash_type,
    reason = "order doesn't matter for emoji entries"
)]
fn load_noto_emojis() -> Result<Vec<EmojiEntry>, String> {
    // Load all three atlases
    let atlas_low = decode_png(NOTO_ATLAS_LOW.png)?;
    let glyphs_low = parse_metadata(NOTO_ATLAS_LOW.meta)?;

    let atlas_mid = decode_png(NOTO_ATLAS_MID.png)?;
    let glyphs_mid = parse_metadata(NOTO_ATLAS_MID.meta)?;

    let atlas_high = decode_png(NOTO_ATLAS_HIGH.png)?;
    let glyphs_high = parse_metadata(NOTO_ATLAS_HIGH.meta)?;

    // Build a map: char -> Vec<(size_px, image)>
    let mut char_to_resolutions: HashMap<char, Vec<EmojiResolution>> = HashMap::new();

    // Add low resolution sprites
    for glyph in &glyphs_low {
        if let Some(image) = copy_sub_image(
            &atlas_low,
            glyph.x as usize,
            glyph.y as usize,
            glyph.width as usize,
            glyph.height as usize,
        ) {
            char_to_resolutions
                .entry(glyph.ch)
                .or_default()
                .push(EmojiResolution {
                    size_px: glyph.height,
                    image: Arc::new(image),
                });
        }
    }

    // Add mid resolution sprites
    for glyph in &glyphs_mid {
        if let Some(image) = copy_sub_image(
            &atlas_mid,
            glyph.x as usize,
            glyph.y as usize,
            glyph.width as usize,
            glyph.height as usize,
        ) {
            char_to_resolutions
                .entry(glyph.ch)
                .or_default()
                .push(EmojiResolution {
                    size_px: glyph.height,
                    image: Arc::new(image),
                });
        }
    }

    // Add high resolution sprites
    for glyph in &glyphs_high {
        if let Some(image) = copy_sub_image(
            &atlas_high,
            glyph.x as usize,
            glyph.y as usize,
            glyph.width as usize,
            glyph.height as usize,
        ) {
            char_to_resolutions
                .entry(glyph.ch)
                .or_default()
                .push(EmojiResolution {
                    size_px: glyph.height,
                    image: Arc::new(image),
                });
        }
    }

    // Convert to entries, sorting resolutions by size
    let mut entries = Vec::with_capacity(char_to_resolutions.len());
    for (ch, mut resolutions) in char_to_resolutions {
        resolutions.sort_by_key(|r| r.size_px);
        entries.push(EmojiEntry { ch, resolutions });
    }

    #[cfg(feature = "log")]
    log::info!(
        "Loaded {} emoji with multi-resolution support (low/mid/high)",
        entries.len()
    );

    Ok(entries)
}

/// Load Noto emojis from single atlas (non-high_res mode).
#[cfg(not(feature = "emoji_high_res"))]
fn load_noto_emojis() -> Result<Vec<EmojiEntry>, String> {
    let atlas = decode_png(NOTO_ATLAS.png)?;
    let glyphs = parse_metadata(NOTO_ATLAS.meta)?;

    let mut entries = Vec::with_capacity(glyphs.len());
    for glyph in glyphs {
        let Some(image) = copy_sub_image(
            &atlas,
            glyph.x as usize,
            glyph.y as usize,
            glyph.width as usize,
            glyph.height as usize,
        ) else {
            #[cfg(feature = "log")]
            log::warn!(
                "Skipping emoji glyph {:?} (U+{:04X}) with invalid bounds",
                glyph.ch,
                glyph.ch as u32,
            );
            continue;
        };
        entries.push(EmojiEntry {
            ch: glyph.ch,
            resolutions: vec![EmojiResolution {
                size_px: glyph.height,
                image: Arc::new(image),
            }],
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

/// Binary metadata format (version 1):
/// - Bytes 0-3: Magic number (0x00001000)
/// - Bytes 4-7: Atlas height in pixels (u32 LE) - used for sanity checking
/// - Bytes 8-11: Glyph count (u32 LE)
/// - Bytes 12+: Glyph entries (12 bytes each: codepoint u32, x u16, y u16, w u16, h u16)
const METADATA_MAGIC: [u8; 4] = [0x00, 0x10, 0x00, 0x00];

/// Maximum reasonable atlas height (16K pixels should be more than enough)
const MAX_ATLAS_HEIGHT: u32 = 16384;

fn parse_metadata(bytes: &[u8]) -> Result<Vec<GlyphMetadata>, String> {
    if bytes.len() < 12 {
        return Err("Emoji metadata is truncated".to_owned());
    }

    // Validate magic number (bytes 0-3)
    if bytes[0..4] != METADATA_MAGIC {
        return Err(format!(
            "Invalid emoji metadata magic: expected {:02x?}, got {:02x?}",
            METADATA_MAGIC,
            &bytes[0..4]
        ));
    }

    let read_u32 = |slice: &[u8]| -> Result<u32, String> {
        let arr: [u8; 4] = slice
            .try_into()
            .map_err(|_err| "Failed to read u32 from metadata".to_owned())?;
        Ok(u32::from_le_bytes(arr))
    };

    let read_u16 = |slice: &[u8]| -> Result<u16, String> {
        let arr: [u8; 2] = slice
            .try_into()
            .map_err(|_err| "Failed to read u16 from metadata".to_owned())?;
        Ok(u16::from_le_bytes(arr))
    };

    // Validate atlas height (bytes 4-7) - sanity check for corrupted files
    let atlas_height = read_u32(&bytes[4..8])?;
    if atlas_height == 0 || atlas_height > MAX_ATLAS_HEIGHT {
        return Err(format!(
            "Invalid atlas height in emoji metadata: {atlas_height} (expected 1-{MAX_ATLAS_HEIGHT})"
        ));
    }

    let count_offset = 8;
    let glyph_count = read_u32(&bytes[count_offset..count_offset + 4])? as usize;
    let mut offset = 12;
    let mut glyphs = Vec::with_capacity(glyph_count);
    while offset + 12 <= bytes.len() {
        let codepoint = read_u32(&bytes[offset..offset + 4])?;
        let ch = char::from_u32(codepoint)
            .ok_or_else(|| format!("Invalid codepoint in emoji metadata: {codepoint:#x}"))?;
        let x = read_u16(&bytes[offset + 4..offset + 6])?;
        let y = read_u16(&bytes[offset + 6..offset + 8])?;
        let width = read_u16(&bytes[offset + 8..offset + 10])?;
        let height = read_u16(&bytes[offset + 10..offset + 12])?;
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
) -> Option<ColorImage> {
    // Bounds validation to prevent panic on corrupted metadata
    if width == 0 || height == 0 {
        return None;
    }
    let x_end = x.checked_add(width)?;
    let y_end = y.checked_add(height)?;
    if x_end > source.width() || y_end > source.height() {
        return None;
    }

    let mut out = ColorImage::filled([width, height], Color32::TRANSPARENT);
    let src_width = source.width();
    for row in 0..height {
        let src_start = (y + row) * src_width + x;
        let src_end = src_start + width;
        let dst_start = row * width;
        out.pixels[dst_start..dst_start + width]
            .copy_from_slice(&source.pixels[src_start..src_end]);
    }
    Some(out)
}
