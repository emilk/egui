use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    sync::Arc,
};

use ahash::AHashMap;
use fontdue::{
    layout::{CoordinateSystem, GlyphPosition, GlyphRasterConfig, LayoutSettings},
    Font, FontSettings, Metrics,
};
use parking_lot::Mutex;

use crate::math::{vec2, Vec2};

use super::texture_atlas::{Texture, TextureAtlas};

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TextStyle {
    Body,
    Button,
    Heading,
    Monospace,
}

#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FontFamily {
    Monospace,
    VariableWidth,
}

impl FontFamily {
    /// Used as index for the font vector. The fonts need to be inserted in this order!
    pub fn font_index(&self) -> usize {
        match self {
            FontFamily::Monospace => 0,
            FontFamily::VariableWidth => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontDefinition {
    pub family: FontFamily,
    pub scale_in_points: f32,
}

/// Configured the typefaces that are used. This is a configuration and is not supposed to be changed while rendering.
#[derive(Clone, Debug, PartialEq)]
pub struct FontConfiguration {
    /// The dpi scale factor. Needed to get pixel perfect fonts.
    pub pixels_per_point: f32,
    pub definitions: BTreeMap<TextStyle, FontDefinition>,
}

impl Default for FontConfiguration {
    fn default() -> Self {
        Self::with_pixels_per_point(f32::NAN) // must be set later
    }
}

impl FontConfiguration {
    pub fn with_pixels_per_point(pixels_per_point: f32) -> Self {
        let mut definitions = BTreeMap::new();
        definitions.insert(
            TextStyle::Body,
            FontDefinition {
                family: FontFamily::VariableWidth,
                scale_in_points: 14.0,
            },
        );
        definitions.insert(
            TextStyle::Button,
            FontDefinition {
                family: FontFamily::VariableWidth,
                scale_in_points: 16.0,
            },
        );
        definitions.insert(
            TextStyle::Heading,
            FontDefinition {
                family: FontFamily::VariableWidth,
                scale_in_points: 24.0,
            },
        );
        definitions.insert(
            TextStyle::Monospace,
            FontDefinition {
                family: FontFamily::Monospace,
                scale_in_points: 13.0,
            },
        );
        Self {
            pixels_per_point,
            definitions,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UvRect {
    /// The size of the element in points.
    pub size: Vec2,

    /// Top left corner UV in texture.
    pub min: (u16, u16),

    /// Bottom right corner (exclusive).
    pub max: (u16, u16),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GlyphInfo {
    /// Glyph metrics.
    pub metrics: Metrics,

    /// Texture coordinates.
    pub uv_rect: UvRect,
}

/// Glyph layout information.
#[derive(Clone, Debug, Default)]
pub struct GlyphLayout {
    pub size: Vec2,
    pub glyph_positions: Vec<GlyphPosition>,
}

impl GlyphLayout {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            size: vec2(0.0, 0.0),
            glyph_positions: Vec::with_capacity(capacity),
        }
    }
}

/// Font renderer.
pub struct Fonts {
    configuration: FontConfiguration,
    fonts: Vec<Font>,
    layout_engine: fontdue::layout::Layout,
    glyph_infos: AHashMap<GlyphRasterConfig, GlyphInfo>,
    atlas: Arc<Mutex<TextureAtlas>>,
    /// Copy of the texture in the texture atlas.
    /// This is so we can return a reference to it (the texture atlas is behind a lock).
    buffered_texture: Mutex<Arc<Texture>>,
    /// Precalculated heights for the TextStyles.
    heights: BTreeMap<TextStyle, f32>,
    /// Precalculated heights for the TextStyles.
    line_spacings: BTreeMap<TextStyle, f32>,
}

impl Default for Fonts {
    fn default() -> Self {
        Self {
            configuration: Default::default(),
            fonts: Default::default(),
            layout_engine: fontdue::layout::Layout::new(CoordinateSystem::PositiveYDown),
            glyph_infos: Default::default(),
            atlas: Default::default(),
            buffered_texture: Default::default(),
            heights: Default::default(),
            line_spacings: Default::default(),
        }
    }
}

impl Fonts {
    pub fn from_definitions(configuration: FontConfiguration) -> Fonts {
        let mut fonts = Fonts::default();
        fonts.set_configuration(configuration);
        fonts
    }

    pub fn configuration(&self) -> &FontConfiguration {
        &self.configuration
    }

    pub fn set_configuration(&mut self, configuration: FontConfiguration) {
        if self.configuration == configuration {
            return;
        }

        self.atlas = Arc::new(Mutex::new(TextureAtlas::new(512, 512)));

        // Make the top left pixel fully white, since it's used for the UI rendering.
        let pos = self.atlas.lock().allocate((1, 1));
        self.atlas.lock().texture_mut()[pos] = 255;
        debug_assert_eq!(pos, (0, 0));

        // FontFamily::Monospace (Use 13 for this. NOTHING ELSE).
        let monospace_typeface_data: &[u8] = include_bytes!("../../fonts/ProggyClean.ttf");
        // FontFamily::VariableWidth.
        // FIXME: https://github.com/mooman219/fontdue/issues/38
        // FIXME: https://github.com/RazrFalcon/ttf-parser/issues/43
        let variable_typeface_data: &[u8] = include_bytes!("../../fonts/Comfortaa-Regular.ttf");
        //let variable_typeface_data: &[u8] = include_bytes!("../../fonts/Roboto-Regular.ttf");

        // Fonts need to be added in the same order as defined in the `FontFamily.font_index()` method.
        self.fonts.push(
            Font::from_bytes(monospace_typeface_data, FontSettings::default())
                .expect("error constructing Font"),
        );
        self.fonts.push(
            Font::from_bytes(variable_typeface_data, FontSettings::default())
                .expect("error constructing Font"),
        );

        self.configuration = configuration;

        let pixel_per_points = self.configuration.pixels_per_point;
        for (text_style, definition) in self.configuration.definitions.clone().iter() {
            let font_index = definition.family.font_index();
            let scale_in_pixels = definition.scale_in_points * pixel_per_points;

            // Preload the printable ASCII characters [33, 126] (which excludes control codes):
            const FIRST_ASCII: usize = 33; // !
            const LAST_ASCII: usize = 126; // ~

            for u in FIRST_ASCII..=LAST_ASCII {
                let c = std::char::from_u32(u as u32)
                    .unwrap_or_else(|| panic!("can't create char from u32: {}", u));
                let key = GlyphRasterConfig {
                    c,
                    px: scale_in_pixels,
                    font_index,
                };
                self.glyph_info(&key);
            }

            // Precalculate the line spacings and heights (in points)
            let px = scale_in_pixels;
            let font = &self.fonts[font_index];
            let line_spacing = font
                .horizontal_line_metrics(px)
                .unwrap_or_else(|| panic!("font doesn't seem to support horizontal text layout"))
                .new_line_size;
            self.line_spacings
                .insert(*text_style, line_spacing / pixel_per_points);
            self.heights
                .insert(*text_style, scale_in_pixels / pixel_per_points);
        }

        // Make sure we seed the texture version with something unique based on the default characters:
        let mut atlas = self.atlas.lock();
        let texture = atlas.texture_mut();
        let mut hasher = ahash::AHasher::default();
        texture.pixels.hash(&mut hasher);
        texture.version = hasher.finish();

        self.buffered_texture = Default::default();
    }

    /// Returns the `GlyphInfo` for the given `GlyphRasterConfig` key. Allocates a new Glyph if necessary.
    pub fn glyph_info(&mut self, grc: &GlyphRasterConfig) -> GlyphInfo {
        if let Some(glyph_info) = self.glyph_infos.get(grc) {
            return *glyph_info;
        }

        let glyph_info = self.allocate_glyph(grc);

        let glyph_info =
            glyph_info.unwrap_or_else(|| panic!("couldn't render glyph: {:#?}", grc.c));
        self.glyph_infos.insert(*grc, glyph_info);
        glyph_info
    }

    pub fn texture(&self) -> Arc<Texture> {
        let atlas = self.atlas.lock();
        let mut buffered_texture = self.buffered_texture.lock();
        if buffered_texture.version != atlas.texture().version {
            *buffered_texture = Arc::new(atlas.texture().clone());
        }

        buffered_texture.clone()
    }

    /// Typeset the given text onto one line. Ignores hard wraps. Returns the dimension of the line.
    pub fn layout_single_line(&mut self, style: TextStyle, text: &str) -> GlyphLayout {
        let settings = LayoutSettings {
            wrap_hard_breaks: false,
            ..Default::default()
        };

        self.layout(style, text, &settings)
    }

    // FIXME: https://github.com/mooman219/fontdue/issues/39
    /// Typeset the given text onto multiple lines.
    pub fn layout_multiline(
        &mut self,
        style: TextStyle,
        text: &str,
        max_width_in_points: Option<f32>,
    ) -> GlyphLayout {
        let settings = LayoutSettings {
            max_width: max_width_in_points,
            wrap_hard_breaks: true,
            ..Default::default()
        };

        self.layout(style, text, &settings)
    }

    fn layout(&mut self, style: TextStyle, text: &str, settings: &LayoutSettings) -> GlyphLayout {
        let mut layout = GlyphLayout::with_capacity(text.len());

        if text.is_empty() {
            return layout;
        }

        let height = self.heights[&style];
        let font_index = self.configuration.definitions[&style].family.font_index();

        let text_style = fontdue::layout::TextStyle {
            text,
            px: height,
            font_index,
        };

        self.layout_engine.layout_horizontal(
            &self.fonts,
            &[&text_style],
            settings,
            &mut layout.glyph_positions,
        );

        // This assumes horizontal layout rendered from left to right.
        let GlyphPosition {
            x: first_x,
            y: first_y,
            height: first_height,
            ..
        } = *layout.glyph_positions.first().unwrap();
        let GlyphPosition {
            x: last_x,
            y: last_y,
            width: last_width,
            ..
        } = *layout.glyph_positions.last().unwrap();

        let width = (last_x + last_width as f32) - first_x;
        let height = last_y - (first_y - first_height as f32);
        layout.size = vec2(width, height);

        layout
    }

    fn allocate_glyph(&mut self, grc: &GlyphRasterConfig) -> Option<GlyphInfo> {
        let font = &self.fonts[grc.font_index];
        let (metrics, glyph_data) = font.rasterize(grc.c, grc.px);

        if glyph_data.is_empty() {
            return None;
        }

        let mut atlas = self.atlas.lock();
        let glyph_pos = atlas.allocate((metrics.width, metrics.height));
        let texture = atlas.texture_mut();

        for (i, v) in glyph_data.iter().enumerate() {
            if *v > 0 {
                let px = glyph_pos.0 + (i % metrics.width);
                let py = glyph_pos.1 + (i / metrics.width);
                texture[(px, py)] = *v;
            }
        }

        let uv_rect = UvRect {
            size: vec2(metrics.width as f32, metrics.height as f32),
            min: (glyph_pos.0 as u16, glyph_pos.1 as u16),
            max: (
                (glyph_pos.0 + metrics.width) as u16,
                (glyph_pos.1 + metrics.height) as u16,
            ),
        };

        Some(GlyphInfo { metrics, uv_rect })
    }

    /// Returns the font height in points of the given `TextStyle`.
    pub fn text_style_height(&self, text_style: TextStyle) -> f32 {
        self.heights[&text_style]
    }

    /// Returns the line spacing in points of the given `TextStyle`.
    pub fn text_style_line_spacing(&self, text_style: TextStyle) -> f32 {
        self.line_spacings[&text_style]
    }
}
