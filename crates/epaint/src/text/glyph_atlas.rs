use std::sync::Arc;

use ahash::HashMapExt;
use cosmic_text::{FontSystem, PhysicalGlyph};
use emath::vec2;

use crate::{mutex::Mutex, TextureAtlas};

use super::font::UvRect;

pub(super) struct GlyphAtlas {
    swash_cache: cosmic_text::SwashCache,
    // TODO: just pass this in from Fonts
    atlas: Arc<Mutex<TextureAtlas>>,
    rendered_glyphs: ahash::HashMap<cosmic_text::CacheKey, Option<UvRect>>,
}

impl GlyphAtlas {
    pub(super) fn new(atlas: Arc<Mutex<TextureAtlas>>) -> Self {
        Self {
            swash_cache: cosmic_text::SwashCache::new(),
            atlas,
            rendered_glyphs: ahash::HashMap::new(),
        }
    }

    pub fn render_glyph(
        &mut self,
        font_system: &mut FontSystem,
        physical: &PhysicalGlyph,
        pixels_per_point: f32,
    ) -> Option<UvRect> {
        if let Some(rendered) = self.rendered_glyphs.get(&physical.cache_key) {
            return *rendered;
        }

        // TODO(valadaptive): can this be optimized?
        let image = self
            .swash_cache
            .get_image_uncached(font_system, physical.cache_key);

        let rendered = if let Some(image) = image {
            let mut atlas = self.atlas.lock();
            let (allocated_pos, font_image) = atlas.allocate((
                image.placement.width as usize,
                image.placement.height as usize,
            ));

            match image.content {
                cosmic_text::SwashContent::Mask => {
                    let mut i = 0;
                    for y in 0..image.placement.height as usize {
                        for x in 0..image.placement.width as usize {
                            font_image[(x + allocated_pos.0, y + allocated_pos.1)] =
                                image.data[i] as f32 / 255.0;
                            i += 1;
                        }
                    }
                }
                // TODO(valadaptive): color emoji support
                cosmic_text::SwashContent::SubpixelMask => todo!(),
                cosmic_text::SwashContent::Color => todo!(),
            }

            let uv_rect = UvRect {
                offset: vec2(image.placement.left as f32, -image.placement.top as f32)
                    / pixels_per_point,
                size: vec2(image.placement.width as f32, image.placement.height as f32)
                    / pixels_per_point,
                min: [allocated_pos.0 as u16, allocated_pos.1 as u16],
                max: [
                    allocated_pos.0 as u16 + image.placement.width as u16,
                    allocated_pos.1 as u16 + image.placement.height as u16,
                ],
            };

            Some(uv_rect)
        } else {
            None
        };

        self.rendered_glyphs.insert(physical.cache_key, rendered);

        rendered
    }
}
