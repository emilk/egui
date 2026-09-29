use std::path::{Path, PathBuf};

#[cfg(target_arch = "wasm32")]
use std::pin::Pin;

#[derive(Debug)]
pub(crate) struct NativeFile {
    path: PathBuf,
}

impl From<PathBuf> for NativeFile {
    fn from(path: PathBuf) -> Self {
        Self { path }
    }
}

impl egui::DroppedFile for NativeFile {
    fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.path).map_err(|err| err.to_string())
    }

    #[cfg(target_arch = "wasm32")]
    fn bytes_async(&self) -> Pin<Box<dyn Future<Output = Result<std::vec::Vec<u8>, String>>>> {
        todo!()
    }
}
