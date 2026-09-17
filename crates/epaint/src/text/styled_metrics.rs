/// A precomputed hash of a [`skrifa::instance::Location`].
///
/// Used as a cache key so that we don't have to re-hash the coordinate list
/// for every glyph lookup. Compute once per text run and reuse for every glyph
/// in the run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct LocationHash(u64);

impl nohash_hasher::IsEnabled for LocationHash {}

impl LocationHash {
    #[inline]
    pub fn new(location: &skrifa::instance::Location) -> Self {
        if location.coords().is_empty() {
            // Fast path for the (common) default-coords case.
            Self(0)
        } else {
            Self(crate::util::hash(location))
        }
    }
}

// ----------------------------------------------------------------------------

/// Metrics for a font at a specific screen-space scale.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct StyledMetrics {
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

    /// Resolved variation coordinates.
    pub location: skrifa::instance::Location,

    /// Precomputed hash of [`Self::location`].
    ///
    /// Hashed once per run of text so per-glyph cache lookups don't have to
    /// re-hash the full coordinate list.
    pub(crate) location_hash: LocationHash,
}
