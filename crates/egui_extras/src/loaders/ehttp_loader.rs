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
            crate::log_trace!("cannot load `{uri}`, not supported");
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
            crate::log_trace!("started loading `{uri}`");

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
                    crate::log_trace!("finished loading `{uri}`");
                    let prev = cache.lock().insert(uri, Poll::Ready(result));
                    assert!(matches!(prev, Some(Poll::Pending)));
                    ctx.request_repaint();
                }
            });

            Ok(BytesPoll::Pending { size: None })
        }
    }
}
