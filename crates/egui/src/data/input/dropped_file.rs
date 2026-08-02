use std::{future::Future, pin::Pin, sync::Arc};

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

/// A file dropped into egui.
///
/// The integration owns the concrete file handle, letting egui remain independent of windowing
/// backends and file APIs.
pub trait DroppedFile: std::fmt::Debug {
    /// The path of a file dropped on a native platform.
    #[cfg(not(target_arch = "wasm32"))]
    fn path(&self) -> &Path;

    /// Read the file contents.
    ///
    /// This is asynchronous because browsers can only read files asynchronously.
    fn bytes_async(&self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + '_>>;

    /// Read the file contents.
    #[cfg(not(target_arch = "wasm32"))]
    fn bytes(&self) -> Result<Vec<u8>, String>;

    /// The browser file handle, if this file was dropped on the web.
    #[cfg(target_arch = "wasm32")]
    fn web_file(&self) -> Option<&web_sys::File> {
        None
    }
}

/// A shared reference to a dropped file.
#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
pub type DroppedFileHandle = Arc<dyn DroppedFile + Send + Sync>;

/// A shared reference to a dropped file.
///
/// This is not necessarily `Send + Sync` when wasm threads are enabled, because
/// [`web_sys::File`] is not thread-safe in that configuration.
#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
pub type DroppedFileHandle = Arc<dyn DroppedFile>;
