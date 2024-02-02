use egui::{
    ahash::HashMap,
    load::{BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
    ColorImage,
};
use std::{mem::size_of, sync::Arc};

type Entry = Result<Arc<ColorImage>, String>;

#[derive(Default)]
pub struct Rgba8ImageLoader {
    cache: Mutex<HashMap<String, Entry>>,
}

impl Rgba8ImageLoader {
    pub const ID: &'static str = egui::generate_loader_id!(DynamicImageCrateLoader);
}

fn is_supported_uri(uri: &str) -> bool {
    uri.starts_with("rgba8://")
}

impl ImageLoader for Rgba8ImageLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str, _: SizeHint) -> ImageLoadResult {
        // three stages of guessing if we support loading the image:
        // 1. URI extension

        // (1)
        if !is_supported_uri(uri) {
            return Err(LoadError::NotSupported);
        }

        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(uri).cloned() {
            match entry {
                Ok(image) => Ok(ImagePoll::Ready { image }),
                Err(err) => Err(LoadError::Loading(err)),
            }
        } else {
            match ctx.try_load_bytes(uri) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    log::trace!("started loading {uri:?}");
                    let result = crate::image::load_rgba(&bytes).map(Arc::new);
                    log::trace!("finished loading {uri:?}");
                    cache.insert(uri.into(), result.clone());
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
                Ok(image) => image.pixels.len() * size_of::<egui::Color32>(),
                Err(err) => err.len(),
            })
            .sum()
    }
}
