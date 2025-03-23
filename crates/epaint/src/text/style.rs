//! Font style definitions. These mostly mirror the ones in Parley, but allow us to not expose Parley types publicly, as
//! well as tweak them to fit our needs.

use std::{borrow::Cow, hash::Hash, str::FromStr, sync::Arc};

use ecolor::Color32;
use emath::{Align, OrderedFloat};
use named_variants::NamedFontVariants;

use crate::Stroke;

// TODO(valadaptive): Cow<'static, str> or Arc<str>?

/// A generic font family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum GenericFamily {
    // Parley exposes a lot more settings here, but not all of them behave well cross-platform. Only expose a subset here.
    /// The default user interface font.
    #[default]
    SystemUi,
    Serif,
    SansSerif,
    Monospace,
    Cursive,
    Emoji,
}

impl GenericFamily {
    pub(crate) fn as_parley(&self) -> parley::GenericFamily {
        match self {
            // TODO(valadaptive): SystemUi is not necessarily well-behaved (e.g. on my Linux system, it causes Arabic
            // text to disappear whereas SansSerif does not). There should be a more complex mapping of these generic
            // families to Parley's at some level.
            Self::SystemUi => parley::GenericFamily::SystemUi,
            Self::Serif => parley::GenericFamily::Serif,
            Self::SansSerif => parley::GenericFamily::SansSerif,
            Self::Monospace => parley::GenericFamily::Monospace,
            Self::Cursive => parley::GenericFamily::Cursive,
            Self::Emoji => parley::GenericFamily::Emoji,
        }
    }

    pub const ALL: [Self; 6] = [
        Self::SystemUi,
        Self::Serif,
        Self::SansSerif,
        Self::Monospace,
        Self::Cursive,
        Self::Emoji,
    ];
}

impl std::fmt::Display for GenericFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

/// A single font family, either a specific named font or a generic family.
///
/// For styling purposes, or if you're exposing an API that lets its users choose a font, you should consider using the
/// more flexible [`FontStack`] API to specify a set of multiple fonts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FontFamily {
    /// One of the names in [`super::FontDefinitions::families`].
    ///
    /// ```
    /// # use epaint::text::style::FontFamily;
    /// // User-chosen names:
    /// FontFamily::Named("arial".into());
    /// FontFamily::Named("serif".into());
    /// ```
    Named(Cow<'static, str>),
    Generic(GenericFamily),
}

impl FontFamily {
    pub(crate) fn as_parley(&self) -> parley::FontFamily<'static> {
        match self {
            Self::Named(cow) => parley::FontFamily::Named(cow.clone()),
            Self::Generic(generic_family) => {
                parley::FontFamily::Generic(generic_family.as_parley())
            }
        }
    }

    pub fn named(name: impl Into<Cow<'static, str>>) -> Self {
        Self::Named(name.into())
    }

    pub fn generic(generic_family: GenericFamily) -> Self {
        Self::Generic(generic_family)
    }
}

impl Default for FontFamily {
    fn default() -> Self {
        Self::Generic(Default::default())
    }
}

impl From<GenericFamily> for FontFamily {
    fn from(value: GenericFamily) -> Self {
        Self::Generic(value)
    }
}

impl std::fmt::Display for FontFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Named(name) => (*name).fmt(f),
            Self::Generic(name) => std::fmt::Debug::fmt(name, f),
        }
    }
}

/// A stack of font families, in order of preference. Fonts lower down the stack will be used if the fonts higher up the
/// stack do not contain the necessary glyphs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FontStack {
    Single(FontFamily),
    // TODO(valadaptive): make this an Arc<Vec<FontFamily>> for better performance?
    // Especially if we're taking away the ability to register custom font stacks under family names
    List(Cow<'static, [FontFamily]>),
}

impl FontStack {
    pub(crate) fn as_parley(&self) -> parley::FontStack<'static> {
        match self {
            Self::Single(family) => parley::FontStack::Single(family.as_parley()),
            Self::List(families) => {
                parley::FontStack::List(families.iter().map(|f| f.as_parley()).collect())
            }
        }
    }

    pub fn first_family(&self) -> &FontFamily {
        match self {
            Self::Single(family) => family,
            Self::List(families) => &families[0],
        }
    }
}

