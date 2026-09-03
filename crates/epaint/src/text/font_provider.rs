use std::sync::Arc;

use crate::text::{
    FontFamily, FontInsert, FontPriority, InsertFontFamily,
    face_store::{FaceStore, FontFaceKey},
    unicode::is_combining_mark,
};

/// Input to a [`FontProvider`].
pub struct FallbackRequest<'a> {
    /// A grapheme cluster that no installed font can render.
    ///
    /// The returned font must have a glyph for its first character.
    pub cluster: &'a str,

    /// The requested font family.
    pub family: &'a FontFamily,
}

/// Finds a font file for text that no installed font covers, e.g. among the system fonts.
///
/// The returned font is appended to the fallback chain of the requested family,
/// after the fonts in [`FontDefinitions`](crate::text::FontDefinitions), and is then used like any other font:
/// shaped, hinted, and kerned.
///
/// Install one with `egui::Context::add_font_provider` or [`Fonts::with_font_providers`](crate::text::Fonts::with_font_providers).
/// `eframe` installs one on native (behind its `system_fonts` feature).
///
/// Any `Fn(&FallbackRequest<'_>) -> Option<FontInsert>` is a [`FontProvider`].
pub trait FontProvider: Send + Sync {
    /// Find a font with a glyph for the first character of `request.cluster`.
    ///
    /// Called at most once per (family, character), also when returning `None`,
    /// until the [`FontDefinitions`](crate::text::FontDefinitions) change.
    ///
    /// The font is appended to the fallback chain of `request.family`
    /// and of the families in [`FontInsert::families`].
    /// The [`FontPriority`] is ignored: discovered fonts always come after the configured ones
    /// (the fonts in [`FontDefinitions`](crate::text::FontDefinitions)).
    fn font_for(&self, request: &FallbackRequest<'_>) -> Option<FontInsert>;
}

impl<F> FontProvider for F
where
    F: Fn(&FallbackRequest<'_>) -> Option<FontInsert> + Send + Sync,
{
    fn font_for(&self, request: &FallbackRequest<'_>) -> Option<FontInsert> {
        self(request)
    }
}

/// The installed [`FontProvider`]s, and what they have discovered so far.
///
/// Survives atlas clears and `TextOptions` changes,
/// so the providers are asked at most once per (family, char).
#[derive(Default)]
pub(crate) struct FontProviders {
    /// Asked in order; the first font found wins.
    providers: Vec<Arc<dyn FontProvider>>,

    /// Fonts discovered so far, in the order they were found.
    discovered: Vec<FontInsert>,

    /// No provider had a font for these.
    misses: ahash::HashSet<(FontFamily, char)>,
}

impl FontProviders {
    pub fn new(providers: Vec<Arc<dyn FontProvider>>) -> Self {
        Self {
            providers,
            discovered: Vec::new(),
            misses: Default::default(),
        }
    }

    /// Fonts discovered so far, in the order they were found.
    pub fn discovered(&self) -> &[FontInsert] {
        &self.discovered
    }

    /// Parse the discovered fonts into `faces`, e.g. after `faces` was rebuilt.
    pub fn install_discovered(&self, faces: &mut FaceStore) {
        for insert in &self.discovered {
            if let Err(err) = faces.install(&insert.name, &insert.data) {
                log::warn!("Failed to parse discovered font {:?}: {err}", insert.name);
            }
        }
    }

    /// The discovered fonts that belong in the fallback chain of `family`.
    pub fn discovered_keys_for<'a>(
        &'a self,
        family: &'a FontFamily,
        faces: &'a FaceStore,
    ) -> impl Iterator<Item = FontFaceKey> + 'a {
        self.discovered
            .iter()
            .filter(move |insert| insert.families.iter().any(|f| f.family == *family))
            .filter_map(move |insert| faces.key_by_name(&insert.name))
    }

    /// Ask the providers for a font with a glyph for `base_char`, and install it into `faces`.
    ///
    /// A font found this way is a *discovered* font, as opposed to the *configured* ones in
    /// [`FontDefinitions`](crate::text::FontDefinitions).
    ///
    /// Misses are remembered, so each provider is asked at most once per (family, char).
    pub fn discover(
        &mut self,
        faces: &mut FaceStore,
        family: &FontFamily,
        cluster: &str,
        base_char: char,
    ) -> Option<FontFaceKey> {
        if self.providers.is_empty()
            || base_char.is_control()
            || is_combining_mark(base_char)
            || self.misses.contains(&(family.clone(), base_char))
        {
            return None;
        }

        let request = FallbackRequest { cluster, family };

        for provider in &self.providers {
            let Some(mut insert) = provider.font_for(&request) else {
                continue;
            };
            let key = match faces.install(&insert.name, &insert.data) {
                Ok(key) => key,
                Err(err) => {
                    log::warn!(
                        "Failed to parse font {:?} from a font provider: {err}",
                        insert.name
                    );
                    continue;
                }
            };
            let has_glyph = faces
                .get_mut(key)
                .is_some_and(|face| face.glyph_id_resolution(base_char).is_some());
            if !has_glyph {
                log::warn!(
                    "Font {:?} from a font provider has no glyph for {base_char:?}",
                    insert.name
                );
                continue;
            }

            if !insert.families.iter().any(|f| f.family == *family) {
                insert.families.push(InsertFontFamily {
                    family: family.clone(),
                    priority: FontPriority::Lowest,
                });
            }
            self.discovered.push(insert);
            return Some(key);
        }

        self.misses.insert((family.clone(), base_char));
        None
    }
}

