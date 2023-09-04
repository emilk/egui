use egui::{
    ahash::HashMap,
    load::{
        BytesLoadResult, BytesLoader, BytesPoll, ImageLoadResult, ImageLoader, ImagePoll,
        LoadError, SizeHint,
    },
    mutex::Mutex,
    ColorImage,
};
use std::{path::Path, sync::Arc, task::Poll, thread};

pub fn install(ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    ctx.add_bytes_loader(Arc::new(self::file_loader::FileLoader::default()));

    #[cfg(feature = "ehttp")]
    ctx.add_bytes_loader(Arc::new(self::ehttp_loader::EhttpLoader::default()));

    #[cfg(feature = "image")]
    ctx.add_image_loader(Arc::new(self::image_loader::ImageCrateLoader::default()));

    #[cfg(feature = "svg")]
    ctx.add_image_loader(Arc::new(self::svg_loader::SvgLoader::default()));

    #[cfg(all(
        target_arch = "wasm32",
        not(feature = "ehttp"),
        not(feature = "image"),
        not(feature = "svg")
    ))]
    crate::log_warn!("`loaders::install` was called, but no loaders are enabled");
}

#[cfg(not(target_arch = "wasm32"))]
mod file_loader {
    use super::*;

    #[derive(Default)]
    pub struct FileLoader {
        /// Cache for loaded files
        cache: Arc<Mutex<HashMap<String, Poll<Result<Arc<[u8]>, String>>>>>,
    }

    const PROTOCOL: &str = "file://";

    impl BytesLoader for FileLoader {
        fn load(&self, ctx: &egui::Context, uri: &str) -> BytesLoadResult {
            // File loader only supports the `file` protocol.
            let Some(path) = uri.strip_prefix(PROTOCOL) else {
                return Err(LoadError::NotSupported);
            };

            let mut cache = self.cache.lock();
            if let Some(entry) = cache.get(path).cloned() {
                // `path` has either begun loading, is loaded, or has failed to load.
                match entry {
                    Poll::Ready(Ok(bytes)) => Ok(BytesPoll::Ready { size: None, bytes }),
                    Poll::Ready(Err(err)) => Err(LoadError::Custom(err)),
                    Poll::Pending => Ok(BytesPoll::Pending { size: None }),
                }
            } else {
                // We need to load the file at `path`.

                // Set the file to `pending` until we finish loading it.
                let path = path.to_owned();
                cache.insert(path.clone(), Poll::Pending);
                drop(cache);

                // Spawn a thread to read the file, so that we don't block the render for too long.
                thread::spawn({
                    let ctx = ctx.clone();
                    let cache = self.cache.clone();
                    move || {
                        let result = match std::fs::read(&path) {
                            Ok(bytes) => Ok(bytes.into()),
                            Err(err) => Err(err.to_string()),
                        };
                        let prev = cache.lock().insert(path, Poll::Ready(result));
                        assert!(matches!(prev, Some(Poll::Pending)));
                        ctx.request_repaint();
                    }
                });

                Ok(BytesPoll::Pending { size: None })
            }
        }
    }
}

#[cfg(feature = "ehttp")]
mod ehttp_loader {
    use super::*;

    #[derive(Default)]
    pub struct EhttpLoader {
        cache: Arc<Mutex<HashMap<String, Poll<Result<Arc<[u8]>, String>>>>>,
    }

    const PROTOCOLS: &[&str] = &["http://", "https://"];

    fn starts_with_one_of(s: &str, prefixes: &[&str]) -> bool {
        prefixes.iter().any(|prefix| s.starts_with(prefix))
    }

    impl BytesLoader for EhttpLoader {
        fn load(&self, ctx: &egui::Context, uri: &str) -> BytesLoadResult {
            if !starts_with_one_of(uri, PROTOCOLS) {
                return Err(LoadError::NotSupported);
            }

            let mut cache = self.cache.lock();
            if let Some(entry) = cache.get(uri).cloned() {
                match entry {
                    Poll::Ready(Ok(bytes)) => Ok(BytesPoll::Ready { size: None, bytes }),
                    Poll::Ready(Err(err)) => Err(LoadError::Custom(err)),
                    Poll::Pending => Ok(BytesPoll::Pending { size: None }),
                }
            } else {
                let uri = uri.to_owned();
                cache.insert(uri.clone(), Poll::Pending);
                drop(cache);

                ehttp::fetch(ehttp::Request::get(uri.clone()), {
                    let ctx = ctx.clone();
                    let cache = self.cache.clone();
                    move |result| {
                        let result = match result {
                            Ok(response) if response.ok => Ok(response.bytes.into()),
                            Ok(response) => match response.text() {
                                Some(response_text) => Err(format!(
                                    "failed to get `{uri}`: {} {} {response_text}",
                                    response.status, response.status_text
                                )),
                                None => Err(format!(
                                    "failed to get `{uri}`: {} {}",
                                    response.status, response.status_text
                                )),
                            },
                            Err(err) => Err(err),
                        };
                        let prev = cache.lock().insert(uri, Poll::Ready(result));
                        assert!(matches!(prev, Some(Poll::Pending)));
                        ctx.request_repaint();
                    }
                });

                Ok(BytesPoll::Pending { size: None })
            }
        }
    }
}