impl Default for FontStack {
    fn default() -> Self {
        Self::Single(Default::default())
    }
}

impl From<FontFamily> for FontStack {
    fn from(value: FontFamily) -> Self {
        Self::Single(value)
    }
}

impl From<GenericFamily> for FontStack {
    fn from(value: GenericFamily) -> Self {
        Self::Single(value.into())
    }
}

impl FromIterator<FontFamily> for FontStack {
    fn from_iter<T: IntoIterator<Item = FontFamily>>(iter: T) -> Self {
        Self::List(Cow::Owned(iter.into_iter().collect()))
    }
}

/// Weight of a font, typically on a scale of 1.0 to 1000.0.
///
/// The default value is [`Self::NORMAL`] or 400.0.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontWeight(pub f32);

impl FontWeight {
    /// Weight value of 100.
    pub const THIN: Self = Self(100.0);
    /// Weight value of 200.
    pub const EXTRA_LIGHT: Self = Self(200.0);
    /// Weight value of 300.
    pub const LIGHT: Self = Self(300.0);
    /// Weight value of 350.
    pub const SEMI_LIGHT: Self = Self(350.0);
    /// Weight value of 400. This is the default value.
    pub const NORMAL: Self = Self(400.0);
    /// Weight value of 500.
    pub const MEDIUM: Self = Self(500.0);
    /// Weight value of 600.
    pub const SEMI_BOLD: Self = Self(600.0);
    /// Weight value of 700.
    pub const BOLD: Self = Self(700.0);
    /// Weight value of 800.
    pub const EXTRA_BOLD: Self = Self(800.0);
    /// Weight value of 900.
    pub const BLACK: Self = Self(900.0);
    /// Weight value of 950.
    pub const EXTRA_BLACK: Self = Self(950.0);

    pub(crate) fn as_parley(&self) -> parley::FontWeight {
        parley::FontWeight::new(self.0)
    }
}

impl Hash for FontWeight {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        OrderedFloat(self.0).hash(state);
    }
}

impl std::cmp::PartialEq for FontWeight {
    fn eq(&self, other: &Self) -> bool {
        OrderedFloat(self.0) == OrderedFloat(other.0)
    }
}
impl std::cmp::Eq for FontWeight {}

impl std::cmp::PartialOrd for FontWeight {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl std::cmp::Ord for FontWeight {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        OrderedFloat(self.0).cmp(&OrderedFloat(other.0))
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Visual width / "condensedness" of a font, relative to its normal aspect ratio (1.0).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontWidth(pub f32);
impl FontWidth {
    /// Width that is 50% of normal.
    pub const ULTRA_CONDENSED: Self = Self(0.5);
    /// Width that is 62.5% of normal.
    pub const EXTRA_CONDENSED: Self = Self(0.625);
    /// Width that is 75% of normal.
    pub const CONDENSED: Self = Self(0.75);
    /// Width that is 87.5% of normal.
    pub const SEMI_CONDENSED: Self = Self(0.875);
    /// Width that is 100% of normal. This is the default value.
    pub const NORMAL: Self = Self(1.0);
    /// Width that is 112.5% of normal.
    pub const SEMI_EXPANDED: Self = Self(1.125);
    /// Width that is 125% of normal.
    pub const EXPANDED: Self = Self(1.25);
    /// Width that is 150% of normal.
    pub const EXTRA_EXPANDED: Self = Self(1.5);
    /// Width that is 200% of normal.
    pub const ULTRA_EXPANDED: Self = Self(2.0);

