use ahash::HashMap;
use core::{mem::size_of, task::Poll};
use egui::{
    ColorImage, decode_animated_image_uri,
    load::{Bytes, BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
};
use image::ImageFormat;
use std::{ffi::OsStr, path::Path, sync::Arc};

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

/// Is there a decoder for this file extension?
///
/// Either one of the enabled built-in formats of the `image` crate,
/// or a format registered by an `image` plugin via [`image::hooks::register_decoding_hook`].
fn is_supported_extension(ext: &str) -> bool {
    // Uses only the enabled image crate features
    ImageFormat::from_extension(ext).is_some_and(|format| format.reading_enabled())
        || image::hooks::decoding_hook_registered(OsStr::new(ext))
}

fn is_supported_uri(uri: &str) -> bool {
    let Some(ext) = Path::new(uri)
        .extension()
        .and_then(|ext| ext.to_str().map(|ext| ext.to_lowercase()))
    else {
        // `true` because if there's no extension, assume that we support it
        return true;
    };

    is_supported_extension(&ext)
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

    // Some servers may return a media type with an optional parameter, e.g. "image/jpeg; charset=utf-8".
    let (mime_type, _) = mime.split_once(';').unwrap_or((mime, ""));
    let mime_type = mime_type.trim().to_ascii_lowercase();

    // Uses only the enabled image crate features
    if ImageFormat::from_mime_type(&mime_type).is_some_and(|format| format.reading_enabled()) {
        return true;
    }

    // `image` plugins register their decoders by file extension, so try to derive one from
    // the mime subtype, e.g. `image/jxl` -> `jxl`, `image/x-foo` -> `foo`, `image/foo+xml` -> `foo`.
    let Some(subtype) = mime_type.strip_prefix("image/") else {
        return false;
    };
    let subtype = subtype.split('+').next().unwrap_or(subtype);
    let candidates = [subtype, subtype.strip_prefix("x-").unwrap_or(subtype)];
    candidates
        .iter()
        .any(|ext| image::hooks::decoding_hook_registered(OsStr::new(ext)))
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
                    // (2)
                    if let Some(mime) = mime
                        && !is_supported_mime(&mime)
                    {
                        return Err(LoadError::FormatNotSupported {
                            detected_format: Some(mime),
                        });
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

    fn has_pending(&self) -> bool {
        self.cache.lock().values().any(|result| result.is_pending())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_support() {
        // No extension: defer to the bytes.
        assert!(is_supported_uri("file://test"));
        assert!(is_supported_uri("https://test"));

        // Never handled by the `image` crate:
        assert!(!is_supported_uri("test.svg"));
        assert!(!is_supported_uri("test.txt"));

        // Only the image formats enabled in the `image` crate are supported.
        // Note that which formats are enabled depends on feature unification,
        // so we can't hard-code e.g. that `png` is supported.
        for (uri, format) in [
            ("https://test.png", ImageFormat::Png),
            ("test.jpeg", ImageFormat::Jpeg),
            ("test.JPG", ImageFormat::Jpeg),
            ("http://test.gif", ImageFormat::Gif),
            ("test.webp", ImageFormat::WebP),
        ] {
            assert_eq!(is_supported_uri(uri), format.reading_enabled(), "{uri}");
        }

        #[cfg(feature = "gif")]
        assert!(is_supported_uri("http://test.gif"));
        #[cfg(feature = "webp")]
        assert!(is_supported_uri("test.webp"));
    }

    #[test]
    fn check_mime_support() {
        assert!(is_supported_mime("application/octet-stream"));
        assert!(!is_supported_mime("text/html"));
        assert!(!is_supported_mime("image/svg+xml"));

        for (mime, format) in [
            ("image/png", ImageFormat::Png),
            ("image/jpeg; charset=utf-8", ImageFormat::Jpeg),
            ("image/gif", ImageFormat::Gif),
            ("image/webp", ImageFormat::WebP),
        ] {
            assert_eq!(is_supported_mime(mime), format.reading_enabled(), "{mime}");
        }
    }

    #[test]
    fn check_plugin_support() {
        // Unique extension, so we don't interfere with other tests (the hooks are global).
        let ext = "eguitestformat";
        assert!(!is_supported_uri(&format!("file://test.{ext}")));
        assert!(!is_supported_mime(&format!("image/{ext}")));
        assert!(!is_supported_mime(&format!("image/x-{ext}")));

        image::hooks::register_decoding_hook(
            ext.into(),
            Box::new(|_| {
                Err(image::ImageError::Unsupported(
                    image::error::ImageFormatHint::Unknown.into(),
                ))
            }),
        );

        assert!(is_supported_uri(&format!("file://test.{ext}")));
        assert!(is_supported_uri(&format!(
            "file://TEST.{}",
            ext.to_uppercase()
        )));
        assert!(is_supported_mime(&format!("image/{ext}")));
        assert!(is_supported_mime(&format!("image/x-{ext}")));
        assert!(is_supported_mime(&format!("image/{ext}; charset=utf-8")));
        assert!(!is_supported_mime(&format!("text/{ext}")));
    }
}