#[cfg(feature = "image")]
mod image_loader {
    use super::*;

    #[derive(Default)]
    pub struct ImageCrateLoader {
        cache: Mutex<HashMap<String, Result<ColorImage, String>>>,
    }

    fn is_supported(uri: &str) -> bool {
        let Some(ext) = Path::new(uri).extension().and_then(|ext| ext.to_str()) else { return false };

        matches!(
            ext,
            "avif" | "bmp" | "gif" | "ico" | "jpeg" | "png" | "webp"
        )
    }

    impl ImageLoader for ImageCrateLoader {
        fn load(&self, ctx: &egui::Context, uri: &str, _: SizeHint) -> ImageLoadResult {
            if !is_supported(uri) {
                return Err(LoadError::NotSupported);
            }

            let mut cache = self.cache.lock();
            // NOTE: this `clone` may clone the entire image.
            if let Some(entry) = cache.get(uri).cloned() {
                match entry {
                    Ok(image) => Ok(ImagePoll::Ready { image }),
                    Err(err) => Err(LoadError::Custom(err)),
                }
            } else {
                match ctx.try_load_bytes(uri) {
                    Ok(BytesPoll::Ready { bytes, .. }) => {
                        let result = crate::image::load_image_bytes(&bytes);
                        cache.insert(uri.into(), result.clone()); // cloning the image again
                        match result {
                            Ok(image) => Ok(ImagePoll::Ready { image }),
                            Err(err) => Err(LoadError::Custom(err)),
                        }
                    }
                    Ok(BytesPoll::Pending { size }) => Ok(ImagePoll::Pending { size }),
                    Err(err) => Err(err),
                }
            }
        }
    }
}

#[cfg(feature = "svg")]
mod svg_loader {
    use super::*;

    #[derive(Default)]
    pub struct SvgLoader {
        cache: Mutex<HashMap<(String, SizeHint), Result<ColorImage, String>>>,
    }

    fn is_supported(uri: &str) -> bool {
        let Some(ext) = Path::new(uri).extension().and_then(|ext| ext.to_str()) else { return false };

        matches!(ext, "svg")
    }

    impl ImageLoader for SvgLoader {
        fn load(&self, ctx: &egui::Context, uri: &str, size_hint: SizeHint) -> ImageLoadResult {
            if !is_supported(uri) {
                return Err(LoadError::NotSupported);
            }

            let uri = uri.to_owned();

            let mut cache = self.cache.lock();
            // NOTE: this `clone` may clone the entire image.
            // We also can't avoid the `uri` clone without unsafe code.
            if let Some(entry) = cache.get(&(uri.clone(), size_hint)).cloned() {
                match entry {
                    Ok(image) => Ok(ImagePoll::Ready { image }),
                    Err(err) => Err(LoadError::Custom(err)),
                }
            } else {
                match ctx.try_load_bytes(&uri) {
                    Ok(BytesPoll::Ready { bytes, .. }) => {
                        let fit_to = match size_hint {
                            SizeHint::Original => usvg::FitTo::Original,
                            SizeHint::Width(w) => usvg::FitTo::Width(w),
                            SizeHint::Height(h) => usvg::FitTo::Height(h),
                            SizeHint::Size(w, h) => usvg::FitTo::Size(w, h),
                        };
                        let result = crate::image::load_svg_bytes_with_size(&bytes, fit_to);
                        cache.insert((uri, size_hint), result.clone()); // potentially cloning the image again
                        match result {
                            Ok(image) => Ok(ImagePoll::Ready { image }),
                            Err(err) => Err(LoadError::Custom(err)),
                        }
                    }
                    Ok(BytesPoll::Pending { size }) => Ok(ImagePoll::Pending { size }),
                    Err(err) => Err(err),
                }
            }
        }
    }
}
