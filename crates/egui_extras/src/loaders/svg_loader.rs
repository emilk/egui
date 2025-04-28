use std::{borrow::Cow, mem::size_of, path::Path, sync::Arc};

use ahash::HashMap;

use egui::{
    load::{BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
    ColorImage,
};

type Entry = Result<Arc<ColorImage>, String>;

pub struct SvgLoader {
    cache: Mutex<HashMap<(Cow<'static, str>, SizeHint), Entry>>,
    options: resvg::usvg::Options<'static>,
}

impl SvgLoader {
    pub const ID: &'static str = egui::generate_loader_id!(SvgLoader);
}

fn is_supported(uri: &str) -> bool {
    let Some(ext) = Path::new(uri).extension().and_then(|ext| ext.to_str()) else {
        return false;
    };

    ext == "svg"
}

impl Default for SvgLoader {
    fn default() -> Self {
        // opt is mutated when `svg_text` feature flag is enabled
        #[allow(unused_mut, clippy::allow_attributes)]
        let mut options = resvg::usvg::Options::default();

        #[cfg(feature = "svg_text")]
        options.fontdb_mut().load_system_fonts();

        Self {
            cache: Mutex::new(HashMap::default()),
            options,
        }
    }
}

impl ImageLoader for SvgLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str, size_hint: SizeHint) -> ImageLoadResult {
        if !is_supported(uri) {
            return Err(LoadError::NotSupported);
        }

        let mut cache = self.cache.lock();
        // We can't avoid the `uri` clone here without unsafe code.
        if let Some(entry) = cache.get(&(Cow::Borrowed(uri), size_hint)).cloned() {
            match entry {
                Ok(image) => Ok(ImagePoll::Ready { image }),
                Err(err) => Err(LoadError::Loading(err)),
            }
        } else {
            match ctx.try_load_bytes(uri) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    log::trace!("started loading {uri:?}");
                    let result = crate::image::load_svg_bytes_with_size(
                        &bytes,
                        Some(size_hint),
                        &self.options,
                    )
                    .map(Arc::new);
                    log::trace!("finished loading {uri:?}");
                    cache.insert((Cow::Owned(uri.to_owned()), size_hint), result.clone());
                    match result {
                        Ok(image) => Ok(ImagePoll::Ready { image }),
                        Err(err) => Err(LoadError::Loading(err)),
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

    fn forget_all(&self) {
        self.cache.lock().clear();
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
