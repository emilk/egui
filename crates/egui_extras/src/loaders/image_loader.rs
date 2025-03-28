use ahash::HashMap;
use egui::{
    decode_animated_image_uri,
    load::{Bytes, BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
    ColorImage,
};
use image::ImageFormat;
use std::{mem::size_of, path::Path, sync::Arc, task::Poll};

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

fn is_supported_uri(uri: &str) -> bool {
    let Some(ext) = Path::new(uri)
        .extension()
        .and_then(|ext| ext.to_str().map(|ext| ext.to_lowercase()))
    else {
        // `true` because if there's no extension, assume that we support it
        return true;
    };

    // Uses only the enabled image crate features
    ImageFormat::from_extension(ext).is_some_and(|format| format.reading_enabled())
}

fn is_supported_mime(mime: &str) -> bool {
    // some mime types e.g. reflect binary files or mark the content as a download, which
    // may be a valid image or not, in this case, defer the decision on the format guessing
    // or the image crate and return true here
    let mimes_to_defer = [
        "application/octet-stream",
        "application/x-msdownload",
        "application/force-download",
    ];
    for m in &mimes_to_defer {
        // use contains instead of direct equality, as e.g. encoding info might be appended
        if mime.contains(m) {
            return true;
        }
    }

    // Uses only the enabled image crate features
    ImageFormat::from_mime_type(mime).is_some_and(|format| format.reading_enabled())
}

impl ImageLoader for ImageCrateLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str, _: SizeHint) -> ImageLoadResult {
        // three stages of guessing if we support loading the image:
        // 1. URI extension (only done for files)
        // 2. Mime from `BytesPoll::Ready`
        // 3. image::guess_format (used internally by image::load_from_memory)

        // TODO(lucasmerlin): Egui currently changes all URIs for webp and gif files to include
        // the frame index (#0), which breaks if the animated image loader is disabled.
        // We work around this by removing the frame index from the URI here
        let uri = decode_animated_image_uri(uri).map_or(uri, |(uri, _frame_index)| uri);

        // (1)
        if uri.starts_with("file://") && !is_supported_uri(uri) {
            return Err(LoadError::NotSupported);
        }

        #[cfg(not(target_arch = "wasm32"))]
        #[allow(clippy::unnecessary_wraps)] // needed here to match other return types
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
                    let cache = cache.clone();

                    let uri = uri.clone();
                    let bytes = bytes.clone();
                    move || {
                        log::trace!("ImageLoader - started loading {uri:?}");
                        let result = crate::image::load_image_bytes(&bytes)
                            .map(Arc::new)
                            .map_err(|err| err.to_string());
                        log::trace!("ImageLoader - finished loading {uri:?}");
                        let prev = cache.lock().insert(uri, Poll::Ready(result));
                        debug_assert!(
                            matches!(prev, Some(Poll::Pending)),
                            "Expected previous state to be Pending"
                        );

                        ctx.request_repaint();
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
            cache_lock.insert(uri.into(), std::task::Poll::Ready(result.clone()));
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
                    // (2)
                    if let Some(mime) = mime {
                        if !is_supported_mime(&mime) {
                            return Err(LoadError::FormatNotSupported {
                                detected_format: Some(mime),
                            });
                        }
                    }
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
