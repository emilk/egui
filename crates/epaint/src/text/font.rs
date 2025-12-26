#![expect(clippy::mem_forget)]

use emath::{GuiRounding as _, OrderedFloat, Vec2, vec2};
use self_cell::self_cell;
use skrifa::{
    MetadataProvider as _,
    raw::{TableProvider as _, tables::kern::SubtableKind},
};
use std::collections::BTreeMap;
use vello_cpu::{color, kurbo};

use crate::{
    TextOptions, TextureAtlas,
    text::{
        FontTweak,
        fonts::{Blob, CachedFamily, FontFaceKey},
    },
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphInfo {
    /// Used for pair-kerning.
    ///
    /// Doesn't need to be unique.
    ///
    /// Is `None` for a special "invisible" glyph.
    pub(crate) id: Option<skrifa::GlyphId>,

    /// In [`skrifa`]s "unscaled" coordinate system.
    pub advance_width_unscaled: OrderedFloat<f32>,
}

impl GlyphInfo {
    /// A valid, but invisible, glyph of zero-width.
    pub const INVISIBLE: Self = Self {
        id: None,
        advance_width_unscaled: OrderedFloat(0.0),
    };
}

// Subpixel binning, taken from cosmic-text:
// https://github.com/pop-os/cosmic-text/blob/974ddaed96b334f560b606ebe5d2ca2d2f9f23ef/src/glyph_cache.rs

/// Bin for subpixel positioning of glyphs.
///
/// For accurate glyph positioning, we want to render each glyph at a subpixel coordinate. However, we also want to
/// cache each glyph's bitmap. As a compromise, we bin each subpixel offset into one of four fractional values. This
/// means one glyph can have up to four subpixel-positioned bitmaps in the cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(super) enum SubpixelBin {
    #[default]
    Zero,
    One,
    Two,
    Three,
}

impl SubpixelBin {
    /// Bin the given position and return the new integral coordinate.
    fn new(pos: f32) -> (i32, Self) {
        let trunc = pos as i32;
        let fract = pos - trunc as f32;

        #[expect(clippy::collapsible_else_if)]
        if pos.is_sign_negative() {
            if fract > -0.125 {
                (trunc, Self::Zero)
            } else if fract > -0.375 {
                (trunc - 1, Self::Three)
            } else if fract > -0.625 {
                (trunc - 1, Self::Two)
            } else if fract > -0.875 {
                (trunc - 1, Self::One)
            } else {
                (trunc - 1, Self::Zero)
            }
        } else {
            if fract < 0.125 {
                (trunc, Self::Zero)
            } else if fract < 0.375 {
                (trunc, Self::One)
            } else if fract < 0.625 {
                (trunc, Self::Two)
            } else if fract < 0.875 {
                (trunc, Self::Three)
            } else {
                (trunc + 1, Self::Zero)
            }
        }
    }

