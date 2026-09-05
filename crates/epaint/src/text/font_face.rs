#![expect(clippy::mem_forget)]

use ahash::HashMap;
use ecolor::Color32;
use emath::{GuiRounding as _, OrderedFloat, vec2};
use self_cell::self_cell;
use skrifa::{GlyphId, MetadataProvider as _};
use vello_cpu::{color, kurbo};

use crate::{
    ColorImage, TextOptions,
    text::{
        FontTweak, VariationCoords,
        font_data::Blob,
        glyph_atlas::{GlyphBitmap, SubpixelBin},
        styled_metrics::{LocationHash, StyledMetrics},
        unicode::invisible_char,
    },
};

/// A glyph id and its advance, as resolved from a `char` in one [`FontFace`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphInfo {
    /// Doesn't need to be unique.
    ///
    /// Is `None` for a special "invisible" glyph.
    pub(crate) id: Option<GlyphId>,

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

/// Result of resolving a `char` to a [`GlyphId`] within a single [`FontFace`].
///
/// Location-independent: only depends on the font's charmap and `FontTweak`,
/// not on variable-font variation coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum GlyphIdResolution {
    /// A real, visible glyph.
    Glyph(GlyphId),

    /// A valid char, but rendered as zero-width (control chars, joiners, …).
    Invisible,
}

// ----------------------------------------------------------------------------

/// The parsed `skrifa` views into a font file, borrowing from the file's [`Blob`].
struct DependentFontData<'a> {
    skrifa: skrifa::FontRef<'a>,
    charmap: skrifa::charmap::Charmap<'a>,
    outline_glyphs: skrifa::outline::OutlineGlyphCollection<'a>,
    metrics: skrifa::metrics::Metrics,
    hinting_instance: Option<skrifa::outline::HintingInstance>,
}

