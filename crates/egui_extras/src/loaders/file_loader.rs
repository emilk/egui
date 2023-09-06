use egui::{
    ahash::HashMap,
    load::{Bytes, BytesLoadResult, BytesLoader, BytesPoll, LoadError},
    mutex::Mutex,
};
use std::{sync::Arc, task::Poll, thread};

type Entry = Poll<Result<Arc<[u8]>, String>>;

#[derive(Default)]
pub struct FileLoader {
    /// Cache for loaded files
    cache: Arc<Mutex<HashMap<String, Entry>>>,
}

const PROTOCOL: &str = "file://";

impl BytesLoader for FileLoader {
    fn load(&self, ctx: &egui::Context, uri: &str) -> BytesLoadResult {
        // File loader only supports the `file` protocol.
        let Some(path) = uri.strip_prefix(PROTOCOL) else {
            return Err(LoadError::NotSupported);
        };

        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(path).cloned() {
            // `path` has either begun loading, is loaded, or has failed to load.
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
            // We need to load the file at `path`.

            // Set the file to `pending` until we finish loading it.
            let path = path.to_owned();
            cache.insert(path.clone(), Poll::Pending);
            drop(cache);

            // Spawn a thread to read the file, so that we don't block the render for too long.
            thread::Builder::new()
                .name(format!("egui_extras::FileLoader::load({uri:?})"))
                .spawn({
                    let ctx = ctx.clone();
                    let cache = self.cache.clone();
                    let uri = uri.to_owned();
                    move || {
                        let result = match std::fs::read(&path) {
                            Ok(bytes) => Ok(bytes.into()),
                            Err(err) => Err(err.to_string()),
                        };
                        let prev = cache.lock().insert(path, Poll::Ready(result));
                        assert!(matches!(prev, Some(Poll::Pending)));
                        ctx.request_repaint();
                        crate::log_trace!("finished loading {uri:?}");
                    }
                })
                .expect("failed to spawn thread");

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
