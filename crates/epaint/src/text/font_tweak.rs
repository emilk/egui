use crate::text::VariationCoords;

/// Extra scale and vertical tweak to apply to all text of a certain font.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontTweak {
    /// Scale the font's glyphs by this much.
    /// this is only a visual effect and does not affect the text layout.
    ///
    /// Default: `1.0` (no scaling).
    pub scale: f32,

    /// Shift font's glyphs downwards by this fraction of the font size (in points).
    /// this is only a visual effect and does not affect the text layout.
    ///
    /// Affects larger font sizes more.
    ///
    /// A positive value shifts the text downwards.
    /// A negative value shifts it upwards.
    ///
    /// Example value: `-0.2`.
    pub y_offset_factor: f32,

    /// Shift font's glyphs downwards by this amount of logical points.
    /// this is only a visual effect and does not affect the text layout.
    ///
    /// Affects all font sizes equally.
    ///
    /// Example value: `2.0`.
    pub y_offset: f32,

    /// Override the global font hinting setting for this specific font.
    ///
    /// `None` means use the global setting in [`TextOptions::font_hinting`](crate::text::TextOptions::font_hinting).
    pub hinting: Option<bool>,

    /// How to grid-fit the glyph outlines when hinting is enabled.
    ///
    /// Has no effect when hinting is disabled (see [`Self::hinting`]).
    pub hinting_target: HintingTarget,

    /// Override the global sub-pixel binning setting for this specific font.
    ///
    /// `None` means use the global setting in [`TextOptions::subpixel_binning`](crate::text::TextOptions::subpixel_binning).
    pub subpixel_binning: Option<bool>,

    /// Override the font's default variation coordinates for its axes ("wght", etc.).
    pub coords: VariationCoords,

    /// Width of a thin space (`\u{2009}`) and narrow no-break space (`\u{202F}`),
    /// as a fraction of the normal space width.
    ///
    /// Thin space is often used as a thousands separator: `1 234 567`.
    ///
    /// Default: `0.5` (half a normal space).
    pub thin_space_width: f32,

    /// Width of a tab character (`\t`), measured in number of space widths.
    ///
    /// Default: `4.0`.
    pub tab_size: f32,
}

impl Default for FontTweak {
    fn default() -> Self {
        Self {
            scale: 1.0,
            y_offset_factor: 0.0,
            y_offset: 0.0,
            hinting: None,
            hinting_target: HintingTarget::default(),
            subpixel_binning: None,
            coords: VariationCoords::default(),
            thin_space_width: 0.5,
            tab_size: 4.0,
        }
    }
}

// ----------------------------------------------------------------------------

/// How to *hint* glyph outlines, i.e. how aggressively to nudge them onto the
/// pixel grid before rasterizing. Mirrors [`skrifa::outline::Target`].
///
/// Hinting trades shape fidelity for sharpness: snapping stems to whole pixels
/// makes text crisp at small sizes / low dpi, at the cost of slightly distorting
/// the designer's outlines. At high dpi it matters little.
///
/// This only has an effect if the font is actually hinted — either it ships
/// TrueType instructions, or it was auto-hinted (see [`FontTweak::hinting`]).
///
/// Used by [`FontTweak::hinting_target`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum HintingTarget {
    /// Strongest hinting, designed for aliased 1-bit (black & white) rendering.
    ///
    /// Snaps stems hard to the pixel grid for maximum sharpness. egui always
    /// renders anti-aliased, so in practice this looks much like [`Self::Smooth`]
    /// here; it mostly exists for completeness. Maps to `skrifa`'s `Target::Mono`.
    Mono,

    /// Hinting tuned for anti-aliased rendering. This is what you normally want,
    /// and what egui uses by default. Maps to `skrifa`'s `Target::Smooth`.
    Smooth(SmoothHinting),
}

impl Default for HintingTarget {
    fn default() -> Self {
        Self::Smooth(SmoothHinting::default())
    }
}

impl From<HintingTarget> for skrifa::outline::Target {
    fn from(hinting_target: HintingTarget) -> Self {
        use skrifa::outline::SmoothMode;
        match hinting_target {
            HintingTarget::Mono => Self::Mono,
            HintingTarget::Smooth(SmoothHinting {
                light,
                symmetric_rendering,
                preserve_linear_metrics,
            }) => Self::Smooth {
                mode: if light {
                    SmoothMode::Light
                } else {
                    SmoothMode::Normal
                },
                symmetric_rendering,
                preserve_linear_metrics,
            },
        }
    }
}

/// Tuning for [`HintingTarget::Smooth`], mirroring `skrifa`'s `Target::Smooth`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SmoothHinting {
    /// Hint only lightly: snap stems vertically but leave horizontal shapes
    /// alone (`FreeType`'s "light" mode). Preserves the font's proportions
    /// better, at the cost of a little horizontal sharpness.
    ///
    /// `false` uses the "normal" mode, which also fits horizontally.
    /// Maps to `SmoothMode::Light` (`true`) vs `SmoothMode::Normal` (`false`).
    pub light: bool,

    /// Render a glyph the same way regardless of its sub-pixel position.
    ///
    /// `true` makes glyphs position-independent (better for caching and
    /// animation), but a font's instructions may then widen stems and look
    /// slightly blurrier under an analytic rasterizer like egui's.
    ///
    /// **Only affects fonts hinted via the TrueType interpreter** (i.e. fonts
    /// that ship their own instructions). It has no effect on the auto-hinter.
    /// Mirrors `Target::Smooth { symmetric_rendering }`.
    pub symmetric_rendering: bool,

    /// Keep advance widths independent of hinting (don't grid-fit horizontally
    /// in a way that changes spacing).
    ///
    /// `true` keeps inter-glyph spacing identical to the unhinted font, so
    /// layout never depends on hinting — but it also prevents horizontal
    /// grid-fitting, leaving vertical stems softer on low-dpi screens.
    ///
    /// `false` lets the (auto)hinter snap horizontally for crisper stems.
    /// egui positions glyphs from the shaper's advances, not the hinted
    /// outline, so this mainly affects sharpness here, not layout.
    /// Mirrors `Target::Smooth { preserve_linear_metrics }`.
    pub preserve_linear_metrics: bool,
}

impl Default for SmoothHinting {
    fn default() -> Self {
        // Matches the behavior egui had before the hinting target was configurable.
        // Note this means horizontal grid-fitting is opt-in (see `preserve_linear_metrics`).
        Self {
            light: false,
            symmetric_rendering: true,
            preserve_linear_metrics: true,
        }
    }
}
