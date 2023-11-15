use egui::{
    ahash::HashMap,
    load::{Bytes, BytesLoadResult, BytesLoader, BytesPoll, LoadError},
    mutex::Mutex,
};
use image::EncodableLayout;
use reqwest::header::CONTENT_TYPE;
use reqwest::Client;
use std::sync::OnceLock;
use std::{sync::Arc, task::Poll};
use tokio::runtime;

#[derive(Clone)]
struct File {
    bytes: Arc<[u8]>,
    mime: Option<String>,
}

static REQWEST: OnceLock<Client> = OnceLock::new();
static RT_HANDLE: OnceLock<runtime::Handle> = OnceLock::new();

impl File {
    async fn from_response(uri: &str, response: reqwest::Response) -> Result<Self, String> {
        let status = response.status();
        let mime = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok().map(|v| v.to_owned()));
        match response.bytes().await {
            Ok(bytes) => Ok(File {
                bytes: bytes.as_bytes().into(),
                mime,
            }),
            Err(err) => Err(format!(
                "failed to load {uri:?}: {} {}; Error: {}",
                status,
                status.canonical_reason().unwrap_or_default(),
                err
            )),
        }
    }
}

type Entry = Poll<Result<File, String>>;

#[derive(Default)]
pub struct ReqwestLoader {
    cache: Arc<Mutex<HashMap<String, Entry>>>,
}

impl ReqwestLoader {
    pub const ID: &str = egui::generate_loader_id!(ReqwestLoader);
}

const PROTOCOLS: &[&str] = &["http://", "https://"];

fn starts_with_one_of(s: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| s.starts_with(prefix))
}

impl BytesLoader for ReqwestLoader {
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

            let rt_handle = if let Some(rth) = RT_HANDLE.get() {
                rth.clone()
            } else {
                match runtime::Handle::try_current() {
                    Ok(rth) => {
                        let r = rth.clone();
                        _ = RT_HANDLE.set(rth);
                        r
                    }
                    Err(err) => {
                        let err = format!("Failed to attach to tokio runtime {err}");
                        cache.insert(uri.clone(), Poll::Ready(Err(err.clone())));
                        log::error!("{}", err);
                        return Err(LoadError::Loading(err));
                    }
                }
            };

            cache.insert(uri.clone(), Poll::Pending);
            drop(cache);

            let ctx = ctx.clone();
            let cache = self.cache.clone();

            _ = rt_handle.spawn(async move {
                let response = REQWEST.get_or_init(Client::new).get(&uri).send().await;
                let result = match response {
                    Ok(response) => File::from_response(&uri, response).await,
                    Err(err) => {
                        // Log details; return summary
                        log::error!("Failed to load {uri:?}: {err}");
                        Err(format!("Failed to load {uri:?}"))
                    }
                };
                log::trace!("finished loading {uri:?}");
                let prev = cache.lock().insert(uri, Poll::Ready(result));
                assert!(matches!(prev, Some(Poll::Pending)));
                ctx.request_repaint();
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
}