    pub(crate) fn as_parley(&self) -> parley::FontWidth {
        parley::FontWidth::from_ratio(self.0)
    }
}

impl Hash for FontWidth {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        OrderedFloat(self.0).hash(state);
    }
}

impl std::cmp::PartialEq for FontWidth {
    fn eq(&self, other: &Self) -> bool {
        OrderedFloat(self.0) == OrderedFloat(other.0)
    }
}
impl std::cmp::Eq for FontWidth {}

impl std::cmp::PartialOrd for FontWidth {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl std::cmp::Ord for FontWidth {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        OrderedFloat(self.0).cmp(&OrderedFloat(other.0))
    }
}

impl Default for FontWidth {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// An OpenType tag, typically a [feature tag](https://learn.microsoft.com/en-us/typography/opentype/spec/featurelist)
/// or [variation axis tag](https://learn.microsoft.com/en-us/typography/opentype/spec/dvaraxisreg). This is a 4-byte
/// identifier, typically represented as a 4-character string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Tag([u8; 4]);

impl Tag {
    pub(crate) fn as_swash(&self) -> parley::swash::Tag {
        parley::swash::tag_from_bytes(&self.0)
    }
}

impl FromStr for Tag {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.as_bytes().try_into().map_err(|_e| ())?))
    }
}

impl From<[u8; 4]> for Tag {
    fn from(value: [u8; 4]) -> Self {
        Self(value)
    }
}

impl From<&[u8; 4]> for Tag {
    fn from(value: &[u8; 4]) -> Self {
        Self(*value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontSetting<T> {
    pub tag: Tag,
    pub value: T,
}

impl<T> FontSetting<T> {
    pub fn new(tag: Tag, value: T) -> Self {
        Self { tag, value }
    }
}

impl<T: Clone> FontSetting<T> {
    pub(crate) fn as_parley(&self) -> parley::swash::Setting<T> {
        parley::swash::Setting {
            tag: self.tag.as_swash(),
            value: self.value.clone(),
        }
    }
}

impl<T, U: Into<Tag>> From<(U, T)> for FontSetting<T> {
    fn from(val: (U, T)) -> Self {
        Self {
            tag: val.0.into(),
            value: val.1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontSettings<T: 'static>(
    #[cfg_attr(
        feature = "serde",
        serde(bound(
            deserialize = "<[FontSetting<T>] as ToOwned>::Owned: serde::Deserialize<'de>"
        ))
    )]
    pub Cow<'static, [FontSetting<T>]>,
)
where
    [FontSetting<T>]: ToOwned,
    <[FontSetting<T>] as ToOwned>::Owned: std::fmt::Debug + PartialEq + Clone;

impl<T> FontSettings<T>
where
    <[parley::swash::Setting<T>] as ToOwned>::Owned:
        std::fmt::Debug + PartialEq + Clone + FromIterator<parley::swash::Setting<T>>,
    T: std::fmt::Debug + PartialEq + Clone,
{
    pub(crate) fn as_parley(&self) -> parley::FontSettings<'static, parley::swash::Setting<T>> {
        let settings = self.0.iter().map(|setting| setting.as_parley()).collect();
        parley::FontSettings::List(Cow::Owned(settings))
    }
}

impl<T> Default for FontSettings<T>
where
    [FontSetting<T>]: ToOwned,
    <[FontSetting<T>] as ToOwned>::Owned: std::fmt::Debug + PartialEq + Clone,
{
    fn default() -> Self {
        Self(Cow::Borrowed(&[]))
    }
}

impl<T: 'static> FontSettings<T>
where
    [FontSetting<T>]: ToOwned,
    <[FontSetting<T>] as ToOwned>::Owned: std::fmt::Debug + PartialEq + Clone,
{
    pub fn new<I: Into<FontSetting<T>>, U: IntoIterator<Item = I>>(iter: U) -> Self
    where
        [FontSetting<T>]: ToOwned<Owned = Vec<FontSetting<T>>>,
    {
        Self(Cow::Owned(
            iter.into_iter().map(|v| v.into()).collect::<Vec<_>>(),
        ))
    }
}

/// Set of OpenType [font variation axis](https://learn.microsoft.com/en-us/typography/opentype/spec/dvaraxisreg) values
/// (for variable fonts).
///
/// The `wght` and `wdth` axes can be controlled via [`FontWeight`] and [`FontWidth`] respectively. If you need to
/// control other, possibly font-specific, axes, you can use this.
#[derive(Debug, Clone, PartialEq, PartialOrd, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontVariations(FontSettings<f32>);

impl FontVariations {
    pub(crate) fn as_parley(&self) -> parley::FontSettings<'static, parley::swash::Setting<f32>> {
        self.0.as_parley()
    }
}

impl Hash for FontVariations {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let settings = &self.0 .0;
        state.write_usize(settings.len());
        for setting in settings.iter() {
            setting.tag.hash(state);
            OrderedFloat(setting.value).hash(state);
        }
    }
}

