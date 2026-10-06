use ahash::HashMap;
use core::task::Poll;
use egui::{
    load::{Bytes, BytesLoadResult, BytesLoader, BytesPoll, LoadError},
    mutex::Mutex,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

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

/// Converts a `file://` URI into a `PathBuf`, without any percent-decoding.
///
/// Note that there is only minimal translation of the uri string into a path to support windows
/// file and unc paths.
///
/// See also [`convert_uri_to_decoded_path`].
fn convert_uri_to_path(uri: &str) -> Result<PathBuf, egui::load::LoadError> {
    // File loader only supports the `file` protocol.
    let s = uri
        .strip_prefix(PROTOCOL)
        .ok_or(egui::load::LoadError::NotSupported)?;
    Ok(uri_path_to_path(s))
}

/// Like [`convert_uri_to_path`], but with `%XX` escapes (e.g. `%20`) percent-decoded.
///
/// Returns `None` if the uri is not a `file://` uri, if it contains no valid escapes
/// (so the result would be the same as [`convert_uri_to_path`]),
/// or if the decoded bytes are not a valid path on this platform.
fn convert_uri_to_decoded_path(uri: &str) -> Option<PathBuf> {
    let s = uri.strip_prefix(PROTOCOL)?;
    let bytes = percent_decode(s)?;

    cfg_select! {
        unix => {
            use std::os::unix::ffi::OsStringExt as _;
            Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
        }
        _ => {
            let s = String::from_utf8(bytes).ok()?;
            Some(uri_path_to_path(&s))
        }
    }
}

/// Converts the part of a `file://` uri after the protocol into a path.
fn uri_path_to_path(s: &str) -> PathBuf {
    cfg_select! {
        target_os = "windows" => {
            // Standard windows file uris should have the form
            //
            // file:///c:/path/to/the%20file.txt
            //
            // in which the hostname field is left out. Check for this by looking at the next character
            // after the schema, if it's a slash then we likely have a standard file path.
            if let Some(stripped) = s.strip_prefix("/") {
                PathBuf::from(stripped)
            } else {
                // If it's not a standard file uri, it might be a UNC network path of the form
                //
                // file://hostname/path/to/the%20file.txt
                //
                // These file uris need to be converted into UNC correct and so need to have the leading
                // two backslashes prepended.
                PathBuf::from(format!("\\\\{s}"))
            }
        }
        _ => PathBuf::from(s),
    }
}

/// Percent-decodes `%XX` escapes, leaving any invalid escape (like `%zz` or a trailing `%`) as-is.
///
/// Returns `None` if there was nothing to decode.
fn percent_decode(s: &str) -> Option<Vec<u8>> {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut decoded_any = false;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2]))
        {
            out.push((hi << 4) | lo);
            decoded_any = true;
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    decoded_any.then_some(out)
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Reads the file at `path`, falling back to `decoded_path` if `path` does not exist.
///
/// We try the raw path first so that paths that worked before percent-decoding was added
/// (e.g. a file literally named `My%20Doc.png`, or `format!("file://{}", path.display())`)
/// keep resolving exactly as before.
fn read_file(path: &Path, decoded_path: Option<&Path>) -> Result<File, String> {
    let (path, bytes) = match std::fs::read(path) {
        Ok(bytes) => (path, bytes),
        Err(err) => match decoded_path {
            Some(decoded_path) if err.kind() == std::io::ErrorKind::NotFound => {
                match std::fs::read(decoded_path) {
                    Ok(bytes) => (decoded_path, bytes),
                    Err(_) => return Err(err.to_string()),
                }
            }
            _ => return Err(err.to_string()),
        },
    };

    Ok(File {
        bytes: bytes.into(),
        mime: guess_mime(path),
    })
}

fn guess_mime(path: &Path) -> Option<String> {
    cfg_select! {
        feature = "file" => {
            mime_guess2::from_path(path)
                .first_raw()
                .map(|v| v.to_owned())
        }
        _ => {
            _ = path;
            None
        }
    }
}

impl BytesLoader for FileLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri: &str) -> BytesLoadResult {
        let path = convert_uri_to_path(uri)?;
        let decoded_path = convert_uri_to_decoded_path(uri);

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
                        let result = read_file(&path, decoded_path.as_deref());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_convert_uri_to_path() {
        let mut checks: Vec<(&str, Result<PathBuf, egui::load::LoadError>, &str)> = vec![
            (
                "http://host/path/to/image.jpg",
                Err(egui::load::LoadError::NotSupported),
                "Schemas other than file are rejected.",
            ),
            (
                "https://host/path/to/image.jpg",
                Err(egui::load::LoadError::NotSupported),
                "Schemas other than file are rejected.",
            ),
            (
                "ftp://host/path/to/image.jpg",
                Err(egui::load::LoadError::NotSupported),
                "Schemas other than file are rejected.",
            ),
        ];
        if cfg!(target_os = "windows") {
            let mut windows_checks = vec![
                (
                    "file:///path/to/image.jpg",
                    Ok(PathBuf::from("path\\to\\image.jpg")),
                    "file uris with no hosts and no drive letter are turned into bare paths on windows.",
                ),
                (
                    "file:///c:/path/to/image.jpg",
                    Ok(PathBuf::from("c:\\path\\to\\image.jpg")),
                    "file uris with no hosts and drive letters are turned into absolute paths on windows.",
                ),
                (
                    "file://host/share/path/to/image.jpg",
                    Ok(PathBuf::from("\\\\host\\share\\path\\to\\image.jpg")),
                    "file uris with a host are turned into UNC paths with leading backslashes on windows.",
                ),
                (
                    "file:///c:/path/to/the%20image.jpg",
                    Ok(PathBuf::from("c:\\path\\to\\the%20image.jpg")),
                    "the raw path is not percent-decoded.",
                ),
            ];
            checks.append(&mut windows_checks);
        } else {
            let mut more_checks = vec![
                (
                    "file://path/to/image.jpg",
                    Ok(PathBuf::from("path/to/image.jpg")),
                    "file uris are turned into bare paths.",
                ),
                (
                    "file://path/to/the%20image.jpg",
                    Ok(PathBuf::from("path/to/the%20image.jpg")),
                    "the raw path is not percent-decoded.",
                ),
            ];
            checks.append(&mut more_checks);
        }
        for (uri_s, path, reason) in checks {
            assert_eq!(convert_uri_to_path(uri_s), path, "{reason}");
        }
    }

