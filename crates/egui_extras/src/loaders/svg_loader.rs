use std::{mem::size_of, sync::Arc};

use ahash::HashMap;

use egui::{
    ColorImage,
    load::{BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
};

struct Entry {
    last_used: u64,
    result: Result<Arc<ColorImage>, String>,
}

struct State {
    pass_index: u64,
    cache: HashMap<String, HashMap<SizeHint, Entry>>,
    options: resvg::usvg::Options<'static>,
}

pub struct SvgLoader {
    state: Mutex<State>,
}

impl SvgLoader {
    pub const ID: &'static str = egui::generate_loader_id!(SvgLoader);
}

fn is_supported(uri: &str) -> bool {
    uri.ends_with(".svg")
}

impl Default for SvgLoader {
    fn default() -> Self {
        // opt is mutated when `svg_text` feature flag is enabled
        #[allow(clippy::allow_attributes, unused_mut)]
        let mut options = resvg::usvg::Options::default();

        #[cfg(feature = "svg_text")]
        options.fontdb_mut().load_system_fonts();

        Self {
            state: Mutex::new(State {
                pass_index: 0,
                cache: HashMap::default(),
                options,
            }),
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

        let mut state = self.state.lock();
        let State {
            pass_index,
            cache,
            options,
        } = &mut *state;

        let bucket = cache.entry(uri.to_owned()).or_default();

        if let Some(entry) = bucket.get_mut(&size_hint) {
            entry.last_used = *pass_index;

            match entry.result.clone() {
                Ok(image) => Ok(ImagePoll::Ready { image }),
                Err(err) => Err(LoadError::Loading(err)),
            }
        } else {
            match ctx.try_load_bytes(uri) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    log::trace!("Started loading {uri:?}");
                    let result = crate::image::load_svg_bytes_with_size(&bytes, size_hint, options)
                        .map(Arc::new);

                    log::trace!("Finished loading {uri:?}");
                    bucket.insert(
                        size_hint,
                        Entry {
                            last_used: *pass_index,
                            result: result.clone(),
                        },
                    );
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
        self.state.lock().cache.retain(|key, _| key != uri);
    }

    fn forget_all(&self) {
        self.state.lock().cache.clear();
    }

    fn byte_size(&self) -> usize {
        self.state
            .lock()
            .cache
            .values()
            .flat_map(|bucket| bucket.values())
            .map(|entry| match &entry.result {
                Ok(image) => image.pixels.len() * size_of::<egui::Color32>(),
                Err(err) => err.len(),
            })
            .sum()
    }

    fn end_pass(&self, pass_index: u64) {
        let mut state = self.state.lock();

        state.pass_index = pass_index;

        state.cache.retain(|_key, bucket| {
            if 2 <= bucket.len() {
                // There are multiple images of the same URI (e.g. SVGs of different scales).
                // This could be because someone has an SVG in a resizable container,
                // and so we get a lot of different sizes of it.
                // This could wast RAM, so we remove the ones that are not used in this frame.
                bucket.retain(|_, texture| pass_index <= texture.last_used + 1);
            }
            !bucket.is_empty()
        });
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
