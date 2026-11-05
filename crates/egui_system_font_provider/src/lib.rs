//! Load system fonts on demand for characters that the fonts installed in egui lack.
//!
//! [`SystemFontProvider`] is an [`epaint::text::FontProvider`] backed by [`fontique`],
//! which uses the operating system's own font fallback (CoreText, DirectWrite, fontconfig,
//! or Android's `fonts.xml`) to find an installed font for a script and locale.
//!
//! ```no_run
//! # let egui_ctx = egui::Context::default();
//! egui_ctx.add_font_provider(std::sync::Arc::new(egui_system_font_provider::SystemFontProvider::new()));
//! ```

use epaint::{
    mutex::Mutex,
    text::{
        FallbackRequest, FontData, FontFamily, FontInsert, FontPriority, FontProvider,
        InsertFontFamily,
    },
};
use fontique::{
    Collection, CollectionOptions, FallbackKey, GenericFamily, Language, QueryFont, QueryStatus,
    Script, SourceCache, SourceCacheOptions,
};
use poll_promise::Promise;
use read_fonts::{
    TableProvider as _,
    tables::layout::{FeatureList, FeatureRecord},
    types::Tag,
};

struct SystemFonts {
    collection: Collection,
    source_cache: SourceCache,
}

/// Finds installed system fonts for characters that the egui fonts lack.
///
/// Uses the operating system's own font fallback via [`fontique`],
/// so e.g. 日本語 gets a Japanese font and العربية an Arabic font.
/// The font files are memory-mapped, not copied.
///
/// Enumerating the system fonts can take hundreds of milliseconds,
/// so [`Self::new`] starts doing that on a background thread.
/// A lookup before it is done blocks until it is.
pub struct SystemFontProvider {
    /// Loaded on a background thread by [`Self::new`].
    fonts: Mutex<Promise<SystemFonts>>,
    locale: Option<Language>,
}

impl Default for SystemFontProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemFontProvider {
    /// Start enumerating the system fonts on a background thread.
    ///
    /// Uses the system locale to pick between fonts (see [`Self::with_locale`]).
    pub fn new() -> Self {
        Self {
            fonts: Mutex::new(Promise::spawn_thread(
                "egui_system_font_provider",
                load_system_fonts,
            )),
            locale: sys_locale::get_locale().and_then(|locale| parse_locale(&locale)),
        }
    }

    /// The locale used to pick between fonts for the same script,
    /// e.g. between a Japanese and a Simplified Chinese font for CJK ideographs.
    ///
    /// A BCP-47 language tag like `"ja"` or `"zh-Hans"`, or `None` for the platform default.
    ///
    /// Default: the system locale.
    #[inline]
    pub fn with_locale(mut self, locale: Option<&str>) -> Self {
        self.locale = locale.and_then(parse_locale);
        self
    }
}

fn parse_locale(locale: &str) -> Option<Language> {
    match Language::parse(locale) {
        Ok(language) => Some(language),
        Err(err) => {
            log::warn!("Failed to parse locale {locale:?}: {err:?}");
            None
        }
    }
}

fn load_system_fonts() -> SystemFonts {
    let start = std::time::Instant::now();
    let collection = Collection::new(CollectionOptions {
        shared: false,
        system_fonts: true,
    });
    log::debug!("Enumerated system fonts in {:?}", start.elapsed());
    SystemFonts {
        collection,
        source_cache: SourceCache::new(SourceCacheOptions::default()),
    }
}