#[cfg(feature = "default_fonts")]
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    #[cfg(feature = "monochrome_emoji_fonts")]
    use epaint_default_fonts::{EMOJI_ICON, NOTO_EMOJI_REGULAR};
    use epaint_default_fonts::{HACK_REGULAR, UBUNTU_LIGHT};

    use super::*;
    use crate::{
        Color32, ColorImage,
        mutex::Mutex,
        text::{
            FontData, FontDefinitions, FontId, Fonts, GlyphBitmap, GlyphRasterizer,
            GlyphRasterizerRequest, RasterizedGlyph, TextOptions,
        },
    };

    /// Latin capital letter schwa: in `Ubuntu-Light`, but not in `Hack`.
    const SCHWA: char = 'Ə';

    /// In neither `Hack` nor `Ubuntu-Light`.
    const HANGUL: char = '한';

    const DISCOVERED_FONT: &str = "discovered:Ubuntu-Light";

    /// Only `Hack`, so that e.g. [`SCHWA`] is missing.
    fn hack_only() -> FontDefinitions {
        let mut definitions = FontDefinitions::empty();
        definitions.font_data.insert(
            "Hack".to_owned(),
            Arc::new(FontData::from_static(HACK_REGULAR)),
        );
        definitions
            .families
            .insert(FontFamily::Proportional, vec!["Hack".to_owned()]);
        definitions
            .families
            .insert(FontFamily::Monospace, vec!["Hack".to_owned()]);
        definitions
    }

    /// A provider that records its requests, and returns `Ubuntu-Light` for everything if `provide`.
    fn recording_provider(
        requests: &Arc<Mutex<Vec<(FontFamily, String)>>>,
        provide: bool,
    ) -> Arc<dyn FontProvider> {
        let requests = Arc::clone(requests);
        Arc::new(move |request: &FallbackRequest<'_>| {
            requests
                .lock()
                .push((request.family.clone(), request.cluster.to_owned()));
            provide.then(|| {
                FontInsert::new(
                    DISCOVERED_FONT,
                    FontData::from_static(UBUNTU_LIGHT),
                    vec![InsertFontFamily {
                        family: request.family.clone(),
                        priority: FontPriority::Lowest,
                    }],
                )
            })
        })
    }

    fn color_rasterizer() -> GlyphRasterizer {
        GlyphRasterizer::new(|_: &GlyphRasterizerRequest<'_>| {
            Some(RasterizedGlyph {
                bitmap: GlyphBitmap {
                    image: ColorImage::new([1, 1], vec![Color32::RED]),
                    offset_px: emath::Vec2::ZERO,
                    is_color: true,
                },
                advance_px: 10.0,
            })
        })
    }

    fn fonts_with(provider: Arc<dyn FontProvider>, rasterizer: Option<GlyphRasterizer>) -> Fonts {
        let mut fonts =
            Fonts::new(TextOptions::default(), hack_only()).with_font_providers(vec![provider]);
        fonts.set_glyph_rasterizer(rasterizer);
        fonts
    }

    fn first_glyph(fonts: &mut Fonts, c: char) -> crate::text::Glyph {
        let galley = fonts.with_pixels_per_point(1.0).layout_no_wrap(
            c.to_string(),
            FontId::proportional(14.0),
            Color32::WHITE,
        );
        galley.rows[0].row.glyphs[0]
    }

    #[test]
    fn provider_hit_installs_the_font_once() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mut fonts = fonts_with(recording_provider(&requests, true), None);
        let font_id = FontId::proportional(14.0);

        assert!(fonts.has_glyph(&font_id, SCHWA));
        assert!(fonts.has_glyph(&font_id, SCHWA));
        assert!(fonts.has_glyph(&font_id, 'a'));
        assert_eq!(
            *requests.lock(),
            vec![(FontFamily::Proportional, SCHWA.to_string())]
        );
        assert_eq!(fonts.discovered_fonts().len(), 1);

        let glyph = first_glyph(&mut fonts, SCHWA);
        assert!(!glyph.uv_rect.is_nothing());
        assert!(!glyph.is_color);
        assert_eq!(requests.lock().len(), 1);

        assert_eq!(
            fonts
                .with_pixels_per_point(1.0)
                .characters(&FontFamily::Proportional)
                .get(&SCHWA),
            Some(&vec![DISCOVERED_FONT.to_owned()])
        );
    }

    #[test]
    fn provider_miss_is_remembered_per_family() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mut fonts = fonts_with(recording_provider(&requests, false), None);

        assert!(!fonts.has_glyph(&FontId::proportional(14.0), HANGUL));
        assert!(!fonts.has_glyph(&FontId::proportional(14.0), HANGUL));
        assert!(!fonts.has_glyph(&FontId::monospace(14.0), HANGUL));
        assert_eq!(
            *requests.lock(),
            vec![
                (FontFamily::Proportional, HANGUL.to_string()),
                (FontFamily::Monospace, HANGUL.to_string()),
            ]
        );
        assert!(fonts.discovered_fonts().is_empty());
    }

    #[test]
    fn provider_is_not_asked_for_control_chars_and_combining_marks() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mut fonts = fonts_with(recording_provider(&requests, true), None);
        let font_id = FontId::proportional(14.0);

        fonts.has_glyph(&font_id, '\n');
        fonts.has_glyph(&font_id, '\u{1AB0}'); // COMBINING DOUBLED CIRCUMFLEX ACCENT
        assert!(requests.lock().is_empty());
    }

    #[test]
    fn discovered_fonts_and_misses_survive_a_rebuild() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mut fonts = fonts_with(recording_provider(&requests, true), None);
        let font_id = FontId::proportional(14.0);

        assert!(fonts.has_glyph(&font_id, SCHWA));
        // The provider returns `Ubuntu-Light` for this too, but it has no glyph for it:
        assert!(!fonts.has_glyph(&font_id, HANGUL));
        assert_eq!(requests.lock().len(), 2);
        assert_eq!(fonts.discovered_fonts().len(), 1);

        // Force a rebuild by changing the text options:
        let options = TextOptions {
            font_hinting: !TextOptions::default().font_hinting,
            ..Default::default()
        };
        fonts.begin_pass(options);

        assert!(fonts.has_glyph(&font_id, SCHWA));
        assert!(!fonts.has_glyph(&font_id, HANGUL));
        assert_eq!(
            requests.lock().len(),
            2,
            "The provider should not be asked again"
        );
        assert_eq!(fonts.discovered_fonts().len(), 1);
    }

    #[test]
    fn discovered_font_beats_the_rasterizer() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mut fonts = fonts_with(
            recording_provider(&requests, true),
            Some(color_rasterizer()),
        );

        let glyph = first_glyph(&mut fonts, SCHWA);
        assert!(!glyph.is_color, "Should come from the discovered font");
        assert_eq!(requests.lock().len(), 1);
    }

    #[test]
    #[cfg(feature = "monochrome_emoji_fonts")]
    fn emoji_presentation_prefers_discovered_fonts_over_configured() {
        // Hack and NotoEmoji from the definitions, emoji-icon-font from the provider.
        // Both emoji fonts have 🚀.
        let mut definitions = hack_only();
        definitions.font_data.insert(
            "NotoEmoji-Regular".to_owned(),
            Arc::new(FontData::from_static(NOTO_EMOJI_REGULAR)),
        );
        definitions.families.insert(
            FontFamily::Proportional,
            vec!["Hack".to_owned(), "NotoEmoji-Regular".to_owned()],
        );
        let requests = Arc::new(Mutex::new(Vec::new()));
        let provider = {
            let requests = Arc::clone(&requests);
            Arc::new(move |request: &FallbackRequest<'_>| {
                requests.lock().push(request.cluster.to_owned());
                Some(FontInsert::new(
                    "discovered:emoji-icon-font",
                    FontData::from_static(EMOJI_ICON),
                    vec![],
                ))
            })
        };
        let mut fonts =
            Fonts::new(TextOptions::default(), definitions).with_font_providers(vec![provider]);
        let font_id = FontId::proportional(14.0);
        let mut layout = |text: &str| {
            let galley = fonts.with_pixels_per_point(1.0).layout_no_wrap(
                text.to_owned(),
                font_id.clone(),
                Color32::WHITE,
            );
            galley.rows[0].row.glyphs[0].uv_rect
        };

        // Text presentation: NotoEmoji from the definitions wins, and the provider is not asked.
        let text_presentation = layout("🚀\u{FE0E}");
        assert!(requests.lock().is_empty());

        // Emoji presentation: the provider is asked first, even though NotoEmoji has the glyph.
        let emoji_presentation = layout("🚀");
        assert_eq!(*requests.lock(), ["🚀"]);
        assert_ne!(
            emoji_presentation.min, text_presentation.min,
            "Should be different glyphs, from different fonts"
        );

        // Both are cached:
        layout("🚀\u{FE0E}");
        layout("🚀");
        assert_eq!(requests.lock().len(), 1);
    }

    #[test]
    fn rasterizer_is_used_when_the_provider_misses() {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let mut fonts = fonts_with(
            recording_provider(&requests, false),
            Some(color_rasterizer()),
        );

        let glyph = first_glyph(&mut fonts, HANGUL);
        assert!(glyph.is_color, "Should come from the rasterizer");
        assert_eq!(requests.lock().len(), 1);
    }
}
