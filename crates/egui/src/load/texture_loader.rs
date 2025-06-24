use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

use emath::Vec2;

use super::{
    BytesLoader as _, Context, HashMap, ImagePoll, Mutex, SizeHint, SizedTexture, TextureHandle,
    TextureLoadResult, TextureLoader, TextureOptions, TexturePoll,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PrimaryKey {
    uri: String,
    texture_options: TextureOptions,
}

/// SVG:s might have several different sizes loaded
type Bucket = HashMap<Option<SizeHint>, Entry>;

struct Entry {
    last_used: AtomicU64,

    /// Size of the original SVG, if any, or the texel size of the image if not an SVG.
    source_size: Vec2,

    handle: TextureHandle,
}

#[derive(Default)]
pub struct DefaultTextureLoader {
    pass_index: AtomicU64,
    cache: Mutex<HashMap<PrimaryKey, Bucket>>,
}

impl TextureLoader for DefaultTextureLoader {
    fn id(&self) -> &'static str {
        crate::generate_loader_id!(DefaultTextureLoader)
    }

    fn load(
        &self,
        ctx: &Context,
        uri: &str,
        texture_options: TextureOptions,
        size_hint: SizeHint,
    ) -> TextureLoadResult {
        let svg_size_hint = if is_svg(uri) {
            // For SVGs it's important that we render at the desired size,
            // or we might get a blurry image when we scale it up.
            // So we make the size hint a part of the cache key.
            // This might lead to a lot of extra entries for the same SVG file,
            // which is potentially wasteful of RAM, but better that than blurry images.
            Some(size_hint)
        } else {
            // For other images we just use one cache value, no matter what the size we render at.
            None
        };

        let mut cache = self.cache.lock();
        let bucket = cache
            .entry(PrimaryKey {
                uri: uri.to_owned(),
                texture_options,
            })
            .or_default();

        if let Some(texture) = bucket.get(&svg_size_hint) {
            texture
                .last_used
                .store(self.pass_index.load(Relaxed), Relaxed);
            let texture = SizedTexture::new(texture.handle.id(), texture.source_size);
            Ok(TexturePoll::Ready { texture })
        } else {
            match ctx.try_load_image(uri, size_hint)? {
                ImagePoll::Pending { size } => Ok(TexturePoll::Pending { size }),
                ImagePoll::Ready { image } => {
                    let source_size = image.source_size;
                    let handle = ctx.load_texture(uri, image, texture_options);
                    let texture = SizedTexture::new(handle.id(), source_size);
                    bucket.insert(
                        svg_size_hint,
                        Entry {
                            last_used: AtomicU64::new(self.pass_index.load(Relaxed)),
                            source_size,
                            handle,
                        },
                    );
                    let reduce_texture_memory = ctx.options(|o| o.reduce_texture_memory);
                    if reduce_texture_memory {
                        let loaders = ctx.loaders();
                        loaders.include.forget(uri);
                        for loader in loaders.bytes.lock().iter().rev() {
                            loader.forget(uri);
                        }
                        for loader in loaders.image.lock().iter().rev() {
                            loader.forget(uri);
                        }
                    }
                    Ok(TexturePoll::Ready { texture })
                }
            }
        }
    }

    fn forget(&self, uri: &str) {
        #[cfg(feature = "log")]
        log::trace!("forget {uri:?}");

        self.cache.lock().retain(|key, _value| key.uri != uri);
    }

    fn forget_all(&self) {
        #[cfg(feature = "log")]
        log::trace!("forget all");

        self.cache.lock().clear();
    }

    fn end_pass(&self, pass_index: u64) {
        self.pass_index.store(pass_index, Relaxed);
        let mut cache = self.cache.lock();
        cache.retain(|_key, bucket| {
            if 2 <= bucket.len() {
                // There are multiple textures of the same URI (e.g. SVGs of different scales).
                // This could be because someone has an SVG in a resizable container,
                // and so we get a lot of different sizes of it.
                // This could wast VRAM, so we remove the ones that are not used in this frame.
                bucket.retain(|_, texture| pass_index <= texture.last_used.load(Relaxed) + 1);
            }
            !bucket.is_empty()
        });
    }

    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .values()
            .map(|bucket| {
                bucket
                    .values()
                    .map(|texture| texture.handle.byte_size())
                    .sum::<usize>()
            })
            .sum()
    }
}

fn is_svg(uri: &str) -> bool {
    uri.ends_with(".svg")
}