/// Set of OpenType [font feature](https://learn.microsoft.com/en-us/typography/opentype/spec/featurelist) values.
/// These can be used to enable or disable specific typographic features.
///
/// For more user-friendly access to common font features, see [`NamedFontVariants`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontFeatures(FontSettings<u16>);

impl FontFeatures {
    pub(crate) fn as_parley(&self) -> parley::FontSettings<'static, parley::swash::Setting<u16>> {
        self.0.as_parley()
    }
}

pub mod named_variants {
    use std::borrow::Cow;

    use super::{FontFeatures, FontSettings};

    /// Controls the selection of glyphs used for e.g. small caps, or for titling.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub enum CapsVariant {
        #[default]
        Normal,
        /// Enables display of small capitals for lowercase letters.
        SmallCaps,
        /// Enables display of small capitals for uppercase and lowercase letters.
        AllSmallCaps,
        /// Enables display of petite capitals for lowercase letters.
        PetiteCaps,
        /// Enables display of petite capitals for uppercase and lowercase letters.
        AllPetiteCaps,
        /// Enables display of small capitals for uppercase letters and normal lowercase letters.
        Unicase,
        /// Enables display of titling capitals (glyphs specifically designed for all-caps titles).
        TitlingCaps,
    }

    /// Controls the selection of specialized typographic subscript and superscript glyphs.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub enum PositionVariant {
        #[default]
        Normal,
        /// Enables display of subscript variants (glyphs specifically designed for use in subscripts).
        Sub,
        /// Enables display of superscript variants (glyphs specifically designed for use in superscripts).
        Super,
    }

    /// Controls the selection of numeric figure variants.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub enum NumericFigureVariant {
        #[default]
        Normal,
        /// Enables display of lining numerals (numerals that share the height of uppercase letters).
        LiningNumerals,
        /// Enables display of old-style numerals (numerals that share the height of lowercase letters).
        OldStyleNumerals,
    }

    /// Controls the selection of numeric spacing variants.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub enum NumericSpacingVariant {
        #[default]
        Normal,
        /// Enables display of proportional numerals.
        ProportionalNumerals,
        /// Enables display of tabular numerals (all digits are the same width).
        TabularNumerals,
    }

    /// Controls the selection of numeric fraction variants.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub enum NumericFractionVariant {
        /// Display fractions as normal text.
        #[default]
        Normal,
        /// Enables display of diagonal fractions (e.g. transforms "1/2" into a diagonal fraction).
        Diagonal,
        /// Enables display of stacked fractions (e.g. transforms "1/2" into a stacked fraction).
        Stacked,
    }

    /// Controls glyph substitution and sizing in East Asian text. The JIS variants reflect the forms defined in different
    /// Japanese national standards.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub enum EastAsianVariant {
        #[default]
        Normal,
        /// Enables rendering of JIS78 forms.
        Jis78,
        /// Enables rendering of JIS83 forms.
        Jis83,
        /// Enables rendering of JIS90 forms.
        Jis90,
        /// Enables rendering of JIS2004 forms.
        Jis04,
        /// Enables rendering of simplified forms.
        Simplified,
        /// Enables rendering of traditional forms.
        Traditional,
    }

    /// Controls the width variant of East Asian characters.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub enum EastAsianWidth {
        /// Enables rendering of full-width variants.
        FullWidth,
        /// Enables rendering of proportionally-spaced variants.
        Proportional,
    }

    /// Somewhat-common OpenType font features, as named and categorized by CSS. A more user-friendly way to obtain
    /// [`FontFeatures`] that carries more semantic meaning.
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
    pub struct NamedFontVariants<'a> {
        /// Enables display of common ligatures.
        pub common_ligatures: bool,
        /// Enables display of discretionary ligatures.
        pub discretionary_ligatures: bool,
        /// Enables display of historical ligatures.
        pub historical_ligatures: bool,
        /// Enables display of contextual alternates (typically used to substitute glyphs based on their surrounding
        /// context).
        pub contextual_alternates: bool,
        /// Selects typographic subscript or superscript glyphs.
        pub position: PositionVariant,
        /// Selects variants of glyphs for different types of capitalization (e.g. small caps).
        pub caps: CapsVariant,
        /// Selects the style of numeric figures.
        pub numeric_figures: NumericFigureVariant,
        /// Selects the spacing (proportional or tabular) of numeric features.
        pub numeric_spacing: NumericSpacingVariant,
        /// Selects the appearance of fractions.
        pub numeric_fractions: NumericFractionVariant,
        /// Enables letterforms used with ordinal numbers (e.g. the "st" and "nd" in "1st" and "2nd").
        pub ordinal: bool,
        /// Enables display of slashed zeros.
        pub slashed_zero: bool,
        /// Enables display of historical letterforms.
        pub historical_forms: bool,
        /// Enables display of font-specific stylistic alternates.
        pub stylistic_alternates: u16,
        /// Enables display of font-specific stylistic sets.
        pub stylesets: Cow<'a, [u16]>,
        /// Enables display of font-specific character variants.
        pub character_variants: Cow<'a, [u16]>,
        /// Enables display of font-specific swash glyphs.
        pub swash: u16,
        /// Enables replacement of default glyphs with font-specific ornaments (typically as replacements for the bullet
        /// character).
        pub ornaments: u16,
        /// Enables display of font-specific alternate annotation forms.
        pub annotation: u16,
        /// Selects the way East Asian glyphs are rendered.
        east_asian_variant: EastAsianVariant,
        /// Selects the width variants of East Asian glyphs.
        pub east_asian_width: Option<EastAsianWidth>,
        /// Enables display of ruby (superscript-like annotations) variant glyphs.
        pub ruby: bool,
    }

    impl Default for NamedFontVariants<'_> {
        fn default() -> Self {
            Self {
                common_ligatures: false,
                discretionary_ligatures: false,
                historical_ligatures: false,
                contextual_alternates: false,
                position: Default::default(),
                caps: Default::default(),
                numeric_figures: Default::default(),
                numeric_spacing: Default::default(),
                numeric_fractions: Default::default(),
                ordinal: false,
                slashed_zero: false,
                historical_forms: false,
                stylistic_alternates: 0,
                stylesets: Cow::Borrowed(&[]),
                character_variants: Cow::Borrowed(&[]),
                swash: 0,
                ornaments: 0,
                annotation: 0,
                east_asian_variant: Default::default(),
                east_asian_width: None,
                ruby: false,
            }
        }
    }

    impl From<NamedFontVariants<'_>> for FontFeatures {
        fn from(value: NamedFontVariants<'_>) -> Self {
            let mut features = vec![];

            if value.common_ligatures {
                features.extend([(*b"clig", 1), (*b"liga", 1)]);
            }
            if value.discretionary_ligatures {
                features.push((*b"dlig", 1));
            }
            if value.historical_ligatures {
                features.push((*b"hlig", 1));
            }
            if value.contextual_alternates {
                features.push((*b"calt", 1));
            }
            match value.position {
                PositionVariant::Normal => {}
                PositionVariant::Sub => {
                    features.push((*b"subs", 1));
                }
                PositionVariant::Super => {
                    features.push((*b"sups", 1));
                }
            }
            match value.caps {
                CapsVariant::Normal => {}
                CapsVariant::SmallCaps => {
                    features.push((*b"smcp", 1));
                }
                CapsVariant::AllSmallCaps => features.extend([(*b"smcp", 1), (*b"c2pc", 1)]),
                CapsVariant::PetiteCaps => {
                    features.push((*b"pcap", 1));
                }
                CapsVariant::AllPetiteCaps => features.extend([(*b"pcap", 1), (*b"c2pc", 1)]),
                CapsVariant::Unicase => {
                    features.push((*b"unic", 1));
                }
                CapsVariant::TitlingCaps => {
                    features.push((*b"titl", 1));
                }
            }
            match value.numeric_figures {
                NumericFigureVariant::Normal => {}
                NumericFigureVariant::LiningNumerals => {
                    features.push((*b"lnum", 1));
                }
                NumericFigureVariant::OldStyleNumerals => {
                    features.push((*b"onum", 1));
                }
            }
            match value.numeric_spacing {
                NumericSpacingVariant::Normal => {}
                NumericSpacingVariant::ProportionalNumerals => {
                    features.push((*b"pnum", 1));
                }
                NumericSpacingVariant::TabularNumerals => {
                    features.push((*b"tnum", 1));
                }
            }
            match value.numeric_fractions {
                NumericFractionVariant::Normal => {}
                NumericFractionVariant::Diagonal => {
                    features.push((*b"frac", 1));
                }
                NumericFractionVariant::Stacked => {
                    features.push((*b"afrc", 1));
                }
            }
            if value.ordinal {
                features.push((*b"ordn", 1));
            }
            if value.slashed_zero {
                features.push((*b"zero", 1));
            }
            if value.historical_forms {
                features.push((*b"hist", 1));
            }
            if value.stylistic_alternates > 0 {
                features.push((*b"salt", value.stylistic_alternates));
            }
            features.extend(value.stylesets.iter().filter_map(|styleset| {
                if *styleset > 20 {
                    return None;
                }
                let styleset = format!("ss{styleset:02}");
                let styleset_tag: [u8; 4] = styleset.into_bytes().try_into().ok()?;
                Some((styleset_tag, 1u16))
            }));
            features.extend(value.character_variants.iter().filter_map(|cvar| {
                if *cvar > 99 {
                    return None;
                }
                let styleset = format!("cv{cvar:02}");
                let styleset_tag: [u8; 4] = styleset.into_bytes().try_into().ok()?;
                Some((styleset_tag, 1u16))
            }));
            if value.swash > 0 {
                features.push((*b"swsh", value.swash));
            }
            if value.ornaments > 0 {
                features.push((*b"ornm", value.ornaments));
            }
            if value.annotation > 0 {
                features.push((*b"nalt", value.annotation));
            }
            match value.east_asian_variant {
                EastAsianVariant::Normal => {}
                EastAsianVariant::Jis78 => {
                    features.push((*b"jp78", 1));
                }
                EastAsianVariant::Jis83 => {
                    features.push((*b"jp83", 1));
                }
                EastAsianVariant::Jis90 => {
                    features.push((*b"jp90", 1));
                }
                EastAsianVariant::Jis04 => {
                    features.push((*b"jp04", 1));
                }
                EastAsianVariant::Simplified => {
                    features.push((*b"smpl", 1));
                }
                EastAsianVariant::Traditional => {
                    features.push((*b"trad", 1));
                }
            }
            match value.east_asian_width {
                Some(EastAsianWidth::FullWidth) => {
                    features.push((*b"fwid", 1));
                }
                Some(EastAsianWidth::Proportional) => {
                    features.push((*b"pwid", 1));
                }
                None => {}
            }
            if value.ruby {
                features.push((*b"ruby", 1));
            }

            Self(FontSettings::new(features))
        }
    }
}

