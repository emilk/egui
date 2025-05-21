use std::borrow::Cow;

use super::{
    BytesLoader as _, Context, HashMap, ImagePoll, Mutex, SizeHint, SizedTexture, TextureHandle,
    TextureLoadResult, TextureLoader, TextureOptions, TexturePoll,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key<'a> {
    uri: Cow<'a, str>,
    texture_options: TextureOptions,
    svg_size_hint: Option<SizeHint>,
}

#[derive(Default)]
pub struct DefaultTextureLoader {
    cache: Mutex<HashMap<Key<'static>, TextureHandle>>,
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
        let svg_size_hint = if uri.ends_with(".svg") {
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
        if let Some(handle) = cache.get(&Key {
            uri: Cow::Borrowed(uri),
            texture_options,
            svg_size_hint,
        }) {
            let texture = SizedTexture::from_handle(handle);
            Ok(TexturePoll::Ready { texture })
        } else {
            match ctx.try_load_image(uri, size_hint)? {
                ImagePoll::Pending { size } => Ok(TexturePoll::Pending { size }),
                ImagePoll::Ready { image } => {
                    let handle = ctx.load_texture(uri, image, texture_options);
                    let texture = SizedTexture::from_handle(&handle);
                    cache.insert(
                        Key {
                            uri: Cow::Owned(uri.to_owned()),
                            texture_options,
                            svg_size_hint,
                        },
                        handle,
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

    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .values()
            .map(|texture| texture.byte_size())
            .sum()
    }
}
