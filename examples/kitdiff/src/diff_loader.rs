use eframe::egui::load::{ImageLoadResult, ImageLoader, ImagePoll, LoadError};
use eframe::egui::mutex::Mutex;
use eframe::egui::{ColorImage, Context, Image, ImageSource, SizeHint};
use eframe::epaint::ahash::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
pub struct DiffLoader {
    diffs: Arc<Mutex<HashMap<String, ImageLoadResult>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiffUri {
    pub old: String,
    pub new: String,
}

impl DiffUri {
    pub fn from_uri(uri: &str) -> Option<Self> {
        let stripped = uri.strip_prefix("diff://")?;
        serde_json::from_str(stripped).ok()
    }

    pub fn to_uri(&self) -> String {
        format!("diff://{}", serde_json::to_string(self).unwrap())
    }
}

impl ImageLoader for DiffLoader {
    fn id(&self) -> &str {
        "DiffLoader"
    }

    fn load(&self, ctx: &Context, uri: &str, size_hint: SizeHint) -> ImageLoadResult {
        if !uri.starts_with("diff://") {
            return ImageLoadResult::Err(LoadError::NotSupported);
        }
        if let Some(image) = self.diffs.lock().get(uri) {
            image.clone()
        } else {
            if let Some(diff_uri) = DiffUri::from_uri(uri) {
                let cache = self.diffs.clone();
                let ctx = ctx.clone();

                self.diffs.lock().insert(
                    diff_uri.to_uri(),
                    ImageLoadResult::Ok(ImagePoll::Pending { size: None }),
                );

                let uri = uri.to_string();
                std::thread::spawn(move || {
                    ctx.request_repaint();
                    let result = load_diffs(&ctx, size_hint, diff_uri);
                    cache.lock().insert(uri, result);
                });
                ImageLoadResult::Ok(ImagePoll::Pending { size: None })
            } else {
                ImageLoadResult::Err(LoadError::NotSupported)
            }
        }
    }

    fn forget(&self, uri: &str) {
        todo!()
    }

    fn forget_all(&self) {
        todo!()
    }

    fn byte_size(&self) -> usize {
        todo!()
    }
}

pub fn load_diffs(ctx: &Context, size_hint: SizeHint, diff_uri: DiffUri) -> ImageLoadResult {
    let (old_img, new_img) = loop {
        let old_image = ctx.try_load_image(&diff_uri.old, size_hint);
        let new_image = ctx.try_load_image(&diff_uri.new, size_hint);

        let old_image = old_image?;
        let new_image = new_image?;

        if let (ImagePoll::Ready { image: old_image }, ImagePoll::Ready { image: new_image }) =
            (old_image, new_image)
        {
            break (old_image, new_image);
        }

        std::thread::sleep(Duration::from_millis(10));
    };

    let old = image::RgbaImage::from_vec(
        old_img.width() as u32,
        old_img.height() as u32,
        old_img.as_raw().to_vec(),
    )
    .ok_or(LoadError::Loading(
        "Failed to convert to RgbaImage".to_string(),
    ))?;

    let new = image::RgbaImage::from_vec(
        new_img.width() as u32,
        new_img.height() as u32,
        new_img.as_raw().to_vec(),
    )
    .ok_or(LoadError::Loading(
        "Failed to convert to RgbaImage".to_string(),
    ))?;

    let result = dify::diff::get_results(old, new, 1.0, false, None, &None, &None);

    if let Some((pixels, image)) = result {
        let image = ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        );

        let arc_image = Arc::new(image);
        Ok(ImagePoll::Ready { image: arc_image })
    } else {
        Ok(ImagePoll::Ready {
            image: Arc::new(ColorImage::filled(
                [1, 1],
                eframe::egui::Color32::TRANSPARENT,
            )),
        })
    }
}