/// All the properties of a given piece of text that affect its layout.
#[derive(Debug, Clone, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FontId {
    /// The font family to use.
    pub family: FontStack,
    /// The font size, in points.
    pub size: f32,
    /// The font weight.
    pub weight: FontWeight,
    /// The font width / "condensedness".
    pub width: FontWidth,
    /// OpenType font variation axes (for variable fonts).
    pub variations: Option<Arc<FontVariations>>,
    /// OpenType font features. These can be initialized more easily via [`NamedFontVariants`].
    pub features: Option<Arc<FontFeatures>>,
}

impl FontId {
    /// Create a new [`FontStyle`] from a given font size and font family or stack of font families. All other font
    /// settings will be set to their default values.
    pub fn simple(size: f32, family: impl Into<FontStack>) -> Self {
        Self {
            family: family.into(),
            size,
            ..Default::default()
        }
    }

    pub fn with_family(&self, family: impl Into<FontStack>) -> Self {
        Self {
            family: family.into(),
            ..self.clone()
        }
    }

    pub fn with_size(&self, size: f32) -> Self {
        Self {
            size,
            ..self.clone()
        }
    }

    pub fn with_weight(&self, weight: FontWeight) -> Self {
        Self {
            weight,
            ..self.clone()
        }
    }

    pub fn with_width(&self, width: FontWidth) -> Self {
        Self {
            width,
            ..self.clone()
        }
    }

