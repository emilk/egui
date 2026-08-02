use std::{future::Future, path::Path, pin::Pin};

#[derive(Debug)]
pub(crate) struct NativeDroppedFile {
    path: std::path::PathBuf,
}

impl NativeDroppedFile {
    pub(crate) fn from_path(path: std::path::PathBuf) -> egui::DroppedFileHandle {
        std::sync::Arc::new(Self { path })
    }
}

impl egui::DroppedFile for NativeDroppedFile {
    fn path(&self) -> &Path {
        &self.path
    }

    fn bytes_async(&self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + '_>> {
        let path = self.path.clone();
        Box::pin(async move { std::fs::read(path).map_err(|err| err.to_string()) })
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.path).map_err(|err| err.to_string())
    }
}
