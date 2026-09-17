use std::sync::Arc;

use emath::Rangef;

use crate::text::{FontTweak, Tag};

/// Shared, immutable bytes of a font file.
///
/// Cheap to clone. Can wrap static bytes, a `Vec<u8>`, or a memory-mapped file.
pub type Blob = Arc<dyn AsRef<[u8]> + Send + Sync>;

/// A `.ttf` or `.otf` file and a font face index.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontData {
    /// The content of a `.ttf` or `.otf` file.
    ///
    /// Shared, so that cloning a [`FontData`] does not copy the file.
    #[cfg_attr(feature = "serde", serde(with = "blob_serde"))]
    pub font: Blob,

    /// Which font face in the file to use.
    /// When in doubt, use `0`.
    pub index: u32,

    /// Extra scale and vertical tweak to apply to all text of this font.
    pub tweak: FontTweak,
}

impl FontData {
    pub fn from_static(font: &'static [u8]) -> Self {
        Self::from_blob(Arc::new(font), 0)
    }

    pub fn from_owned(font: Vec<u8>) -> Self {
        Self::from_blob(Arc::new(font), 0)
    }

    /// Use already shared bytes, e.g. a memory-mapped system font file, without copying them.
    pub fn from_blob(font: Blob, index: u32) -> Self {
        Self {
            font,
            index,
            tweak: Default::default(),
        }
    }

    /// The content of the font file.
    #[inline]
    pub fn bytes(&self) -> &[u8] {
        (*self.font).as_ref()
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

        let Ok(font) = skrifa::FontRef::from_index(self.bytes(), self.index) else {
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
        self.bytes()
    }
}

impl PartialEq for FontData {
    fn eq(&self, other: &Self) -> bool {
        let Self { font, index, tweak } = self;
        *index == other.index
            && *tweak == other.tweak
            && (Arc::ptr_eq(font, &other.font) || self.bytes() == other.bytes())
    }
}

impl core::fmt::Debug for FontData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let Self { font, index, tweak } = self;
        f.debug_struct("FontData")
            .field("font", &format_args!("{} bytes", (**font).as_ref().len()))
            .field("index", index)
            .field("tweak", tweak)
            .finish()
    }
}

#[cfg(feature = "serde")]
mod blob_serde {
    use super::Blob;

    pub fn serialize<S: serde::Serializer>(blob: &Blob, serializer: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize((**blob).as_ref(), serializer)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Blob, D::Error> {
        let bytes: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
        Ok(std::sync::Arc::new(bytes))
    }
}
