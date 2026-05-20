use ahash::HashMap;
use egui::{
    load::{Bytes, BytesLoadResult, BytesLoader, BytesPoll, LoadError},
    mutex::Mutex,
};
use std::{path::PathBuf, sync::Arc, task::Poll, thread};

#[derive(Clone)]
struct File {
    bytes: Arc<[u8]>,
    mime: Option<String>,
}

type Entry = Poll<Result<File, String>>;

#[derive(Default)]
pub struct FileLoader {
    /// Cache for loaded files
    cache: Arc<Mutex<HashMap<String, Entry>>>,
}

impl FileLoader {
    pub const ID: &'static str = egui::generate_loader_id!(FileLoader);
}

const PROTOCOL: &str = "file://";

/// Converts a hopefully uri encoded string into a `PathBuf`
///
/// Note that there is only minimal transation of the uri string into a path to support windows
/// file and unc paths. Other translations like percent un-encoding are not handled.
fn convert_uri_to_path(s: &str) -> Result<PathBuf, egui::load::LoadError> {
    // File loader only supports the `file` protocol.
    let s = s
        .strip_prefix(PROTOCOL)
        .ok_or(egui::load::LoadError::NotSupported)?;

    if cfg!(target_os = "windows") {
        // Standard windows file uris should have the form
        //
        // file:///c:/path/to/the%20file.txt
        //
        // in which the hostname field is left out. Check for this by looking at the next character
        // after the schema, if it's a slash then we likely have a standard file path.
        if let Some(stripped) = s.strip_prefix("/") {
            let path = PathBuf::from(stripped);
            return Ok(path);
        }

        // If it's not a standard file uri, it might be a UNC network path of the form
        //
        // file://hostname/path/to/the%20file.txt
        //
        // These file uris need to be converted into UNC correct and so need to have the leading
        // two backslashes prepended.
        let path = PathBuf::from(format!("\\\\{s}"));
        return Ok(path);
    }

    Ok(PathBuf::from(s))
}

impl BytesLoader for FileLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str) -> BytesLoadResult {
        let path = convert_uri_to_path(uri)?;

        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(uri).cloned() {
            // `path` has either begun loading, is loaded, or has failed to load.
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
            // We need to load the file at `path`.

            // Set the file to `pending` until we finish loading it.
            cache.insert(uri.to_owned(), Poll::Pending);
            drop(cache);

            // Spawn a thread to read the file, so that we don't block the render for too long.
            thread::Builder::new()
                .name(format!("egui_extras::FileLoader::load({uri:?})"))
                .spawn({
                    let ctx = ctx.clone();
                    let cache = Arc::clone(&self.cache);
                    let uri = uri.to_owned();
                    move || {
                        let result = match std::fs::read(&path) {
                            Ok(bytes) => {
                                #[cfg(feature = "file")]
                                let mime = mime_guess2::from_path(&path)
                                    .first_raw()
                                    .map(|v| v.to_owned());

                                #[cfg(not(feature = "file"))]
                                let mime = None;

                                Ok(File {
                                    bytes: bytes.into(),
                                    mime,
                                })
                            }
                            Err(err) => Err(err.to_string()),
                        };
                        let repaint = {
                            let mut cache = cache.lock();
                            if let std::collections::hash_map::Entry::Occupied(mut entry) = cache.entry(uri.clone()) {
                                let entry = entry.get_mut();
                                *entry = Poll::Ready(result);
                                log::trace!("Finished loading {uri:?}");
                                true
                            } else {
                                log::trace!("Canceled loading {uri:?}\nNote: This can happen if `forget_image` is called while the image is still loading.");
                                false
                            }
                        };
                        // We may not lock Context while the cache lock is held (see ImageLoader::load
                        // for details).
                        if repaint {
                            ctx.request_repaint();
                        }
                    }
                })
                .expect("failed to spawn thread");

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
