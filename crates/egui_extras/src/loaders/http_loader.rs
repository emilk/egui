use ahash::HashMap;
use egui::{
    load::{Bytes, BytesLoadResult, BytesLoader, BytesPoll, LoadError},
    mutex::Mutex,
};
use std::{sync::Arc, task::Poll};

#[derive(Clone)]
struct File {
    bytes: Arc<[u8]>,
    mime: Option<String>,
}

impl File {
    fn from_response(uri: &str, response: ehttp::Response) -> Result<Self, String> {
        if !response.ok {
            match response.text() {
                Some(response_text) => {
                    return Err(format!(
                        "failed to load {uri:?}: {} {} {response_text}",
                        response.status, response.status_text
                    ));
                }
                None => {
                    return Err(format!(
                        "failed to load {uri:?}: {} {}",
                        response.status, response.status_text
                    ));
                }
            }
        }

        let mime = response.content_type().map(|v| v.to_owned());
        let bytes = response.bytes.into();

        Ok(Self { bytes, mime })
    }
}

type Entry = Poll<Result<File, String>>;

#[derive(Default)]
pub struct EhttpLoader {
    cache: Arc<Mutex<HashMap<String, Entry>>>,
}

impl EhttpLoader {
    pub const ID: &'static str = egui::generate_loader_id!(EhttpLoader);
}

const PROTOCOLS: &[&str] = &["http://", "https://"];

fn starts_with_one_of(s: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| s.starts_with(prefix))
}

impl BytesLoader for EhttpLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str) -> BytesLoadResult {
        if !starts_with_one_of(uri, PROTOCOLS) {
            return Err(LoadError::NotSupported);
        }

        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(uri).cloned() {
            match entry {
                Poll::Ready(Ok(file)) => Ok(BytesPoll::Ready {
                    size: None,
                    bytes: Bytes::Shared(file.bytes),
                    mime: file.mime,
                }),
                Poll::Ready(Err(err)) => Err(LoadError::Loading(err)),
                Poll::Pending => Ok(BytesPoll::Pending { size: None }),
            }
        } else {
            log::trace!("started loading {uri:?}");

            let uri = uri.to_owned();
            cache.insert(uri.clone(), Poll::Pending);
            drop(cache);

            ehttp::fetch(ehttp::Request::get(uri.clone()), {
                let ctx = ctx.clone();
                let cache = Arc::clone(&self.cache);
                move |response| {
                    let result = match response {
                        Ok(response) => File::from_response(&uri, response),
                        Err(err) => {
                            // Log details; return summary
                            log::error!("Failed to load {uri:?}: {err}");
                            Err(format!("Failed to load {uri:?}"))
                        }
                    };
                    let repaint = {
                        let mut cache = cache.lock();
                        if let std::collections::hash_map::Entry::Occupied(mut entry) =
                            cache.entry(uri.clone())
                        {
                            let entry = entry.get_mut();
                            *entry = Poll::Ready(result);
                            log::trace!("Finished loading {uri:?}");
                            true
                        } else {
                            log::trace!(
                                "Canceled loading {uri:?}\nNote: This can happen if `forget_image` is called while the image is still loading."
                            );
                            false
                        }
                    };
                    // We may not lock Context while the cache lock is held (see ImageLoader::load
                    // for details).
                    if repaint {
                        ctx.request_repaint();
                    }
                }
            });

            Ok(BytesPoll::Pending { size: None })
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
            .map(|entry| match entry {
                Poll::Ready(Ok(file)) => {
                    file.bytes.len() + file.mime.as_ref().map_or(0, |m| m.len())
                }
                Poll::Ready(Err(err)) => err.len(),
                _ => 0,
            })
            .sum()
    }

    fn has_pending(&self) -> bool {
        self.cache.lock().values().any(|entry| entry.is_pending())
    }
}
