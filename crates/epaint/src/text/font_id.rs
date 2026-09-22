use std::sync::Arc;

/// How to select a sized font.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontId {
    /// Height in points.
    pub size: f32,

    /// What font family to use.
    pub family: FontFamily,
    // TODO(emilk): weight (bold), italics, …
}

impl Default for FontId {
    #[inline]
    fn default() -> Self {
        Self {
            size: 14.0,
            family: FontFamily::Proportional,
        }
    }
}

impl FontId {
    #[inline]
    pub const fn new(size: f32, family: FontFamily) -> Self {
        Self { size, family }
    }

    #[inline]
    pub const fn proportional(size: f32) -> Self {
        Self::new(size, FontFamily::Proportional)
    }

    #[inline]
    pub const fn monospace(size: f32) -> Self {
        Self::new(size, FontFamily::Monospace)
    }
}

impl core::hash::Hash for FontId {
    #[inline(always)]
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let Self { size, family } = self;
        emath::OrderedFloat(*size).hash(state);
        family.hash(state);
    }
}

// ----------------------------------------------------------------------------

/// Font of unknown size.
///
/// Which style of font: [`Monospace`][`FontFamily::Monospace`], [`Proportional`][`FontFamily::Proportional`],
/// or by user-chosen name.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FontFamily {
    /// A font where some characters are wider than other (e.g. 'w' is wider than 'i').
    ///
    /// Proportional fonts are easier to read and should be the preferred choice in most situations.
    #[default]
    Proportional,

    /// A font where each character is the same width (`w` is the same width as `i`).
    ///
    /// Useful for code snippets, or when you need to align numbers or text.
    Monospace,

    /// One of the names in [`FontDefinitions::families`](crate::text::FontDefinitions::families).
    ///
    /// ```
    /// # use epaint::FontFamily;
    /// // User-chosen names:
    /// FontFamily::Name("arial".into());
    /// FontFamily::Name("serif".into());
    /// ```
    Name(Arc<str>),
}

impl core::fmt::Display for FontFamily {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Monospace => "Monospace".fmt(f),
            Self::Proportional => "Proportional".fmt(f),
            Self::Name(name) => (*name).fmt(f),
        }
    }
}