    pub fn as_float(&self) -> f32 {
        match self {
            Self::Zero => 0.0,
            Self::One => 0.25,
            Self::Two => 0.5,
            Self::Three => 0.75,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct GlyphAllocation {
    /// Used for pair-kerning.
    ///
    /// Doesn't need to be unique.
    /// Use [`skrifa::GlyphId::NOTDEF`] if you just want to have an id, and don't care.
    pub(crate) id: skrifa::GlyphId,

    /// Unit: screen pixels.
    pub advance_width_px: f32,

    /// UV rectangle for drawing.
    pub uv_rect: UvRect,
}

#[derive(Hash, PartialEq, Eq)]
struct GlyphCacheKey(u64);

impl nohash_hasher::IsEnabled for GlyphCacheKey {}

impl GlyphCacheKey {
    fn new(glyph_id: skrifa::GlyphId, metrics: &ScaledMetrics, bin: SubpixelBin) -> Self {
        let ScaledMetrics {
            pixels_per_point,
            px_scale_factor,
            ..
        } = *metrics;
        debug_assert!(
            0.0 < pixels_per_point && pixels_per_point.is_finite(),
            "Bad pixels_per_point {pixels_per_point}"
        );
        debug_assert!(
            0.0 < px_scale_factor && px_scale_factor.is_finite(),
            "Bad px_scale_factor: {px_scale_factor}"
        );
        Self(crate::util::hash((
            glyph_id,
            pixels_per_point.to_bits(),
            px_scale_factor.to_bits(),
            bin,
        )))
    }
}

// ----------------------------------------------------------------------------

struct DependentFontData<'a> {
    skrifa: skrifa::FontRef<'a>,
    charmap: skrifa::charmap::Charmap<'a>,
    outline_glyphs: skrifa::outline::OutlineGlyphCollection<'a>,
    metrics: skrifa::metrics::Metrics,
    glyph_metrics: skrifa::metrics::GlyphMetrics<'a>,
    hinting_instance: Option<skrifa::outline::HintingInstance>,
}

self_cell! {
    struct FontCell {
        owner: Blob,

        #[covariant]
        dependent: DependentFontData,
    }
}

impl FontCell {
    fn px_scale_factor(&self, scale: f32) -> f32 {
        let units_per_em = self.borrow_dependent().metrics.units_per_em as f32;
        scale / units_per_em
    }

    fn allocate_glyph_uncached(
        &mut self,
        atlas: &mut TextureAtlas,
        metrics: &ScaledMetrics,
        glyph_info: &GlyphInfo,
        bin: SubpixelBin,
        location: &skrifa::instance::Location,
    ) -> Option<GlyphAllocation> {
        let glyph_id = glyph_info.id?;

        debug_assert!(
            glyph_id != skrifa::GlyphId::NOTDEF,
            "Can't allocate glyph for id 0"
        );

        let mut path = kurbo::BezPath::new();
        let mut pen = VelloPen {
            path: &mut path,
            x_offset: bin.as_float() as f64,
        };

        self.with_dependent_mut(|_, font_data| {
            let outline = font_data.outline_glyphs.get(glyph_id)?;

            if let Some(hinting_instance) = &mut font_data.hinting_instance {
                let size = skrifa::instance::Size::new(metrics.scale);
                if hinting_instance.size() != size {
                    hinting_instance
                        .reconfigure(
                            &font_data.outline_glyphs,
                            size,
                            location,
                            skrifa::outline::Target::Smooth {
                                mode: skrifa::outline::SmoothMode::Normal,
                                symmetric_rendering: true,
                                preserve_linear_metrics: true,
                            },
                        )
                        .ok()?;
                }
                let draw_settings = skrifa::outline::DrawSettings::hinted(hinting_instance, false);
                outline.draw(draw_settings, &mut pen).ok()?;
            } else {
                let draw_settings = skrifa::outline::DrawSettings::unhinted(
                    skrifa::instance::Size::new(metrics.scale),
                    location,
                );
                outline.draw(draw_settings, &mut pen).ok()?;
            }

            Some(())
        })?;

        let bounds = path.control_box().expand();
        let width = bounds.width() as u16;
        let height = bounds.height() as u16;

        let mut ctx = vello_cpu::RenderContext::new(width, height);
        ctx.set_transform(kurbo::Affine::translate((-bounds.x0, -bounds.y0)));
        ctx.set_paint(color::OpaqueColor::<color::Srgb>::WHITE);
        ctx.fill_path(&path);
        let mut dest = vello_cpu::Pixmap::new(width, height);
        ctx.render_to_pixmap(&mut dest);
        let uv_rect = if width == 0 || height == 0 {
            UvRect::default()
        } else {
            let glyph_pos = {
                let alpha_from_coverage = atlas.options().alpha_from_coverage;
                let (glyph_pos, image) = atlas.allocate((width as usize, height as usize));
                let pixels = dest.data_as_u8_slice();
                for y in 0..height as usize {
                    for x in 0..width as usize {
                        image[(x + glyph_pos.0, y + glyph_pos.1)] = alpha_from_coverage
                            .color_from_coverage(
                                pixels[((y * width as usize) + x) * 4 + 3] as f32 / 255.0,
                            );
                    }
                }
                glyph_pos
            };
            let offset_in_pixels = vec2(bounds.x0 as f32, bounds.y0 as f32);
            let offset =
                offset_in_pixels / metrics.pixels_per_point + metrics.y_offset_in_points * Vec2::Y;
            UvRect {
                offset,
                size: vec2(width as f32, height as f32) / metrics.pixels_per_point,
                min: [glyph_pos.0 as u16, glyph_pos.1 as u16],
                max: [
                    (glyph_pos.0 + width as usize) as u16,
                    (glyph_pos.1 + height as usize) as u16,
                ],
            }
        };

        Some(GlyphAllocation {
            id: glyph_id,
            advance_width_px: glyph_info.advance_width_unscaled.0 * metrics.px_scale_factor,
            uv_rect,
        })
    }
}

struct VelloPen<'a> {
    path: &'a mut kurbo::BezPath,
    x_offset: f64,
}

impl skrifa::outline::OutlinePen for VelloPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to((x as f64 + self.x_offset, -y as f64));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to((x as f64 + self.x_offset, -y as f64));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.path.quad_to(
            (cx0 as f64 + self.x_offset, -cy0 as f64),
            (x as f64 + self.x_offset, -y as f64),
        );
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.path.curve_to(
            (cx0 as f64 + self.x_offset, -cy0 as f64),
            (cx1 as f64 + self.x_offset, -cy1 as f64),
            (x as f64 + self.x_offset, -y as f64),
        );
    }

    fn close(&mut self) {
        self.path.close_path();
    }
}

