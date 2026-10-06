//! Shared machinery for image loaders that decode images on a background thread.

use ahash::HashMap;
use core::task::Poll;
use egui::{load::Bytes, mutex::Mutex};
use std::sync::Arc;

type Entry<T> = Poll<Result<T, String>>;

/// A cache of decoded images, keyed by uri.
///
/// On native, the decoding happens on a background thread, and the entry is
/// [`Poll::Pending`] until it is done.
/// On web, the decoding happens immediately.
pub(crate) struct DecodeCache<T> {
    /// Used for logging and thread names.
    loader_name: &'static str,
    cache: Arc<Mutex<HashMap<String, Entry<T>>>>,
}

impl<T: Clone + Send + 'static> DecodeCache<T> {
    pub fn new(loader_name: &'static str) -> Self {
        Self {
            loader_name,
            cache: Default::default(),
        }
    }

    pub fn get(&self, uri: &str) -> Option<Entry<T>> {
        self.cache.lock().get(uri).cloned()
    }

    /// Start decoding the given bytes, and cache the result under `uri`.
    ///
    /// On native this returns [`Poll::Pending`] and requests a repaint once decoding is done.
    /// On web this decodes immediately and returns [`Poll::Ready`].
    pub fn decode(
        &self,
        ctx: &egui::Context,
        uri: &str,
        bytes: &Bytes,
        decode: impl FnOnce(&[u8]) -> Result<T, String> + Send + 'static,
    ) -> Entry<T> {
        let loader_name = self.loader_name;

        cfg_select! {
            target_arch = "wasm32" => {
                let _ = ctx;
                log::trace!("{loader_name} - started loading {uri:?}");
                let result = decode(bytes);
                log::trace!("{loader_name} - finished loading {uri:?}");
                self.cache
                    .lock()
                    .insert(uri.to_owned(), Poll::Ready(result.clone()));
                Poll::Ready(result)
            }
            _ => {
                let uri = uri.to_owned();
                self.cache.lock().insert(uri.clone(), Poll::Pending);

                // Do the image parsing on a bg thread
                std::thread::Builder::new()
                    .name(format!("egui_extras::{loader_name}::load({uri:?})"))
                    .spawn({
                        let ctx = ctx.clone();
                        let cache = Arc::clone(&self.cache);
                        let bytes = bytes.clone();
                        move || {
                            log::trace!("{loader_name} - started loading {uri:?}");
                            let result = decode(&bytes);
                            let repaint = {
                                let mut cache = cache.lock();
                                if let Some(entry) = cache.get_mut(&uri) {
                                    *entry = Poll::Ready(result);
                                    log::trace!("{loader_name} - finished loading {uri:?}");
                                    true
                                } else {
                                    log::trace!(
                                        "{loader_name} - canceled loading {uri:?}\nNote: This can happen if `forget_image` is called while the image is still loading."
                                    );
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

                Poll::Pending
            }
        }
    }

    pub fn forget(&self, uri: &str) {
        let _ = self.cache.lock().remove(uri);
    }

    pub fn forget_all(&self) {
        self.cache.lock().clear();
    }

    pub fn byte_size(&self, byte_size: impl Fn(&T) -> usize) -> usize {
        self.cache
            .lock()
            .values()
            .map(|entry| match entry {
                Poll::Ready(Ok(value)) => byte_size(value),
                Poll::Ready(Err(err)) => err.len(),
                Poll::Pending => 0,
            })
            .sum()
    }

    pub fn has_pending(&self) -> bool {
        self.cache.lock().values().any(|entry| entry.is_pending())
    }
}