    pub fn with_variations(&self, variations: Option<Arc<FontVariations>>) -> Self {
        Self {
            variations,
            ..self.clone()
        }
    }

    pub fn with_features(&self, features: Option<Arc<FontFeatures>>) -> Self {
        Self {
            features,
            ..self.clone()
        }
    }

    pub fn with_named_variants(&self, named_variants: NamedFontVariants<'_>) -> Self {
        Self {
            features: Some(Arc::new(named_variants.into())),
            ..self.clone()
        }
    }

    /// Create a new [`FontStyle`] of the given font size and the [`GenericFamily::SystemUi`] family.
    pub fn system_ui(size: f32) -> Self {
        Self::simple(size, GenericFamily::SystemUi)
    }

    /// Create a new [`FontStyle`] of the given font size and the [`GenericFamily::Serif`] family.
    pub fn serif(size: f32) -> Self {
        Self::simple(size, GenericFamily::Serif)
    }

    /// Create a new [`FontStyle`] of the given font size and the [`GenericFamily::SansSerif`] family.
    pub fn sans_serif(size: f32) -> Self {
        Self::simple(size, GenericFamily::SansSerif)
    }

    /// Create a new [`FontStyle`] of the given font size and the [`GenericFamily::Monospace`] family.
    pub fn monospace(size: f32) -> Self {
        Self::simple(size, GenericFamily::Monospace)
    }

