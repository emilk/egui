use std::{collections::BTreeMap, sync::Arc};

use crate::text::{FontData, FontFamily, FontTweak};

#[cfg(feature = "default_fonts")]
use epaint_default_fonts::{EMOJI_ICON, HACK_REGULAR, NOTO_EMOJI_REGULAR, UBUNTU_LIGHT};

/// Describes the font data and the sizes to use.
///
/// Often you would start with [`FontDefinitions::default()`] and then add/change the contents.
///
/// This is how you install your own custom fonts:
/// ```
/// # use {epaint::text::{FontDefinitions, FontFamily, FontData}};
/// # struct FakeEguiCtx {};
/// # impl FakeEguiCtx { fn set_fonts(&self, _: FontDefinitions) {} }
/// # let egui_ctx = FakeEguiCtx {};
/// let mut fonts = FontDefinitions::default();
///
/// // Install my own font (maybe supporting non-latin characters):
/// fonts.font_data.insert("my_font".to_owned(),
///    std::sync::Arc::new(
///        // .ttf and .otf supported
///        FontData::from_static(include_bytes!("../../../epaint_default_fonts/fonts/Ubuntu-Light.ttf"))
///    )
/// );
///
/// // Put my font first (highest priority):
/// fonts.families.get_mut(&FontFamily::Proportional).unwrap()
///     .insert(0, "my_font".to_owned());
///
/// // Put my font as last fallback for monospace:
/// fonts.families.get_mut(&FontFamily::Monospace).unwrap()
///     .push("my_font".to_owned());
///
/// egui_ctx.set_fonts(fonts);
/// ```
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct FontDefinitions {
    /// List of font names and their definitions.
    ///
    /// `epaint` has built-in-default for these, but you can override them if you like.
    pub font_data: BTreeMap<String, Arc<FontData>>,

    /// Which fonts (names) to use for each [`FontFamily`].
    ///
    /// The list should be a list of keys into [`Self::font_data`].
    /// When looking for a character glyph `epaint` will start with
    /// the first font and then move to the second, and so on.
    /// So the first font is the primary, and then comes a list of fallbacks in order of priority.
    pub families: BTreeMap<FontFamily, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct FontInsert {
    /// Font name
    pub name: String,

    /// A `.ttf` or `.otf` file and a font face index.
    pub data: FontData,

    /// Sets the font family and priority
    pub families: Vec<InsertFontFamily>,
}

#[derive(Debug, Clone)]
pub struct InsertFontFamily {
    /// Font family
    pub family: FontFamily,

    /// Fallback or Primary font
    pub priority: FontPriority,
}

#[derive(Debug, Clone)]
pub enum FontPriority {
    /// Prefer this font before all existing ones.
    ///
    /// If a desired glyph exists in this font, it will be used.
    Highest,

    /// Use this font as a fallback, after all existing ones.
    ///
    /// This font will only be used if the glyph is not found in any of the previously installed fonts.
    Lowest,
}

impl FontInsert {
    pub fn new(name: &str, data: FontData, families: Vec<InsertFontFamily>) -> Self {
        Self {
            name: name.to_owned(),
            data,
            families,
        }
    }
}

impl Default for FontDefinitions {
    /// Specifies the default fonts if the feature `default_fonts` is enabled,
    /// otherwise this is the same as [`Self::empty`].
    #[cfg(not(feature = "default_fonts"))]
    fn default() -> Self {
        Self::empty()
    }

    /// Specifies the default fonts if the feature `default_fonts` is enabled,
    /// otherwise this is the same as [`Self::empty`].
    #[cfg(feature = "default_fonts")]
    fn default() -> Self {
        let mut font_data: BTreeMap<String, Arc<FontData>> = BTreeMap::new();

        let mut families = BTreeMap::new();

        font_data.insert(
            "Hack".to_owned(),
            Arc::new(FontData::from_static(HACK_REGULAR)),
        );

        // Some good looking emojis. Use as first priority:
        font_data.insert(
            "NotoEmoji-Regular".to_owned(),
            Arc::new(FontData::from_static(NOTO_EMOJI_REGULAR).tweak(FontTweak {
                scale: 0.81, // Make smaller
                ..Default::default()
            })),
        );

        font_data.insert(
            "Ubuntu-Light".to_owned(),
            Arc::new(FontData::from_static(UBUNTU_LIGHT)),
        );

        // Bigger emojis, and more. <http://jslegers.github.io/emoji-icon-font/>:
        font_data.insert(
            "emoji-icon-font".to_owned(),
            Arc::new(FontData::from_static(EMOJI_ICON).tweak(FontTweak {
                scale: 0.90, // Make smaller
                ..Default::default()
            })),
        );

        families.insert(
            FontFamily::Monospace,
            vec![
                "Hack".to_owned(),
                "Ubuntu-Light".to_owned(), // fallback for √ etc
                "NotoEmoji-Regular".to_owned(),
                "emoji-icon-font".to_owned(),
            ],
        );
        families.insert(
            FontFamily::Proportional,
            vec![
                "Ubuntu-Light".to_owned(),
                "NotoEmoji-Regular".to_owned(),
                "emoji-icon-font".to_owned(),
            ],
        );

        Self {
            font_data,
            families,
        }
    }
}

impl FontDefinitions {
    /// No fonts.
    pub fn empty() -> Self {
        let mut families = BTreeMap::new();
        families.insert(FontFamily::Monospace, vec![]);
        families.insert(FontFamily::Proportional, vec![]);

        Self {
            font_data: Default::default(),
            families,
        }
    }

    /// List of all the builtin font names used by `epaint`.
    #[cfg(feature = "default_fonts")]
    pub fn builtin_font_names() -> &'static [&'static str] {
        &[
            "Ubuntu-Light",
            "NotoEmoji-Regular",
            "emoji-icon-font",
            "Hack",
        ]
    }

    /// List of all the builtin font names used by `epaint`.
    #[cfg(not(feature = "default_fonts"))]
    pub fn builtin_font_names() -> &'static [&'static str] {
        &[]
    }
}