/// A specific font face.
/// The interface uses points as the unit for everything.
pub struct FontFace {
    name: String,
    font: FontCell,
    tweak: FontTweak,

    /// Variable font location (for weight axis, etc.)
    location: skrifa::instance::Location,
    glyph_info_cache: ahash::HashMap<char, GlyphInfo>,
    glyph_alloc_cache: ahash::HashMap<GlyphCacheKey, GlyphAllocation>,
}

impl FontFace {
    pub fn new(
        options: TextOptions,
        name: String,
        font_data: Blob,
        index: u32,
        tweak: FontTweak,
        preferred_weight: Option<u16>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let font = FontCell::try_new(font_data, |font_data| {
            let skrifa_font =
                skrifa::FontRef::from_index(AsRef::<[u8]>::as_ref(font_data.as_ref()), index)?;

            let charmap = skrifa_font.charmap();
            let glyphs = skrifa_font.outline_glyphs();

            // Note: We use default location here during initialization because
            // the actual weight will be applied via the stored location during rendering.
            // The metrics won't be significantly different at this unscaled size.
            let metrics = skrifa_font.metrics(
                skrifa::instance::Size::unscaled(),
                skrifa::instance::LocationRef::default(),
            );
            let glyph_metrics = skrifa_font.glyph_metrics(
                skrifa::instance::Size::unscaled(),
                skrifa::instance::LocationRef::default(),
            );

            let hinting_enabled = tweak.hinting_override.unwrap_or(options.font_hinting);
            let hinting_instance = hinting_enabled
                .then(|| {
                    // It doesn't really matter what we put here for options. Since the size is `unscaled()`, we will
                    // always reconfigure this hinting instance with the real options when rendering for the first time.
                    skrifa::outline::HintingInstance::new(
                        &glyphs,
                        skrifa::instance::Size::unscaled(),
                        skrifa::instance::LocationRef::default(),
                        skrifa::outline::Target::default(),
                    )
                    .ok()
                })
                .flatten();

            Ok::<DependentFontData<'_>, Box<dyn std::error::Error>>(DependentFontData {
                skrifa: skrifa_font,
                charmap,
                outline_glyphs: glyphs,
                metrics,
                glyph_metrics,
                hinting_instance,
            })
        })?;

        // Use preferred_weight if provided, otherwise try to read from the OS/2 table or fvar default
        let weight = preferred_weight.or_else(|| {
            // First try OS/2 table
            if let Some(w) = font
                .borrow_dependent()
                .skrifa
                .os2()
                .ok()
                .map(|os2| os2.us_weight_class())
            {
                return Some(w);
            }
            // If no OS/2 or preferred_weight, try to get default from variable font's fvar table
            font.borrow_dependent()
                .skrifa
                .axes()
                .iter()
                .find(|axis| axis.tag() == skrifa::raw::types::Tag::new(b"wght"))
                .map(|axis| axis.default_value() as u16)
        });

        // Create location for variable font with weight axis
        // If weight is provided (either from preferred_weight, OS/2, or fvar default), use it
        // Otherwise fall back to Location::default() which uses all axis defaults
        let location = if let Some(w) = weight {
            font.borrow_dependent()
                .skrifa
                .axes()
                .location([("wght", w as f32)])
        } else {
            skrifa::instance::Location::default()
        };

        Ok(Self {
            name,
            font,
            tweak,
            location,
            glyph_info_cache: Default::default(),
            glyph_alloc_cache: Default::default(),
        })
    }

    /// Code points that will always be replaced by the replacement character.
    ///
    /// See also [`invisible_char`].
    fn ignore_character(&self, chr: char) -> bool {
        use crate::text::FontDefinitions;

        if !FontDefinitions::builtin_font_names().contains(&self.name.as_str()) {
            return false;
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
        self.font
            .borrow_dependent()
            .charmap
            .mappings()
            .filter_map(|(chr, _)| char::from_u32(chr).filter(|c| !self.ignore_character(*c)))
    }

    /// `\n` will result in `None`
    pub(super) fn glyph_info(&mut self, c: char) -> Option<GlyphInfo> {
        if let Some(glyph_info) = self.glyph_info_cache.get(&c) {
            return Some(*glyph_info);
        }

        if self.ignore_character(c) {
            return None; // these will result in the replacement character when rendering
        }

        if c == '\t'
            && let Some(space) = self.glyph_info(' ')
        {
            let glyph_info = GlyphInfo {
                advance_width_unscaled: (crate::text::TAB_SIZE as f32
                    * space.advance_width_unscaled.0)
                    .into(),
                ..space
            };
            self.glyph_info_cache.insert(c, glyph_info);
            return Some(glyph_info);
        }

        if c == '\u{2009}' {
            // Thin space, often used as thousands deliminator: 1 234 567 890
            // https://www.compart.com/en/unicode/U+2009
            // https://en.wikipedia.org/wiki/Thin_space

            if let Some(space) = self.glyph_info(' ') {
                let em = self.font.borrow_dependent().metrics.units_per_em as f32;
                let advance_width = f32::min(em / 6.0, space.advance_width_unscaled.0 * 0.5); // TODO(emilk): make configurable
                let glyph_info = GlyphInfo {
                    advance_width_unscaled: advance_width.into(),
                    ..space
                };
                self.glyph_info_cache.insert(c, glyph_info);
                return Some(glyph_info);
            }
        }

        if invisible_char(c) {
            let glyph_info = GlyphInfo::INVISIBLE;
            self.glyph_info_cache.insert(c, glyph_info);
            return Some(glyph_info);
        }

        let font_data = self.font.borrow_dependent();

        // Add new character:
        let glyph_id = font_data
            .charmap
            .map(c)
            .filter(|id| *id != skrifa::GlyphId::NOTDEF)?;

        let glyph_info = GlyphInfo {
            id: Some(glyph_id),
            advance_width_unscaled: font_data
                .glyph_metrics
                .advance_width(glyph_id)
                .unwrap_or_default()
                .into(),
        };
        self.glyph_info_cache.insert(c, glyph_info);
        Some(glyph_info)
    }

    #[inline]
    pub(super) fn pair_kerning_pixels(
        &self,
        metrics: &ScaledMetrics,
        last_glyph_id: skrifa::GlyphId,
        glyph_id: skrifa::GlyphId,
    ) -> f32 {
        let skrifa_font = &self.font.borrow_dependent().skrifa;
        let Ok(kern) = skrifa_font.kern() else {
            return 0.0;
        };
        kern.subtables()
            .find_map(|st| match st.ok()?.kind().ok()? {
                SubtableKind::Format0(table_ref) => table_ref.kerning(last_glyph_id, glyph_id),
                SubtableKind::Format1(_) => None,
                SubtableKind::Format2(subtable2) => subtable2.kerning(last_glyph_id, glyph_id),
                SubtableKind::Format3(table_ref) => table_ref.kerning(last_glyph_id, glyph_id),
            })
            .unwrap_or_default() as f32
            * metrics.px_scale_factor
    }

    #[inline]
    pub fn pair_kerning(
        &self,
        metrics: &ScaledMetrics,
        last_glyph_id: skrifa::GlyphId,
        glyph_id: skrifa::GlyphId,
    ) -> f32 {
        self.pair_kerning_pixels(metrics, last_glyph_id, glyph_id) / metrics.pixels_per_point
    }

    #[inline(always)]
    pub fn scaled_metrics(&self, pixels_per_point: f32, font_size: f32) -> ScaledMetrics {
        let pt_scale_factor = self.font.px_scale_factor(font_size * self.tweak.scale);
        let font_data = self.font.borrow_dependent();
        let ascent = (font_data.metrics.ascent * pt_scale_factor).round_ui();
        let descent = (font_data.metrics.descent * pt_scale_factor).round_ui();
        let line_gap = (font_data.metrics.leading * pt_scale_factor).round_ui();

        let scale = font_size * self.tweak.scale * pixels_per_point;
        let px_scale_factor = self.font.px_scale_factor(scale);

        let y_offset_in_points = ((font_size * self.tweak.scale * self.tweak.y_offset_factor)
            + self.tweak.y_offset)
            .round_ui();

        ScaledMetrics {
            pixels_per_point,
            px_scale_factor,
            scale,
            y_offset_in_points,
            ascent,
            row_height: ascent - descent + line_gap,
        }
    }

    pub fn allocate_glyph(
        &mut self,
        atlas: &mut TextureAtlas,
        metrics: &ScaledMetrics,
        glyph_info: GlyphInfo,
        chr: char,
        h_pos: f32,
    ) -> (GlyphAllocation, i32) {
        let advance_width_px = glyph_info.advance_width_unscaled.0 * metrics.px_scale_factor;

        let Some(glyph_id) = glyph_info.id else {
            // Invisible.
            return (GlyphAllocation::default(), h_pos as i32);
        };

        // CJK scripts contain a lot of characters and could hog the glyph atlas if we stored 4 subpixel offsets per
        // glyph.
        let (h_pos_round, bin) = if is_cjk(chr) {
            (h_pos.round() as i32, SubpixelBin::Zero)
        } else {
            SubpixelBin::new(h_pos)
        };

        let entry = match self
            .glyph_alloc_cache
            .entry(GlyphCacheKey::new(glyph_id, metrics, bin))
        {
            std::collections::hash_map::Entry::Occupied(glyph_alloc) => {
                let mut glyph_alloc = *glyph_alloc.get();
                glyph_alloc.advance_width_px = advance_width_px; // Hack to get `\t` and thin space to work, since they use the same glyph id as ` ` (space).
                return (glyph_alloc, h_pos_round);
            }
            std::collections::hash_map::Entry::Vacant(entry) => entry,
        };

        let allocation = self
            .font
            .allocate_glyph_uncached(atlas, metrics, &glyph_info, bin, &self.location)
            .unwrap_or_default();

        entry.insert(allocation);
        (allocation, h_pos_round)
    }
}

