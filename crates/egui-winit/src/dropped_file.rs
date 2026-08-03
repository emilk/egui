use std::path::Path;

#[derive(Debug)]
pub(crate) struct NativeFile {
    path: std::path::PathBuf,
}

impl NativeFile {
    pub(crate) fn from_path(path: std::path::PathBuf) -> egui::DroppedFileHandle {
        std::sync::Arc::new(Self { path })
    }
}

impl egui::DroppedFile for NativeFile {
    fn path(&self) -> &Path {
        &self.path
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.path).map_err(|err| err.to_string())
    }
}