    #[test]
    fn check_percent_decode() {
        assert_eq!(percent_decode("a/b.png"), None, "nothing to decode");
        assert_eq!(
            percent_decode("a%20b.png").as_deref(),
            Some(&b"a b.png"[..])
        );
        assert_eq!(
            percent_decode("na%C3%AFve.png").as_deref(),
            Some("naïve.png".as_bytes()),
            "multi-byte UTF-8"
        );
        assert_eq!(
            percent_decode("na%c3%afve.png").as_deref(),
            Some("naïve.png".as_bytes()),
            "lower-case hex"
        );
        assert_eq!(
            percent_decode("100%25.png").as_deref(),
            Some(&b"100%.png"[..])
        );
        assert_eq!(
            percent_decode("%2520").as_deref(),
            Some(&b"%20"[..]),
            "no double-decoding"
        );

        // Invalid escapes are left as-is:
        assert_eq!(percent_decode("a%zzb"), None);
        assert_eq!(percent_decode("a%"), None);
        assert_eq!(percent_decode("a%2"), None);
        assert_eq!(percent_decode("a%2g"), None);
        assert_eq!(percent_decode("%zz%20%").as_deref(), Some(&b"%zz %"[..]));

        // Invalid UTF-8 is kept losslessly:
        assert_eq!(percent_decode("na%EFve").as_deref(), Some(&b"na\xEFve"[..]));
    }

    #[test]
    fn check_convert_uri_to_decoded_path() {
        assert_eq!(convert_uri_to_decoded_path("http://host/a%20b.jpg"), None);
        assert_eq!(
            convert_uri_to_decoded_path("file://path/to/image.jpg"),
            None
        );

        if cfg!(target_os = "windows") {
            assert_eq!(
                convert_uri_to_decoded_path("file:///c:/path/to/the%20image.jpg"),
                Some(PathBuf::from("c:\\path\\to\\the image.jpg")),
            );
            assert_eq!(
                convert_uri_to_decoded_path("file://host/share/na%C3%AFve.jpg"),
                Some(PathBuf::from("\\\\host\\share\\naïve.jpg")),
            );
        } else {
            assert_eq!(
                convert_uri_to_decoded_path("file:///path/to/the%20image.jpg"),
                Some(PathBuf::from("/path/to/the image.jpg")),
            );
            assert_eq!(
                convert_uri_to_decoded_path("file:///na%C3%AFve/100%25.jpg"),
                Some(PathBuf::from("/naïve/100%.jpg")),
            );
        }

        cfg_select! {
            unix => {
                use std::os::unix::ffi::OsStrExt as _;
                assert_eq!(
                    convert_uri_to_decoded_path("file:///na%EFve.jpg"),
                    Some(PathBuf::from(std::ffi::OsStr::from_bytes(b"/na\xEFve.jpg"))),
                    "invalid UTF-8 is a valid path on unix"
                );
            }
            _ => {
                assert_eq!(
                    convert_uri_to_decoded_path("file:///c:/na%EFve.jpg"),
                    None,
                    "invalid UTF-8 can't be decoded"
                );
            }
        }
    }

    #[test]
    fn check_read_file_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let dir = dir.path();

        let read = |name: &str| {
            let uri = if cfg!(target_os = "windows") {
                format!("{PROTOCOL}/{}/{name}", dir.display())
            } else {
                format!("{PROTOCOL}{}/{name}", dir.display())
            };
            let path = convert_uri_to_path(&uri).unwrap();
            let decoded_path = convert_uri_to_decoded_path(&uri);
            read_file(&path, decoded_path.as_deref()).map(|file| file.bytes.to_vec())
        };

        std::fs::write(dir.join("a b.txt"), "space").unwrap();
        std::fs::write(dir.join("My%20Doc.txt"), "literal").unwrap();
        std::fs::write(dir.join("both%20.txt"), "both raw").unwrap();
        std::fs::write(dir.join("both .txt"), "both decoded").unwrap();

        assert_eq!(read("a%20b.txt").unwrap(), b"space", "decoded");
        assert_eq!(read("a b.txt").unwrap(), b"space", "unencoded");
        assert_eq!(
            read("My%20Doc.txt").unwrap(),
            b"literal",
            "literal %20 in file name"
        );
        assert_eq!(read("both%20.txt").unwrap(), b"both raw", "raw wins");
        assert!(read("missing%20.txt").is_err());
    }
}