    /// Create a new [`FontStyle`] of the given font size and the [`GenericFamily::Cursive`] family.
    pub fn cursive(size: f32) -> Self {
        Self::simple(size, GenericFamily::Cursive)
    }

    /// Create a new [`FontStyle`] of the given font size and the [`GenericFamily::Emoji`] family.
    pub fn emoji(size: f32) -> Self {
        Self::simple(size, GenericFamily::Emoji)
    }
}

impl Default for FontId {
    fn default() -> Self {
        Self {
            family: Default::default(),
            size: 14.0,
            weight: Default::default(),
            width: Default::default(),
            variations: Default::default(),
            features: Default::default(),
        }
    }
}

impl Hash for FontId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.family.hash(state);
        OrderedFloat(self.size).hash(state);
        self.weight.hash(state);
        self.width.hash(state);
        self.variations.hash(state);
        self.features.hash(state);
    }
}

impl std::cmp::PartialEq for FontId {
    fn eq(&self, other: &Self) -> bool {
        self.family == other.family
            && OrderedFloat(self.size) == OrderedFloat(other.size)
            && self.weight == other.weight
            && self.width == other.width
            && self.variations == other.variations
            && self.features == other.features
    }
}
impl std::cmp::Eq for FontId {}

// ----------------------------------------------------------------------------

