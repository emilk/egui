use std::{future::Future, pin::Pin};

#[derive(Debug)]
pub(crate) struct WebDroppedFile {
    file: web_sys::File,
}

impl WebDroppedFile {
    pub(crate) fn from_web_file(file: web_sys::File) -> egui::DroppedFileHandle {
        std::sync::Arc::new(Self { file })
    }
}

impl egui::DroppedFile for WebDroppedFile {
    fn bytes_async(&self) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + '_>> {
        let file = self.file.clone();
        Box::pin(async move {
            let array_buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
                .await
                .map_err(|err| crate::web::string_from_js_value(&err))?;
            Ok(js_sys::Uint8Array::new(&array_buffer).to_vec())
        })
    }

    fn web_file(&self) -> Option<&web_sys::File> {
        Some(&self.file)
    }
}
