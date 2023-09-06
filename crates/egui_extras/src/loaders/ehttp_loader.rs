use egui::{
    ahash::HashMap,
    load::{Bytes, BytesLoadResult, BytesLoader, BytesPoll, LoadError},
    mutex::Mutex,
};
use std::{sync::Arc, task::Poll};

type Entry = Poll<Result<Arc<[u8]>, String>>;

#[derive(Default)]
pub struct EhttpLoader {
    cache: Arc<Mutex<HashMap<String, Entry>>>,
}

const PROTOCOLS: &[&str] = &["http://", "https://"];

fn starts_with_one_of(s: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| s.starts_with(prefix))
}

fn get_image_bytes(
    uri: &str,
    response: Result<ehttp::Response, String>,
) -> Result<Arc<[u8]>, String> {
    let response = response?;
    if !response.ok {
        match response.text() {
            Some(response_text) => {
                return Err(format!(
                    "failed to load {uri:?}: {} {} {response_text}",
                    response.status, response.status_text
                ))
            }
            None => {
                return Err(format!(
                    "failed to load {uri:?}: {} {}",
                    response.status, response.status_text
                ))
            }
        }
    }

    let Some(content_type) = response.content_type() else {
    return Err(format!("failed to load {uri:?}: no content-type header found"));
  };
    if !content_type.starts_with("image/") {
        return Err(format!("failed to load {uri:?}: expected content-type starting with \"image/\", found {content_type:?}"));
    }

    Ok(response.bytes.into())
}

impl BytesLoader for EhttpLoader {
    fn load(&self, ctx: &egui::Context, uri: &str) -> BytesLoadResult {
        if !starts_with_one_of(uri, PROTOCOLS) {
            return Err(LoadError::NotSupported);
        }

        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(uri).cloned() {
            match entry {
                Poll::Ready(Ok(bytes)) => Ok(BytesPoll::Ready {
                    size: None,
                    bytes: Bytes::Shared(bytes),
                }),
                Poll::Ready(Err(err)) => Err(LoadError::Custom(err)),
                Poll::Pending => Ok(BytesPoll::Pending { size: None }),
            }
        } else {
            crate::log_trace!("started loading {uri:?}");

            let uri = uri.to_owned();
            cache.insert(uri.clone(), Poll::Pending);
            drop(cache);

            ehttp::fetch(ehttp::Request::get(uri.clone()), {
                let ctx = ctx.clone();
                let cache = self.cache.clone();
                move |response| {
                    let result = get_image_bytes(&uri, response);
                    crate::log_trace!("finished loading {uri:?}");
                    let prev = cache.lock().insert(uri, Poll::Ready(result));
                    assert!(matches!(prev, Some(Poll::Pending)));
                    ctx.request_repaint();
                }
            });

            Ok(BytesPoll::Pending { size: None })
        }
    }

    fn forget(&self, uri: &str) {
        let _ = self.cache.lock().remove(uri);
    }

    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .values()
            .map(|entry| match entry {
                Poll::Ready(Ok(bytes)) => bytes.len(),
                Poll::Ready(Err(err)) => err.len(),
                _ => 0,
            })
            .sum()
    }
}
