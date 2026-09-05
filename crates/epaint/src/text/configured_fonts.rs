use crate::text::{FontDefinitions, FontFamily, FontInsert, font_provider::FontProvider};

/// The fonts an app configures up front, as a [`FontProvider`].
///
/// Wraps [`FontDefinitions`]: for each family it hands out the fonts listed there,
/// in priority order. It never discovers anything on demand,
/// so the same text looks the same on every machine.
///
/// This is always the first provider, so configured fonts win over discovered ones.
#[derive(Clone, Debug)]
pub struct ConfiguredFonts {
    definitions: FontDefinitions,
}

impl ConfiguredFonts {
    pub fn new(definitions: FontDefinitions) -> Self {
        Self { definitions }
    }

    #[inline]
    pub fn definitions(&self) -> &FontDefinitions {
        &self.definitions
    }
}

impl FontProvider for ConfiguredFonts {
    fn fonts_for_family(&self, family: &FontFamily) -> Vec<FontInsert> {
        let Some(font_names) = self.definitions.families.get(family) else {
            log::warn!("FontFamily::{family:?} is not bound to any fonts");
            return Vec::new();
        };

        font_names
            .iter()
            .map(|name| {
                let data = self.definitions.font_data.get(name).unwrap_or_else(|| {
                    let available: Vec<&String> = self.definitions.font_data.keys().collect();
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