// TODO(emilk): rename?
/// Wrapper over multiple [`FontFace`] (e.g. a primary + fallbacks for emojis)
pub struct Font<'a> {
    pub(super) fonts_by_id: &'a mut nohash_hasher::IntMap<FontFaceKey, FontFace>,
    pub(super) cached_family: &'a mut CachedFamily,
    pub(super) atlas: &'a mut TextureAtlas,
}

impl Font<'_> {
    pub fn preload_characters(&mut self, s: &str) {
        for c in s.chars() {
            self.glyph_info(c);
        }
    }

    /// All supported characters, and in which font they are available in.
    pub fn characters(&mut self) -> &BTreeMap<char, Vec<String>> {
        self.cached_family.characters.get_or_insert_with(|| {
            let mut characters: BTreeMap<char, Vec<String>> = Default::default();
            for font_id in &self.cached_family.fonts {
                let font = self.fonts_by_id.get(font_id).expect("Nonexistent font ID");
                for chr in font.characters() {
                    characters.entry(chr).or_default().push(font.name.clone());
                }
            }
            characters
        })
    }

    pub fn scaled_metrics(&self, pixels_per_point: f32, font_size: f32) -> ScaledMetrics {
        self.cached_family
            .fonts
            .first()
            .and_then(|key| self.fonts_by_id.get(key))
            .map(|font_face| font_face.scaled_metrics(pixels_per_point, font_size))
            .unwrap_or_default()
    }

    /// Width of this character in points.
    pub fn glyph_width(&mut self, c: char, font_size: f32) -> f32 {
        let (key, glyph_info) = self.glyph_info(c);
        if let Some(font) = &self.fonts_by_id.get(&key) {
            glyph_info.advance_width_unscaled.0 * font.font.px_scale_factor(font_size)
        } else {
            0.0
        }
    }

    /// Can we display this glyph?
    pub fn has_glyph(&mut self, c: char) -> bool {
        self.glyph_info(c) != self.cached_family.replacement_glyph // TODO(emilk): this is a false negative if the user asks about the replacement character itself 🤦‍♂️
    }

    /// Can we display all the glyphs in this text?
    pub fn has_glyphs(&mut self, s: &str) -> bool {
        s.chars().all(|c| self.has_glyph(c))
    }

    /// `\n` will (intentionally) show up as the replacement character.
    pub(crate) fn glyph_info(&mut self, c: char) -> (FontFaceKey, GlyphInfo) {
        if let Some(font_index_glyph_info) = self.cached_family.glyph_info_cache.get(&c) {
            return *font_index_glyph_info;
        }

        let font_index_glyph_info = self
            .cached_family
            .glyph_info_no_cache_or_fallback(c, self.fonts_by_id);
        let font_index_glyph_info =
            font_index_glyph_info.unwrap_or(self.cached_family.replacement_glyph);
        self.cached_family
            .glyph_info_cache
            .insert(c, font_index_glyph_info);
        font_index_glyph_info
    }
}