impl FontProvider for SystemFontProvider {
    fn font_for(&self, request: &FallbackRequest<'_>) -> Option<FontInsert> {
        let FallbackRequest { cluster, family } = request;
        let base_char = cluster.chars().next()?;

        // Blocks if the background thread is not done enumerating the system fonts yet:
        let mut fonts = self.fonts.lock();
        let SystemFonts {
            collection,
            source_cache,
        } = fonts.block_until_ready_mut();

        // A monospace font that cannot render the script (see `can_render`) must not win
        // just because it is first, so the chain falls through to sans-serif:
        // Arabic in a monospace label may then render proportionally.
        // That is better than tofu or disconnected letters, and no real monospace Arabic
        // font ships on most systems; we do not synthesize monospacing.
        let generics: &[GenericFamily] = match family {
            FontFamily::Monospace => &[
                GenericFamily::Monospace,
                GenericFamily::SansSerif,
                GenericFamily::SystemUi,
            ],
            FontFamily::Proportional | FontFamily::Name(_) => &[GenericFamily::SansSerif],
        };

        let font = if let Some(script) = cluster_script(cluster) {
            // The fallback for a script may be a single font for one locale,
            // e.g. a Japanese font for Han, which lacks many Simplified Chinese characters.
            // So if it can't render the char, try the other locales of the script:
            // TODO(https://github.com/linebender/parley/issues/644): remove `other_locales`
            // once fontique falls back per char (https://github.com/linebender/parley/pull/689).
            core::iter::chain(
                [self.locale.as_ref()],
                other_locales(script).iter().map(Some),
            )
            .find_map(|locale| {
                let mut query = collection.query(source_cache);
                query.set_families(generics.iter().copied());
                query.set_fallbacks(fallback_key(script, locale));
                find_font(&mut query, base_char)
            })
        } else {
            // Punctuation, symbols, emoji, etc. belong to no script, so there is no fallback for them.
            // Try the generic families instead.
            let mut query = collection.query(source_cache);
            query.set_families(
                generics
                    .iter()
                    .copied()
                    .chain([
                        GenericFamily::SystemUi,
                        GenericFamily::Emoji,
                        GenericFamily::Math,
                        GenericFamily::Serif,
                    ])
                    .chain(
                        (!generics.contains(&GenericFamily::Monospace))
                            .then_some(GenericFamily::Monospace),
                    ),
            );
            find_font(&mut query, base_char)
        }?;

        let family_name = collection.family_name(font.family.0).unwrap_or("unknown");
        let name = format!("system:{family_name}:{}:{}", font.family.1, font.index);
        log::debug!("Using {name:?} for {base_char:?}");

        let (blob, _blob_id) = font.blob.into_raw_parts();
        Some(FontInsert {
            name,
            data: FontData::from_blob(blob, font.index),
            families: vec![InsertFontFamily {
                family: (*family).clone(),
                priority: FontPriority::Lowest,
            }],
        })
    }
}

/// The first font in the query that can render `c`.
fn find_font(query: &mut fontique::Query<'_>, c: char) -> Option<QueryFont> {
    let mut found = None;
    query.matches_with(|font| {
        if can_render(font, c) {
            found = Some(font.clone());
            QueryStatus::Stop
        } else {
            QueryStatus::Continue
        }
    });
    found
}

const HAN: Script = Script::from_bytes(*b"Hani");

/// The locales of the Han fonts, starting with Simplified Chinese.
///
/// Simplified Chinese is `zh-SG` rather than `zh-CN`, because fontique treats `zh-CN`
/// as the default Han locale. Its font is whatever the first locale-less Han lookup returned,
/// which every `Query::set_fallbacks` does, so it is often the Japanese font.
static HAN_LOCALES: std::sync::LazyLock<Vec<Language>> = std::sync::LazyLock::new(|| {
    ["zh-SG", "zh-TW", "ja", "ko"]
        .iter()
        .filter_map(|locale| Language::parse(locale).ok())
        .collect()
});

/// The fallback key for a script and locale, avoiding fontique's default Han locale (see [`HAN_LOCALES`]).
fn fallback_key(script: Script, locale: Option<&Language>) -> FallbackKey {
    let locale = match locale {
        Some(locale) if script == HAN && is_simplified_chinese(locale) => HAN_LOCALES.first(),
        locale => locale,
    };
    FallbackKey::new(script, locale)
}

/// Is this a Simplified Chinese locale, like `zh`, `zh-CN` or `zh-Hans`?
///
/// Mirrors how fontique picks between Simplified and Traditional Chinese.
fn is_simplified_chinese(locale: &Language) -> bool {
    locale.language() == "zh"
        && locale.script() != Some("Hant")
        && !matches!(locale.region(), Some("HK" | "TW" | "MO"))
}

/// Locales to try for a script when the fallback for the preferred locale lacks a char.
///
/// Only Han needs this: its fonts are per locale, and each covers a different set of characters.
fn other_locales(script: Script) -> &'static [Language] {
    if script == HAN { &HAN_LOCALES } else { &[] }
}

/// The script of a character, or `None` for characters shared between scripts (punctuation, symbols, …).
fn script_of(c: char) -> Option<Script> {
    use unicode_script::UnicodeScript as _;

    match c.script() {
        unicode_script::Script::Common
        | unicode_script::Script::Inherited
        | unicode_script::Script::Unknown => None,
        script => Script::parse(script.short_name()).ok(),
    }
}

