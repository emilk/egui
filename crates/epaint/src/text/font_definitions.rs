use std::{collections::BTreeMap, sync::Arc};

use crate::text::{FontData, FontFamily, font_provider::FontProvider};

#[cfg(feature = "default_fonts")]
use crate::text::FontTweak;

#[cfg(feature = "default_fonts")]
use epaint_default_fonts::{EGUI_ICONS, HACK_REGULAR, UBUNTU_LIGHT};
#[cfg(feature = "monochrome_emoji_fonts")]
use epaint_default_fonts::{EMOJI_ICON, NOTO_EMOJI_REGULAR};

/// Describes a set of pre-configured fonts.
///
/// These are the fonts epaint will use first, before falling back to any system fonts
/// (if configured).
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

/// A font to add to [`FontDefinitions`], and which families to add it to.
///
/// Used by `egui::Context::add_font`.
#[derive(Debug, Clone)]
pub struct FontInsert {
    /// Font name
    pub name: String,

    /// A `.ttf` or `.otf` file and a font face index.
    pub data: FontData,

    /// Sets the font family and priority
    pub families: Vec<InsertFontFamily>,
}

/// Where in the fallback chain of a [`FontFamily`] a [`FontInsert`] goes.
#[derive(Debug, Clone)]
pub struct InsertFontFamily {
    /// Font family
    pub family: FontFamily,

    /// Fallback or Primary font
    pub priority: FontPriority,
}

/// Whether an inserted font goes before or after the existing fonts of a family.
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

        font_data.insert(
            "Ubuntu-Light".to_owned(),
            Arc::new(FontData::from_static(UBUNTU_LIGHT)),
        );

        // The handful of icons in `egui::special_emojis`, which no platform font has:
        font_data.insert(
            "egui-icons".to_owned(),
            Arc::new(FontData::from_static(EGUI_ICONS).tweak(FontTweak {
                scale: 0.90, // Make smaller
                ..Default::default()
            })),
        );

        #[cfg(feature = "monochrome_emoji_fonts")]
        {
            // Some good looking emojis:
            font_data.insert(
                "NotoEmoji-Regular".to_owned(),
                Arc::new(FontData::from_static(NOTO_EMOJI_REGULAR).tweak(FontTweak {
                    scale: 0.81, // Make smaller
                    ..Default::default()
                })),
            );

            // Bigger emojis, and more. <http://jslegers.github.io/emoji-icon-font/>:
            font_data.insert(
                "emoji-icon-font".to_owned(),
                Arc::new(FontData::from_static(EMOJI_ICON).tweak(FontTweak {
                    scale: 0.90, // Make smaller
                    ..Default::default()
                })),
            );
        }

        // Last resort, after the fonts that cover the text of a script:
        let fallback_fonts: &[&str] = if cfg!(feature = "monochrome_emoji_fonts") {
            &["egui-icons", "NotoEmoji-Regular", "emoji-icon-font"]
        } else {
            &["egui-icons"]
        };
        let family = |fonts: &[&str]| -> Vec<String> {
            core::iter::chain(fonts, fallback_fonts)
                .map(|name| (*name).to_owned())
                .collect()
        };

        families.insert(
            FontFamily::Monospace,
            family(&[
                "Hack",
                "Ubuntu-Light", // fallback for √ etc
            ]),
        );
        families.insert(FontFamily::Proportional, family(&["Ubuntu-Light"]));

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
        if cfg!(feature = "monochrome_emoji_fonts") {
            &[
                "Ubuntu-Light",
                "egui-icons",
                "NotoEmoji-Regular",
                "emoji-icon-font",
                "Hack",
            ]
        } else {
            &["Ubuntu-Light", "egui-icons", "Hack"]
        }
    }

    /// List of all the builtin font names used by `epaint`.
    #[cfg(not(feature = "default_fonts"))]
    pub fn builtin_font_names() -> &'static [&'static str] {
        &[]
    }
}

/// The configured fonts are the head of every family's fallback chain:
/// for each family they are handed out in the order they are listed,
/// so the same text looks the same on every machine.
///
/// [`FontDefinitions`] never discovers anything on demand,
/// and is always the first [`FontProvider`] asked.
impl FontProvider for FontDefinitions {
    fn fonts_for_family(&self, family: &FontFamily) -> Vec<FontInsert> {
        let Some(font_names) = self.families.get(family) else {
            log::warn!("FontFamily::{family:?} is not bound to any fonts");
            return Vec::new();
        };

        font_names
            .iter()
            .map(|name| {
                let data = self.font_data.get(name).unwrap_or_else(|| {
                    let available: Vec<&String> = self.font_data.keys().collect();
                    panic!("No font data found for {name:?}. Configured fonts: {available:?}")
                });
                FontInsert {
                    name: name.clone(),
                    data: (**data).clone(),
                    families: Vec::new(),
                }
            })
            .collect()
    }
}
