use egui::{
    ahash::HashMap,
    load::{BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
    ColorImage,
};
use std::{mem::size_of, path::Path, sync::Arc};

type Entry = Result<Arc<ColorImage>, String>;

#[derive(Default)]
pub struct SvgLoader {
    cache: Mutex<HashMap<(String, SizeHint), Entry>>,
}

fn is_supported(uri: &str) -> bool {
    let Some(ext) = Path::new(uri).extension().and_then(|ext| ext.to_str()) else { return false };

    ext == "svg"
}

impl ImageLoader for SvgLoader {
    fn load(&self, ctx: &egui::Context, uri: &str, size_hint: SizeHint) -> ImageLoadResult {
        if !is_supported(uri) {
            return Err(LoadError::NotSupported);
        }

        let uri = uri.to_owned();

        let mut cache = self.cache.lock();
        // We can't avoid the `uri` clone here without unsafe code.
        if let Some(entry) = cache.get(&(uri.clone(), size_hint)).cloned() {
            match entry {
                Ok(image) => Ok(ImagePoll::Ready { image }),
                Err(err) => Err(LoadError::Custom(err)),
            }
        } else {
            match ctx.try_load_bytes(&uri) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    crate::log_trace!("started loading {uri:?}");
                    let fit_to = match size_hint {
                        SizeHint::Original => usvg::FitTo::Original,
                        SizeHint::Width(w) => usvg::FitTo::Width(w),
                        SizeHint::Height(h) => usvg::FitTo::Height(h),
                        SizeHint::Size(w, h) => usvg::FitTo::Size(w, h),
                    };
                    let result =
                        crate::image::load_svg_bytes_with_size(&bytes, fit_to).map(Arc::new);
                    crate::log_trace!("finished loading {uri:?}");
                    cache.insert((uri, size_hint), result.clone());
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

    fn forget(&self, uri: &str) {
        self.cache.lock().retain(|(u, _), _| u != uri);
    }

    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .values()
            .map(|result| match result {
                Ok(image) => image.pixels.len() * size_of::<egui::Color32>(),
                Err(err) => err.len(),
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_support() {
        // inverse of same test in `image_loader.rs`
        assert!(!is_supported("https://test.png"));
        assert!(!is_supported("test.jpeg"));
        assert!(!is_supported("http://test.gif"));
        assert!(!is_supported("test.webp"));
        assert!(!is_supported("file://test"));
        assert!(is_supported("test.svg"));
    }
}
