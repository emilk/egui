use std::{future::Future, pin::Pin};

#[derive(Debug)]
pub(crate) struct WebFile {
    file: web_sys::File,
}

impl WebFile {
    pub(crate) fn from_web_file(file: web_sys::File) -> egui::DroppedFileHandle {
        std::sync::Arc::new(Self { file })
    }
}

impl egui::DroppedFile for WebFile {
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
