//! Load system fonts on demand for characters that the fonts installed in egui lack.
//!
//! [`SystemFontProvider`] is an [`epaint::text::FontProvider`] backed by [`fontique`],
//! which uses the operating system's own font fallback (CoreText, DirectWrite, fontconfig,
//! or Android's `fonts.xml`) to find an installed font for a script and locale.
//!
//! ```no_run
//! # let egui_ctx = egui::Context::default();
//! egui_ctx.add_font_provider(std::sync::Arc::new(egui_system_fonts::SystemFontProvider::new()));
//! ```

use epaint::{
    mutex::Mutex,
    text::{
        FallbackRequest, FontData, FontFamily, FontInsert, FontPriority, FontProvider,
        InsertFontFamily,
    },
};
use fontique::{
    Collection, CollectionOptions, FallbackKey, GenericFamily, Language, QueryFamily, QueryFont,
    QueryStatus, Script, SourceCache, SourceCacheOptions,
};
use poll_promise::Promise;

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
/// Only fonts with outline glyphs are returned. System emoji fonts contain
/// bitmaps or color glyphs, which epaint cannot render yet, so they are skipped.
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
                "egui_system_fonts",
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

        let generic = match family {
            FontFamily::Monospace => GenericFamily::Monospace,
            FontFamily::Proportional | FontFamily::Name(_) => GenericFamily::SansSerif,
        };

        let mut query = collection.query(source_cache);
        if let Some(script) = script_of(base_char) {
            query.set_families([QueryFamily::Generic(generic)]);
            query.set_fallbacks(FallbackKey::new(script, self.locale.as_ref()));
        } else {
            // Punctuation, symbols, etc. belong to no script, so there is no fallback for them.
            // Try the generic families instead.
            query.set_families(
                [
                    generic,
                    GenericFamily::SystemUi,
                    GenericFamily::Math,
                    GenericFamily::Serif,
                    GenericFamily::Monospace,
                ]
                .map(QueryFamily::Generic),
            );
        }

        let mut found = None;
        query.matches_with(|font| {
            if has_outline_for(font, base_char) {
                found = Some(font.clone());
                QueryStatus::Stop
            } else {
                QueryStatus::Continue
            }
        });
        drop(query);
        let font = found?;

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

/// Does the font have an outline glyph for the character?
///
/// False for bitmap and color glyphs, which epaint cannot render yet.
fn has_outline_for(font: &QueryFont, c: char) -> bool {
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
    font_ref.outline_glyphs().get(glyph_id).is_some()
}

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
