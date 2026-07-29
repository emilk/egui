/// A file dropped into egui.
///
/// egui never reads the contents. The representation depends on the target because native and web
/// integrations cannot produce each other's handle type.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    all(feature = "serde", not(target_arch = "wasm32")),
    derive(serde::Deserialize, serde::Serialize)
)]
pub struct DroppedFile {
    /// A path on the local file system.
    #[cfg(not(target_arch = "wasm32"))]
    pub path: std::path::PathBuf,

    /// A handle to a file picked by the user in the browser.
    ///
    /// Nothing is read until you ask for it, e.g. with `Blob::array_buffer` or
    /// `Blob::stream`. Both are asynchronous, so a typical app spawns the read with
    /// `wasm_bindgen_futures::spawn_local` and stores the result in its own state.
    ///
    /// A `web_sys::File` is a JavaScript handle, so it is `Send + Sync` only in wasm builds
    /// without `target_feature = "atomics"`. With atomics enabled, [`crate::Context`]
    /// therefore stops being `Send + Sync`.
    #[cfg(target_arch = "wasm32")]
    pub file: web_sys::File,
}
