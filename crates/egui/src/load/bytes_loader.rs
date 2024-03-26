use super::*;

/// Maps URI:s to [`Bytes`], e.g. found with `include_bytes!`.
///
/// By convention, the URI:s should be prefixed with `bytes://`.
#[derive(Default)]
pub struct DefaultBytesLoader {
    cache: Mutex<HashMap<Cow<'static, str>, Bytes>>,
}

impl DefaultBytesLoader {
    pub fn insert(&self, uri: impl Into<Cow<'static, str>>, bytes: impl Into<Bytes>) {
        self.cache
            .lock()
            .entry(uri.into())
            .or_insert_with_key(|_uri| {
                let bytes: Bytes = bytes.into();

                #[cfg(feature = "log")]
                log::trace!("loaded {} bytes for uri {_uri:?}", bytes.len());

                bytes
            });
    }
}

impl BytesLoader for DefaultBytesLoader {
    fn id(&self) -> &str {
        generate_loader_id!(DefaultBytesLoader)
    }

    fn load(&self, _: &Context, uri: &str) -> BytesLoadResult {
        // We accept uri:s that don't start with `bytes://` too… for now.
        match self.cache.lock().get(uri).cloned() {
            Some(bytes) => Ok(BytesPoll::Ready {
                size: None,
                bytes,
                mime: None,
            }),
            None => {
                if uri.starts_with("bytes://") {
                    Err(LoadError::Loading(
                        "Bytes not found. Did you forget to call Context::include_bytes?".into(),
                    ))
                } else {
                    Err(LoadError::NotSupported)
                }
            }
        }
    }

    fn forget(&self, uri: &str) {
        #[cfg(feature = "log")]
        log::trace!("forget {uri:?}");

        let _ = self.cache.lock().remove(uri);
    }

    fn forget_all(&self) {
        #[cfg(feature = "log")]
        log::trace!("forget all");

        self.cache.lock().clear();
    }

    fn byte_size(&self) -> usize {
        self.cache.lock().values().map(|bytes| bytes.len()).sum()
    }
}
