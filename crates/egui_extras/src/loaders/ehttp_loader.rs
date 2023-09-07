use egui::{
    ahash::HashMap,
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

        let mime = response.content_type().map(|v| v.to_owned());
        let bytes = response.bytes.into();

        Ok(File { bytes, mime })
    }
}

type Entry = Poll<Result<File, String>>;

#[derive(Default)]
pub struct EhttpLoader {
    cache: Arc<Mutex<HashMap<String, Entry>>>,
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
                Poll::Ready(Ok(file)) => Ok(BytesPoll::Ready {
                    size: None,
                    bytes: Bytes::Shared(file.bytes),
                    mime: file.mime,
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
                    let result = match response {
                        Ok(response) => File::from_response(&uri, response),
                        Err(err) => {
                            crate::log_err!("failed to load {uri:?}\n{err}");
                            Err(format!("failed to load {uri:?}"))
                        }
                    };
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
                Poll::Ready(Ok(file)) => {
                    file.bytes.len() + file.mime.as_ref().map_or(0, |m| m.len())
                }
                Poll::Ready(Err(err)) => err.len(),
                _ => 0,
            })
            .sum()
    }
}