self_cell! {
    /// A font file together with the parsed [`DependentFontData`] that borrows from it.
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

    /// Render the outline of a glyph to a coverage bitmap.
    ///
    /// Returns `None` if the glyph has no outline (e.g. a space).
    fn rasterize_outline(
        &mut self,
        metrics: &StyledMetrics,
        glyph_id: GlyphId,
        bin: SubpixelBin,
        hinting_target: skrifa::outline::Target,
    ) -> Option<GlyphBitmap> {
        debug_assert!(
            glyph_id != skrifa::GlyphId::NOTDEF,
            "Can't rasterize glyph id 0"
        );

        let location: skrifa::instance::LocationRef<'_> = (&metrics.location).into();

        let mut path = kurbo::BezPath::new();
        let mut pen = VelloPen {
            path: &mut path,
            x_offset: bin.as_float() as f64,
        };

        self.with_dependent_mut(|_, font_data| {
            let outline = font_data.outline_glyphs.get(glyph_id)?;

            if let Some(hinting_instance) = &mut font_data.hinting_instance {
                let size = skrifa::instance::Size::new(metrics.scale);
                if hinting_instance.size() != size
                    || hinting_instance.location().coords() != location.coords()
                    || hinting_instance.target() != hinting_target
                {
                    hinting_instance
                        .reconfigure(&font_data.outline_glyphs, size, location, hinting_target)
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
        if width == 0 || height == 0 {
            return None;
        }

        let mut ctx = vello_cpu::RenderContext::new(width, height);
        ctx.set_transform(kurbo::Affine::translate((-bounds.x0, -bounds.y0)));
        ctx.set_paint(color::OpaqueColor::<color::Srgb>::WHITE);
        ctx.fill_path(&path);
        let mut dest = vello_cpu::Pixmap::new(width, height);
        let mut resources = vello_cpu::Resources::new();
        ctx.render(&mut dest, &mut resources);

        let pixels = dest
            .data_as_u8_slice()
            .chunks_exact(4)
            .map(|px| Color32::from_rgba_premultiplied(px[0], px[1], px[2], px[3]))
            .collect();
        let image = ColorImage::new([width as usize, height as usize], pixels);

        Some(GlyphBitmap {
            image,
            offset_px: vec2(bounds.x0 as f32, bounds.y0 as f32),
        })
    }
}

/// Collects a `skrifa` glyph outline into a `kurbo` path, flipping Y to point down.
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
    subpixel_binning: bool,

    /// Cached `harfrust` shaper data (parsed GSUB/GPOS tables).
    /// `ShaperData` is `Copy` — lives outside the `self_cell`.
    shaper_data: harfrust::ShaperData,

    /// Location-independent: `char → GlyphId | Invisible`.
    ///
    /// Only depends on the font's charmap + `FontTweak`. A miss means the char
    /// is not in this face's repertoire and the fallback chain should be tried.
    glyph_id_cache: HashMap<char, GlyphIdResolution>,

    /// Location-dependent: `(char, LocationHash) → unscaled advance width`.
    ///
    /// Variable fonts can vary advance widths per axis (HVAR table), so this
    /// must be re-keyed per resolved [`skrifa::instance::Location`].
    advance_width_cache: HashMap<(char, LocationHash), OrderedFloat<f32>>,
}

impl FontFace {
    pub fn new(
        options: TextOptions,
        name: String,
        font_data: Blob,
        index: u32,
        tweak: FontTweak,
    ) -> Result<Self, Box<dyn core::error::Error>> {
        let font = FontCell::try_new(font_data, |font_data| {
            let skrifa_font =
                skrifa::FontRef::from_index(AsRef::<[u8]>::as_ref(font_data.as_ref()), index)?;

            let charmap = skrifa_font.charmap();
            let glyphs = skrifa_font.outline_glyphs();

            // Note: We use default location here during initialization because
            // the actual weight will be applied via the stored location during rendering.
            // The metrics won't be significantly different at this unscaled size.
            // TODO(emilk): heed location for vertical metrics too (HVAR/MVAR).
            let metrics = skrifa_font.metrics(
                skrifa::instance::Size::unscaled(),
                skrifa::instance::LocationRef::default(),
            );

            let hinting_enabled = tweak.hinting.unwrap_or(options.font_hinting);
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

            Ok::<DependentFontData<'_>, Box<dyn core::error::Error>>(DependentFontData {
                skrifa: skrifa_font,
                charmap,
                outline_glyphs: glyphs,
                metrics,
                hinting_instance,
            })
        })?;

        let shaper_data = harfrust::ShaperData::new(&font.borrow_dependent().skrifa);

        let subpixel_binning = tweak.subpixel_binning.unwrap_or(options.subpixel_binning);

        Ok(Self {
            name,
            font,
            tweak,
            subpixel_binning,
            shaper_data,
            glyph_id_cache: Default::default(),
            advance_width_cache: Default::default(),
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

    /// The name the font was installed under, i.e. its key in `FontDefinitions::font_data`.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Scale factor from the font's unscaled units to `scale` (points or pixels).
    #[inline]
    pub(crate) fn px_scale_factor(&self, scale: f32) -> f32 {
        self.font.px_scale_factor(scale)
    }

    /// An un-ordered iterator over all supported characters.
    pub fn characters(&self) -> impl Iterator<Item = char> + '_ {
        self.font
            .borrow_dependent()
            .charmap
            .mappings()
            .filter_map(|(chr, _)| char::from_u32(chr).filter(|c| !self.ignore_character(*c)))
    }

    /// Resolve a `char` to a [`GlyphId`] within this face.
    ///
    /// Location-independent. Returns `None` when this face cannot represent
    /// the char (the caller should try the fallback chain).
    ///
    /// `\t` and thin spaces share `' '`s glyph id (they just have a custom advance).
    pub(super) fn glyph_id_resolution(&mut self, c: char) -> Option<GlyphIdResolution> {
        if let Some(resolution) = self.glyph_id_cache.get(&c) {
            return Some(*resolution);
        }

        if self.ignore_character(c) {
            return None; // these will result in the replacement character when rendering
        }

        let resolution = if c == '\t' || c == '\u{2009}' || c == '\u{202F}' {
            // `\t` and thin spaces are rendered as a space glyph with a custom advance.
            self.glyph_id_resolution(' ')?
        } else if invisible_char(c) {
            GlyphIdResolution::Invisible
        } else {
            let glyph_id = self
                .font
                .borrow_dependent()
                .charmap
                .map(c)
                .filter(|id| *id != GlyphId::NOTDEF)?;
            GlyphIdResolution::Glyph(glyph_id)
        };

        self.glyph_id_cache.insert(c, resolution);
        Some(resolution)
    }

    /// Unscaled advance width for `c` at the given variation location.
    ///
    /// Location-dependent (variable fonts can vary advances via HVAR).
    /// Cached per `(char, LocationHash)`.
    fn advance_width_unscaled(&mut self, c: char, metrics: &StyledMetrics) -> f32 {
        let cache_key = (c, metrics.location_hash);
        if let Some(advance) = self.advance_width_cache.get(&cache_key) {
            return advance.0;
        }

        let advance = match c {
            '\t' => self.tweak.tab_size * self.advance_width_unscaled(' ', metrics),
            '\u{2009}' | '\u{202F}' => {
                // Thin space (U+2009) and narrow no-break space (U+202F),
                // often used as thousands separator.
                self.tweak.thin_space_width * self.advance_width_unscaled(' ', metrics)
            }
            _ => {
                let Some(GlyphIdResolution::Glyph(glyph_id)) = self.glyph_id_resolution(c) else {
                    return 0.0;
                };
                let font_data = self.font.borrow_dependent();
                let glyph_metrics = font_data
                    .skrifa
                    .glyph_metrics(skrifa::instance::Size::unscaled(), &metrics.location);
                glyph_metrics.advance_width(glyph_id).unwrap_or_default()
            }
        };

        self.advance_width_cache.insert(cache_key, advance.into());
        advance
    }

    /// `\n` will result in `None`.
    ///
    /// Caller must pass [`StyledMetrics`] resolved against *this* face so that
    /// variable-font advance widths are looked up at the correct location.
    pub(super) fn glyph_info(&mut self, c: char, metrics: &StyledMetrics) -> Option<GlyphInfo> {
        let resolution = self.glyph_id_resolution(c)?;
        let glyph_info = match resolution {
            GlyphIdResolution::Invisible => GlyphInfo::INVISIBLE,
            GlyphIdResolution::Glyph(glyph_id) => GlyphInfo {
                id: Some(glyph_id),
                advance_width_unscaled: self.advance_width_unscaled(c, metrics).into(),
            },
        };
        Some(glyph_info)
    }

    #[inline(always)]
    pub fn styled_metrics(
        &self,
        pixels_per_point: f32,
        font_size: f32,
        coords: &VariationCoords,
    ) -> StyledMetrics {
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

        let axes = font_data.skrifa.axes();
        // Override the default coordinates with ones specified via FontTweak, then the ones specified directly via the
        // argument (probably from TextFormat).
        let settings = core::iter::chain(self.tweak.coords.as_ref(), coords.as_ref());
        let location = axes.location(settings);
        let location_hash = LocationHash::new(&location);

        StyledMetrics {
            pixels_per_point,
            px_scale_factor,
            scale,
            y_offset_in_points,
            ascent,
            row_height: ascent - descent + line_gap,
            location,
            location_hash,
        }
    }

    pub(crate) fn skrifa_font_ref(&self) -> &skrifa::FontRef<'_> {
        &self.font.borrow_dependent().skrifa
    }

    pub(crate) fn tweak(&self) -> &FontTweak {
        &self.tweak
    }

    pub(crate) fn shaper_data(&self) -> &harfrust::ShaperData {
        &self.shaper_data
    }

    /// Whether to render this face at up to four sub-pixel offsets.
    #[inline]
    pub(crate) fn subpixel_binning(&self) -> bool {
        self.subpixel_binning
    }

    /// Render the outline of a glyph to a coverage bitmap.
    ///
    /// Not cached: see [`GlyphAtlas::allocate_outline`].
    pub(crate) fn rasterize_outline(
        &mut self,
        metrics: &StyledMetrics,
        glyph_id: GlyphId,
        bin: SubpixelBin,
    ) -> Option<GlyphBitmap> {
        let hinting_target = self.tweak.hinting_target.into();
        self.font
            .rasterize_outline(metrics, glyph_id, bin, hinting_target)
    }
}

/// Positioning info for a single glyph, ready for atlas allocation.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShapedGlyph {
    pub glyph_id: GlyphId,

    /// Horizontal position of the glyph origin, in physical pixels.
    pub h_pos: f32,

    /// CJK glyphs skip subpixel positioning to save atlas space.
    pub is_cjk: bool,
}
