use ahash::HashMap;
use core::{mem::size_of, task::Poll};
use egui::{
    ColorImage, decode_animated_image_uri,
    load::{Bytes, BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use std::thread;

type Entry = Poll<Result<Arc<ColorImage>, String>>;

#[derive(Default)]
pub struct ImageCrateLoader {
    cache: Arc<Mutex<HashMap<String, Entry>>>,
}

impl ImageCrateLoader {
    pub const ID: &'static str = egui::generate_loader_id!(ImageCrateLoader);
}

impl ImageLoader for ImageCrateLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str, _: SizeHint) -> ImageLoadResult {
        // TODO(lucasmerlin): Egui currently changes all URIs for webp and gif files to include
        // the frame index (#0), which breaks if the animated image loader is disabled.
        // We work around this by removing the frame index from the URI here
        let uri = decode_animated_image_uri(uri).map_or(uri, |(uri, _frame_index)| uri);

        #[cfg(not(target_arch = "wasm32"))]
        #[expect(clippy::unnecessary_wraps)] // needed here to match other return types
        fn load_image(
            ctx: &egui::Context,
            uri: &str,
            cache: &Arc<Mutex<HashMap<String, Entry>>>,
            bytes: &Bytes,
        ) -> ImageLoadResult {
            let uri = uri.to_owned();
            cache.lock().insert(uri.clone(), Poll::Pending);

            // Do the image parsing on a bg thread
            thread::Builder::new()
                .name(format!("egui_extras::ImageLoader::load({uri:?})"))
                .spawn({
                    let ctx = ctx.clone();
                    let cache = Arc::clone(cache);

                    let uri = uri.clone();
                    let bytes = bytes.clone();
                    move || {
                        log::trace!("ImageLoader - started loading {uri:?}");
                        let result = crate::image::load_image_bytes(&bytes)
                            .map(Arc::new)
                            .map_err(|err| err.to_string());
                        let repaint = {
                            let mut cache = cache.lock();

                            if let std::collections::hash_map::Entry::Occupied(mut entry) = cache.entry(uri.clone()) {
                                let entry = entry.get_mut();
                                *entry = Poll::Ready(result);
                                log::trace!("ImageLoader - finished loading {uri:?}");
                                true
                            } else {
                                log::trace!("ImageLoader - canceled loading {uri:?}\nNote: This can happen if `forget_image` is called while the image is still loading.");
                                false
                            }
                        };
                        // We may not lock Context while the cache lock is held, since this can
                        // deadlock.
                        // Example deadlock scenario:
                        // - loader thread: lock cache
                        // - main thread: lock ctx (e.g. in `Context::has_pending_images`)
                        // - loader thread: try to lock ctx (in `request_repaint`)
                        // - main thread: try to lock cache (from `Self::has_pending`)
                        if repaint {
                            ctx.request_repaint();
                        }
                    }
                })
                .expect("failed to spawn thread");

            Ok(ImagePoll::Pending { size: None })
        }

        #[cfg(target_arch = "wasm32")]
        fn load_image(
            _ctx: &egui::Context,
            uri: &str,
            cache: &Arc<Mutex<HashMap<String, Entry>>>,
            bytes: &Bytes,
        ) -> ImageLoadResult {
            let mut cache_lock = cache.lock();
            log::trace!("started loading {uri:?}");
            let result = crate::image::load_image_bytes(bytes)
                .map(Arc::new)
                .map_err(|err| err.to_string());
            log::trace!("finished loading {uri:?}");
            cache_lock.insert(uri.into(), core::task::Poll::Ready(result.clone()));
            match result {
                Ok(image) => Ok(ImagePoll::Ready { image }),
                Err(err) => Err(LoadError::Loading(err)),
            }
        }

        let entry = self.cache.lock().get(uri).cloned();
        if let Some(entry) = entry {
            match entry {
                Poll::Ready(Ok(image)) => Ok(ImagePoll::Ready { image }),
                Poll::Ready(Err(err)) => Err(LoadError::Loading(err)),
                Poll::Pending => Ok(ImagePoll::Pending { size: None }),
            }
        } else {
            match ctx.try_load_bytes(uri) {
                Ok(BytesPoll::Ready { bytes, mime, .. }) => {
                    load_image(ctx, uri, &self.cache, &bytes)
                }
                Ok(BytesPoll::Pending { size }) => Ok(ImagePoll::Pending { size }),
                Err(err) => Err(err),
            }
        }
    }

    fn forget(&self, uri: &str) {
        let _ = self.cache.lock().remove(uri);
    }

    fn forget_all(&self) {
        self.cache.lock().clear();
    }

    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .values()
            .map(|result| match result {
                Poll::Ready(Ok(image)) => image.pixels.len() * size_of::<egui::Color32>(),
                Poll::Ready(Err(err)) => err.len(),
                Poll::Pending => 0,
            })
            .sum()
    }

    fn has_pending(&self) -> bool {
        self.cache.lock().values().any(|result| result.is_pending())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_support() {
        assert!(is_supported_uri("https://test.png"));
        assert!(is_supported_uri("test.jpeg"));
        assert!(is_supported_uri("http://test.gif"));
        assert!(is_supported_uri("file://test"));
        assert!(!is_supported_uri("test.svg"));
    }
}
