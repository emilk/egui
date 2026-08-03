use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

#[derive(Debug)]
pub(crate) struct WebFile {
    file: web_sys::File,
    // We store a `PathBuf` here so that we can hand out `Path`s
    // without allocating each time.
    path: PathBuf,
}

impl From<web_sys::File> for WebFile {
    fn from(file: web_sys::File) -> Self {
        let path = file.name().into();
        Self { file, path }
    }
}

impl egui::DroppedFile for WebFile {
    fn path(&self) -> &Path {
        &self.path
    }

    fn bytes_async(&self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + '_>> {
        let file = self.file.clone();
        Box::pin(async move {
            if file.size() > f64::from(u32::MAX) {
                return Err(format!(
                    "File is too large: browser file reads are limited to {} bytes",
                    u32::MAX
                ));
            }

            let array_buffer = file
                .array_buffer()
                .await
                .map_err(|err| crate::web::string_from_js_value(&err))?;
            Ok(js_sys::Uint8Array::new(&array_buffer).to_vec())
        })
    }

    fn web_file(&self) -> Option<&web_sys::File> {
        Some(&self.file)
    }
}