/// The script of a grapheme cluster: the script of its first character, or, if that is
/// one of the script-neutral scripts (Common, Inherited, Unknown), the script of the
/// first later character that has one.
///
/// A fallback request carries a whole grapheme cluster, which can be a base character
/// plus combining marks, and the base may be script-neutral while a later character
/// is not.
fn cluster_script(cluster: &str) -> Option<Script> {
    cluster.chars().find_map(script_of)
}

/// The GSUB features that connect Arabic letters into words.
const ARABIC_JOINING_FEATURES: [Tag; 3] = [Tag::new(b"init"), Tag::new(b"medi"), Tag::new(b"fina")];

/// Does the font shape Arabic contextual joining forms?
///
/// Arabic letters connect into words through the `init`, `medi` and `fina` GSUB features
/// (see the Microsoft Arabic script typography spec). A font that has glyphs for the
/// isolated codepoints but none of these features renders Arabic as disconnected
/// letters, so [`can_render`] rejects it for Arabic characters. Legacy AAT fonts shape
/// joining with a `morx` table instead of GSUB, which also counts. The older `mort`
/// table is not exposed by `read-fonts`, so it is not accepted.
fn has_arabic_shaping(font_ref: &skrifa::FontRef<'_>) -> bool {
    if font_ref.morx().is_ok() {
        return true;
    }
    font_ref
        .gsub()
        .ok()
        .and_then(|gsub| gsub.feature_list().ok())
        .is_some_and(|feature_list| feature_list_covers(&feature_list, &ARABIC_JOINING_FEATURES))
}

/// Does a GSUB feature list contain every tag in `needed`?
fn feature_list_covers(feature_list: &FeatureList<'_>, needed: &[Tag]) -> bool {
    let tags: Vec<Tag> = feature_list
        .feature_records()
        .iter()
        .map(FeatureRecord::feature_tag)
        .collect();
    tags_cover(&tags, needed)
}

/// Does `tags` contain every tag in `needed`?
///
/// Split from [`feature_list_covers`] so it can be tested without a font fixture.
fn tags_cover(tags: &[Tag], needed: &[Tag]) -> bool {
    needed
        .iter()
        .all(|needed_tag| tags.iter().any(|tag| tag == needed_tag))
}

/// Does the font have a glyph for the character that epaint can render?
///
/// That is an outline, a bitmap (`sbix`, `CBDT`), or a `COLR` glyph.
fn can_render(font: &QueryFont, c: char) -> bool {
    use skrifa::MetadataProvider as _;

    let Some(glyph_id) = font.charmap().and_then(|charmap| charmap.map(c)) else {
        return false;
    };
    let glyph_id = skrifa::GlyphId::new(glyph_id);
    if glyph_id == skrifa::GlyphId::NOTDEF {
        return false;
    }
    let Ok(font_ref) = skrifa::FontRef::from_index(font.blob.as_ref(), font.index) else {
        return false;
    };
    if !(font_ref.outline_glyphs().get(glyph_id).is_some()
        || font_ref.color_glyphs().get(glyph_id).is_some()
        || font_ref
            .bitmap_strikes()
            .glyph_for_size(skrifa::instance::Size::unscaled(), glyph_id)
            .is_some())
    {
        return false;
    }
    // A font that maps Arabic codepoints but cannot shape them renders Arabic words as
    // disconnected letters, so for Arabic characters also require joining support
    // (see [`has_arabic_shaping`]).
    script_of(c) != Some(ARAB) || has_arabic_shaping(&font_ref)
}

