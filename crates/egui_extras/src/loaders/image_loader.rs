use core::{mem::size_of, task::Poll};
use egui::{
    ColorImage, decode_animated_image_uri,
    load::{BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
};
use image::ImageFormat;
use std::{ffi::OsStr, path::Path, sync::Arc};

use super::background_decode::DecodeCache;

pub struct ImageCrateLoader {
    cache: DecodeCache<Arc<ColorImage>>,
}

impl Default for ImageCrateLoader {
    fn default() -> Self {
        Self {
            cache: DecodeCache::new("ImageLoader"),
        }
    }
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

        let entry = match self.cache.get(uri) {
            Some(entry) => entry,
            None => match ctx.try_load_bytes(uri) {
                Ok(BytesPoll::Ready { bytes, mime, .. }) => {
                    // (2)
                    if let Some(mime) = mime
                        && !is_supported_mime(&mime)
                    {
                        return Err(LoadError::FormatNotSupported {
                            detected_format: Some(mime),
                        });
                    }
                    self.cache.decode(ctx, uri, &bytes, |bytes| {
                        crate::image::load_image_bytes(bytes)
                            .map(Arc::new)
                            .map_err(|err| err.to_string())
                    })
                }
                Ok(BytesPoll::Pending { size }) => return Ok(ImagePoll::Pending { size }),
                Err(err) => return Err(err),
            },
        };

        match entry {
            Poll::Ready(Ok(image)) => Ok(ImagePoll::Ready { image }),
            Poll::Ready(Err(err)) => Err(LoadError::Loading(err)),
            Poll::Pending => Ok(ImagePoll::Pending { size: None }),
        }
    }

    fn forget(&self, uri: &str) {
        self.cache.forget(uri);
    }

    fn forget_all(&self) {
        self.cache.forget_all();
    }

    fn byte_size(&self) -> usize {
        self.cache
            .byte_size(|image| image.pixels.len() * size_of::<egui::Color32>())
    }

    fn has_pending(&self) -> bool {
        self.cache.has_pending()
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
