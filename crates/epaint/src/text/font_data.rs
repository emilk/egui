use std::{borrow::Cow, sync::Arc};

use emath::Rangef;

use crate::text::{FontTweak, Tag};

/// A `.ttf` or `.otf` file and a font face index.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontData {
    /// The content of a `.ttf` or `.otf` file.
    pub font: Cow<'static, [u8]>,

    /// Which font face in the file to use.
    /// When in doubt, use `0`.
    pub index: u32,

    /// Extra scale and vertical tweak to apply to all text of this font.
    pub tweak: FontTweak,
}

impl FontData {
    pub fn from_static(font: &'static [u8]) -> Self {
        Self {
            font: Cow::Borrowed(font),
            index: 0,
            tweak: Default::default(),
        }
    }

    pub fn from_owned(font: Vec<u8>) -> Self {
        Self {
            font: Cow::Owned(font),
            index: 0,
            tweak: Default::default(),
        }
    }

    pub fn tweak(self, tweak: FontTweak) -> Self {
        Self { tweak, ..self }
    }

    /// The variation axes of this font, e.g. `wght` (weight) and `wdth` (width).
    ///
    /// Use this to discover which axes a variable font supports, and their valid
    /// ranges, so a UI can offer the right knobs instead of making the user guess
    /// tags and values for [`FontTweak::coords`].
    ///
    /// Returns an empty list for non-variable (static) fonts, or if the font data
    /// fails to parse.
    pub fn variation_axes(&self) -> Vec<FontVariationAxis> {
        use skrifa::MetadataProvider as _;

        let Ok(font) = skrifa::FontRef::from_index(self.font.as_ref(), self.index) else {
            return Vec::new();
        };

        font.axes()
            .iter()
            .map(|axis| FontVariationAxis {
                tag: axis.tag(),
                name: font
                    .localized_strings(axis.name_id())
                    .english_or_first()
                    .map(|name| name.chars().collect()),
                range: Rangef::new(axis.min_value(), axis.max_value()),
                default: axis.default_value(),
                hidden: axis.is_hidden(),
            })
            .collect()
    }
}

/// A single variation axis of a variable font, e.g. weight (`wght`) or width (`wdth`).
///
/// Obtained via [`FontData::variation_axes`].
#[derive(Clone, Debug, PartialEq)]
pub struct FontVariationAxis {
    /// The axis tag, e.g. `wght` or `wdth`.
    pub tag: Tag,

    /// Human-readable axis name, if the font provides one (e.g. "Weight").
    pub name: Option<String>,

    /// Valid range of values for this axis, `min..=max`.
    pub range: Rangef,

    /// The value used when the axis is not overridden.
    pub default: f32,

    /// Whether the font recommends hiding this axis from user interfaces.
    pub hidden: bool,
}

impl AsRef<[u8]> for FontData {
    fn as_ref(&self) -> &[u8] {
        self.font.as_ref()
    }
}

// ----------------------------------------------------------------------------

pub type Blob = Arc<dyn AsRef<[u8]> + Send + Sync>;

pub(super) fn blob_from_font_data(data: &FontData) -> Blob {
    match data.clone().font {
        Cow::Borrowed(bytes) => Arc::new(bytes) as Blob,
        Cow::Owned(bytes) => Arc::new(bytes) as Blob,
    }
}