const ARAB: Script = Script::from_bytes(*b"Arab");

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn scripts() {
        assert_eq!(script_of('a'), Some(Script::from_bytes(*b"Latn")));
        assert_eq!(script_of('日'), Some(Script::from_bytes(*b"Hani")));
        assert_eq!(script_of('ع'), Some(Script::from_bytes(*b"Arab")));
        assert_eq!(script_of('1'), None);
        assert_eq!(script_of('⌥'), None);
        assert_eq!(script_of('\u{0301}'), None); // combining acute accent
    }

    #[test]
    fn cluster_scripts() {
        assert_eq!(cluster_script("ع"), Some(Script::from_bytes(*b"Arab")));
        // A base plus combining mark takes the script of the base:
        assert_eq!(cluster_script("بّ"), Some(Script::from_bytes(*b"Arab")));
        // A script-neutral character followed by a script-bearing one takes that script:
        assert_eq!(cluster_script("‘ع’"), Some(Script::from_bytes(*b"Arab")));
        // A cluster of only script-neutral characters has no script:
        assert_eq!(cluster_script("\u{0651}"), None); // combining shadda: all-Inherited
        assert_eq!(cluster_script("1\u{0654}"), None); // digit + combining hamza above
        assert_eq!(cluster_script(""), None);
    }

    #[test]
    fn feature_tags_cover() {
        let all = [Tag::new(b"init"), Tag::new(b"medi"), Tag::new(b"fina")];
        assert!(tags_cover(&all, &ARABIC_JOINING_FEATURES));
        // A superset of the joining features also covers them:
        assert!(tags_cover(
            &[
                Tag::new(b"c2sc"),
                Tag::new(b"liga"),
                Tag::new(b"init"),
                Tag::new(b"medi"),
                Tag::new(b"fina"),
                Tag::new(b"rlig"),
            ],
            &ARABIC_JOINING_FEATURES
        ));
        // Missing `medi` does not:
        assert!(!tags_cover(
            &[Tag::new(b"init"), Tag::new(b"fina")],
            &ARABIC_JOINING_FEATURES
        ));
        // Nothing needs to be covered when nothing is needed:
        assert!(tags_cover(&all, &[]));
        assert!(!tags_cover(&[], &ARABIC_JOINING_FEATURES));
    }

    /// The `NOTOSERIF_AUTOHINT_SHAPING` fixture from `font-test-data` has a GSUB feature
    /// list of exactly `c2sc` and `liga`, no `morx`, and no Arabic characters in its cmap:
    /// a clean negative case for the Arabic shaping gate.
    ///
    /// There is no Arabic *positive* fixture in `font-test-data`, so the positive case is
    /// covered by the `#[ignore]`d machine tests below.
    #[test]
    fn fixture_without_arabic_shaping_is_rejected() {
        let font_ref = skrifa::FontRef::new(font_test_data::NOTOSERIF_AUTOHINT_SHAPING).unwrap();
        assert!(!has_arabic_shaping(&font_ref));

        let feature_list = font_ref.gsub().unwrap().feature_list().unwrap();
        // It covers its own features…
        assert!(feature_list_covers(
            &feature_list,
            &[Tag::new(b"c2sc"), Tag::new(b"liga")]
        ));
        // …but none of the Arabic joining features:
        assert!(!feature_list_covers(
            &feature_list,
            &ARABIC_JOINING_FEATURES
        ));
        // …and an empty requirement is trivially covered:
        assert!(feature_list_covers(&feature_list, &[]));
    }

    /// A font blob truncated after the sfnt header must fail to parse rather than panic
    /// or misreport shaping support; the gate path in [`can_render`] handles these errors
    /// by returning `false`. (fontique's `QueryFont` has no public constructor, so the
    /// error paths are exercised through the same `skrifa` calls `can_render` makes.)
    #[test]
    fn truncated_font_data_is_rejected() {
        let data = font_test_data::NOTOSERIF_AUTOHINT_SHAPING;
        // Too short to even hold the sfnt header: parsing fails outright.
        for len in [0, 4] {
            assert!(skrifa::FontRef::new(&data[..len]).is_err(), "len {len}");
        }
        // A header without its table records parses, but the tables are gone:
        // the gate path must report no shaping support, not panic.
        let font_ref = skrifa::FontRef::new(&data[..12]).unwrap();
        assert!(font_ref.gsub().is_err());
        assert!(!has_arabic_shaping(&font_ref));
    }

    /// A base-plus-mark cluster like `بّ` is resolved through its base character, which
    /// must still pass the Arabic shaping gate: the gate must not make a mark cluster
    /// resolve to a font with only isolated forms.
    #[test]
    #[ignore = "Depends on which fonts are installed on this machine"]
    #[expect(clippy::print_stdout)]
    fn arabic_mark_cluster_resolves_through_base() {
        let provider = SystemFontProvider::new();
        let request = FallbackRequest {
            cluster: "بّ",
            family: &FontFamily::Proportional,
        };
        let Some(insert) = provider.font_for(&request) else {
            panic!("No system font for the Arabic mark cluster");
        };
        println!("mark cluster: {}", insert.name);
        let font_ref = skrifa::FontRef::from_index(insert.data.bytes(), insert.data.index)
            .unwrap_or_else(|err| panic!("{insert:?}: {err:?}"));
        assert!(
            has_arabic_shaping(&font_ref),
            "{insert:?} does not support Arabic joining"
        );
    }

    #[test]
    #[ignore = "Depends on which fonts are installed on this machine"]
    #[expect(clippy::print_stdout)]
    fn finds_system_fonts() {
        let provider = SystemFontProvider::new();
        for (c, script) in [
            ('日', "CJK"),
            ('ع', "Arabic"),
            ('א', "Hebrew"),
            ('क', "Devanagari"),
            ('ก', "Thai"),
            ('⌥', "Symbol"),
        ] {
            let cluster = c.to_string();
            let request = FallbackRequest {
                cluster: &cluster,
                family: &FontFamily::Proportional,
            };
            let Some(insert) = provider.font_for(&request) else {
                panic!("No system font for {script} {c:?}");
            };
            println!("{script} {c:?}: {}", insert.name);
            assert!(insert.name.starts_with("system:"));
            if script == "Arabic" {
                // System Arabic fonts such as Noto Naskh and Amiri carry `init`/`medi`/`fina`.
                // If this fails, the shaping gate in `can_render` is not doing its job.
                let font_ref = skrifa::FontRef::from_index(insert.data.bytes(), insert.data.index)
                    .unwrap_or_else(|err| panic!("{insert:?}: {err:?}"));
                assert!(
                    has_arabic_shaping(&font_ref),
                    "{insert:?} does not support Arabic joining"
                );
            }
        }
    }

    #[test]
    fn simplified_chinese_locales() {
        let is_simplified = |locale| is_simplified_chinese(&Language::parse(locale).unwrap());
        for locale in ["zh", "zh-CN", "zh-Hans", "zh-Hans-CN", "zh-SG"] {
            assert!(is_simplified(locale), "{locale}");
        }
        for locale in ["zh-TW", "zh-HK", "zh-MO", "zh-Hant", "ja", "ko", "de-DE"] {
            assert!(!is_simplified(locale), "{locale}");
        }
    }

    /// A `zh-CN` locale should get a Chinese font for characters shared with Japanese,
    /// even after fontique has looked up Han without a locale.
    #[test]
    #[ignore = "Depends on which fonts are installed on this machine"]
    #[expect(clippy::print_stdout)]
    fn zh_cn_locale_prefers_chinese_font() {
        let provider = SystemFontProvider::new().with_locale(Some("zh-CN"));
        let ja_provider = SystemFontProvider::new().with_locale(Some("ja"));
        let font_name = |provider: &SystemFontProvider, c: char| {
            let cluster = c.to_string();
            let request = FallbackRequest {
                cluster: &cluster,
                family: &FontFamily::Proportional,
            };
            provider.font_for(&request).unwrap().name
        };

        // Each of these does a locale-less Han lookup in fontique:
        for c in ['ก', '한'] {
            font_name(&provider, c);
        }
        let zh = font_name(&provider, '直');
        let ja = font_name(&ja_provider, '直');
        println!("zh-CN: {zh}, ja: {ja}");
        assert_ne!(zh, ja, "zh-CN should not get the Japanese font");
    }

    /// With a non-CJK locale, the Han fallback may be a Japanese font,
    /// which lacks many Simplified and Traditional Chinese characters.
    ///
    /// Other scripts are looked up first, because that changes what fontique returns for Han.
    #[test]
    #[ignore = "Depends on which fonts are installed on this machine"]
    #[expect(clippy::print_stdout)]
    fn finds_chinese_chars_with_non_cjk_locale() {
        let provider = SystemFontProvider::new().with_locale(Some("de-DE"));
        for c in ['ไ', '한', 'م', 'न', '这', '还', '測', '個'] {
            let cluster = c.to_string();
            let request = FallbackRequest {
                cluster: &cluster,
                family: &FontFamily::Proportional,
            };
            let Some(insert) = provider.font_for(&request) else {
                panic!("No system font for {c:?}");
            };
            println!("{c:?}: {}", insert.name);
        }
    }

    #[test]
    #[ignore = "Depends on which fonts are installed on this machine"]
    fn renders_cjk_through_egui_context() {
        let ctx = egui::Context::default();
        let font_id = egui::FontId::proportional(14.0);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            ui.fonts_mut(|fonts| assert!(!fonts.has_glyph(&font_id, '日')));
        });
        output.textures_delta.clear();

        ctx.add_font_provider(Arc::new(SystemFontProvider::new()));
        let mut output = ctx.run_ui(Default::default(), |ui| {
            ui.fonts_mut(|fonts| assert!(fonts.has_glyph(&font_id, '日')));
            assert_eq!(ui.fonts(|fonts| fonts.discovered_fonts().len()), 1);
        });
        output.textures_delta.clear();
    }
}
