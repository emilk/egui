use ahash::HashMap;
use egui::{
    load::{BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
    ColorImage,
};
use std::{mem::size_of, path::Path, sync::Arc, task::Poll, thread};

type Entry = Poll<Result<Arc<ColorImage>, String>>;

#[derive(Default)]
pub struct ImageCrateLoader {
    cache: Arc<Mutex<HashMap<String, Entry>>>,
}

impl ImageCrateLoader {
    pub const ID: &'static str = egui::generate_loader_id!(ImageCrateLoader);
}

fn is_supported_uri(uri: &str) -> bool {
    // TODO(emilk): use https://github.com/image-rs/image/pull/2038 when new `image` crate is released.
    let Some(ext) = Path::new(uri).extension().and_then(|ext| ext.to_str()) else {
        // `true` because if there's no extension, assume that we support it
        return true;
    };

    ext != "svg"
}

fn is_unsupported_mime(mime: &str) -> bool {
    // TODO(emilk): use https://github.com/image-rs/image/pull/2038 when new `image` crate is released.
    mime.contains("svg")
}

impl ImageLoader for ImageCrateLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str, _: SizeHint) -> ImageLoadResult {
        // three stages of guessing if we support loading the image:
        // 1. URI extension
        // 2. Mime from `BytesPoll::Ready`
        // 3. image::guess_format

        // (1)
        if !is_supported_uri(uri) {
            return Err(LoadError::NotSupported);
        }

        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(uri).cloned() {
            match entry {
                Poll::Ready(Ok(image)) => Ok(ImagePoll::Ready { image }),
                Poll::Ready(Err(err)) => Err(LoadError::Loading(err)),
                Poll::Pending => Ok(ImagePoll::Pending { size: None }),
            }
        } else {
            match ctx.try_load_bytes(uri) {
                Ok(BytesPoll::Ready { bytes, mime, .. }) => {
                    // (2 and 3)
                    if mime.as_deref().is_some_and(is_unsupported_mime)
                        || image::guess_format(&bytes).is_err()
                    {
                        return Err(LoadError::NotSupported);
                    }

                    let uri = uri.to_owned();
                    cache.insert(uri.clone(), Poll::Pending);
                    drop(cache);

                    // Do the image parsing on a bg thread
                    thread::Builder::new()
                        .name(format!("egui_extras::ImageLoader::load({uri:?})"))
                        .spawn({
                            let ctx = ctx.clone();
                            let cache = self.cache.clone();

                            let uri = uri.clone();
                            move || {
                                log::trace!("ImageLoader - started loading {uri:?}");
                                let result = crate::image::load_image_bytes(&bytes).map(Arc::new);
                                log::trace!("ImageLoader - finished loading {uri:?}");
                                let prev = cache.lock().insert(uri, Poll::Ready(result));
                                assert!(matches!(prev, Some(Poll::Pending)));

                                ctx.request_repaint();
                            }
                        })
                        .expect("failed to spawn thread");

                    Ok(ImagePoll::Pending { size: None })
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_support() {
        assert!(is_supported_uri("https://test.png"));
        assert!(is_supported_uri("test.jpeg"));
        assert!(is_supported_uri("http://test.gif"));
        assert!(is_supported_uri("test.webp"));
        assert!(is_supported_uri("file://test"));
        assert!(!is_supported_uri("test.svg"));
    }
}