/// Metrics for a font at a specific screen-space scale.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ScaledMetrics {
    /// The DPI part of the screen-space scale.
    pub pixels_per_point: f32,

    /// Scale factor, relative to the font's units per em (so, probably much less than 1).
    ///
    /// Translates "unscaled" units to physical (screen) pixels.
    pub px_scale_factor: f32,

    /// Absolute scale in screen pixels, for skrifa.
    pub scale: f32,

    /// Vertical offset, in UI points (not screen-space).
    pub y_offset_in_points: f32,

    /// This is the distance from the top to the baseline.
    ///
    /// Unit: points.
    pub ascent: f32,

    /// Height of one row of text in points.
    ///
    /// Returns a value rounded to [`emath::GUI_ROUNDING`].
    pub row_height: f32,
}

/// Code points that will always be invisible (zero width).
///
/// See also [`FontFace::ignore_character`].
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

#[inline]
pub(super) fn is_cjk_ideograph(c: char) -> bool {
    ('\u{4E00}' <= c && c <= '\u{9FFF}')
        || ('\u{3400}' <= c && c <= '\u{4DBF}')
        || ('\u{2B740}' <= c && c <= '\u{2B81F}')
}

#[inline]
pub(super) fn is_kana(c: char) -> bool {
    ('\u{3040}' <= c && c <= '\u{309F}') // Hiragana block
        || ('\u{30A0}' <= c && c <= '\u{30FF}') // Katakana block
}

#[inline]
pub(super) fn is_cjk(c: char) -> bool {
    // TODO(bigfarts): Add support for Korean Hangul.
    is_cjk_ideograph(c) || is_kana(c)
}

#[inline]
pub(super) fn is_cjk_break_allowed(c: char) -> bool {
    // See: https://en.wikipedia.org/wiki/Line_breaking_rules_in_East_Asian_languages#Characters_not_permitted_on_the_start_of_a_line.
    !")]｝〕〉》」』】〙〗〟'\"｠»ヽヾーァィゥェォッャュョヮヵヶぁぃぅぇぉっゃゅょゎゕゖㇰㇱㇲㇳㇴㇵㇶㇷㇸㇹㇺㇻㇼㇽㇾㇿ々〻‐゠–〜?!‼⁇⁈⁉・、:;,。.".contains(c)
}