/// Style / formatting for a section of text. Includes not only the [`FontStyle`] but also things like color,
/// strikethrough, background color, etc.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextFormat {
    pub font_id: FontId,

    /// Extra spacing between letters, in points.
    ///
    /// Default: 0.0.
    ///
    /// For even text it is recommended you round this to an even number of _pixels_.
    pub extra_letter_spacing: f32,

    /// Explicit line height of the text in points.
    ///
    /// This is the distance between the bottom row of two subsequent lines of text.
    ///
    /// If `None` (the default), the line height is determined by the font.
    ///
    /// For even text it is recommended you round this to an even number of _pixels_.
    pub line_height: Option<f32>,

    /// Text color
    pub color: Color32,

    pub background: Color32,

    /// Amount to expand background fill by.
    ///
    /// Default: 1.0
    pub expand_bg: f32,

    pub italics: bool,

    pub underline: Stroke,

    pub strikethrough: Stroke,

    /// If you use a small font and [`Align::TOP`] you
    /// can get the effect of raised text.
    ///
    /// If you use a small font and [`Align::BOTTOM`]
    /// you get the effect of a subscript.
    ///
    /// If you use [`Align::Center`], you get text that is centered
    /// around a common center-line, which is nice when mixining emojis
    /// and normal text in e.g. a button.
    pub valign: Align,
}

impl TextFormat {
    pub(crate) fn as_parley(&self) -> parley::TextStyle<'static, Color32> {
        parley::TextStyle {
            font_stack: self.font_id.family.as_parley(),
            font_size: self.font_id.size,
            font_width: self.font_id.width.as_parley(),
            font_style: if self.italics {
                parley::FontStyle::Italic
            } else {
                parley::FontStyle::Normal
            },
            font_weight: self.font_id.weight.as_parley(),
            font_variations: self.font_id.variations.as_ref().map_or_else(
                || parley::FontSettings::List(Cow::Borrowed(&[])),
                |s| s.as_parley(),
            ),
            font_features: self.font_id.features.as_ref().map_or_else(
                || parley::FontSettings::List(Cow::Borrowed(&[])),
                |s| s.as_parley(),
            ),
            locale: None,
            brush: self.color,
            has_underline: !self.underline.is_empty(),
            underline_offset: None,
            underline_size: (!self.underline.is_empty()).then_some(self.underline.width),
            underline_brush: (!self.underline.is_empty()).then_some(self.underline.color),
            has_strikethrough: !self.strikethrough.is_empty(),
            strikethrough_offset: None,
            strikethrough_size: (!self.strikethrough.is_empty())
                .then_some(self.strikethrough.width),
            strikethrough_brush: (!self.strikethrough.is_empty())
                .then_some(self.strikethrough.color),
            line_height: self.line_height() / self.font_id.size,
            word_spacing: 0.0,
            letter_spacing: self.extra_letter_spacing,
        }
    }
}

impl Default for TextFormat {
    #[inline]
    fn default() -> Self {
        Self {
            font_id: FontId::default(),
            extra_letter_spacing: 0.0,
            line_height: None,
            color: Color32::GRAY,
            background: Color32::TRANSPARENT,
            expand_bg: 1.0,
            italics: false,
            underline: Stroke::NONE,
            strikethrough: Stroke::NONE,
            valign: Align::BOTTOM,
        }
    }
}

impl std::hash::Hash for TextFormat {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self {
            font_id: font,
            extra_letter_spacing,
            line_height,
            color,
            background,
            expand_bg,
            italics,
            underline,
            strikethrough,
            valign,
        } = self;
        font.hash(state);
        OrderedFloat(*extra_letter_spacing).hash(state);
        if let Some(line_height) = *line_height {
            OrderedFloat(line_height).hash(state);
        }
        color.hash(state);
        background.hash(state);
        OrderedFloat(*expand_bg).hash(state);
        italics.hash(state);
        underline.hash(state);
        strikethrough.hash(state);
        valign.hash(state);
    }
}

impl TextFormat {
    #[inline]
    pub fn simple(font: FontId, color: Color32) -> Self {
        Self {
            font_id: font,
            color,
            ..Default::default()
        }
    }

    pub(crate) fn line_height(&self) -> f32 {
        self.line_height.unwrap_or(self.font_id.size)
    }
}
