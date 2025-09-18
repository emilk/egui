use eframe::egui::load::{ImageLoadResult, ImageLoader, ImagePoll, LoadError};
use eframe::egui::mutex::Mutex;
use eframe::egui::{ColorImage, Context, Image, ImageSource, SizeHint};
use eframe::epaint::ahash::HashMap;
use egui_extras::loaders::image_loader::ImageCrateLoader;
use std::sync::Arc;
use std::time::Duration;

#[derive(Default)]
pub struct DiffLoader {
    image_loader: Arc<ImageCrateLoader>,
    diffs: Arc<Mutex<HashMap<String, ImageLoadResult>>>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct DiffOptions {
    pub threshold: f32,
    pub detect_aa_pixels: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            threshold: 1.0,
            detect_aa_pixels: true,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiffUri {
    pub old: String,
    pub new: String,
    pub options: DiffOptions,
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

impl DiffLoader {
    pub fn new(ctx: &Context) -> Self {
        let image_loader = ctx
            .loaders()
            .image
            .lock()
            .iter()
            .find_map(|l| Arc::downcast(l.clone()).ok())
            .expect("egui_extra ImageLoader should be installed");

        Self {
            image_loader,
            diffs: Arc::new(Mutex::new(HashMap::default())),
        }
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
                let old_image = self.image_loader.load(ctx, &diff_uri.old, size_hint);
                let new_image = self.image_loader.load(ctx, &diff_uri.new, size_hint);

                let (old_image, new_image) = (old_image?, new_image?);

                if let (
                    ImagePoll::Ready { image: old_image },
                    ImagePoll::Ready { image: new_image },
                ) = (old_image, new_image)
                {
                    let cache = self.diffs.clone();
                    let ctx = ctx.clone();

                    self.diffs.lock().insert(
                        diff_uri.to_uri(),
                        ImageLoadResult::Ok(ImagePoll::Pending { size: None }),
                    );

                    let uri = uri.to_string();
                    std::thread::spawn(move || {
                        ctx.request_repaint();
                        let result = load_diffs(&ctx, old_image, new_image, size_hint, diff_uri);
                        cache.lock().insert(uri, result);
                    });
                }
                ImageLoadResult::Ok(ImagePoll::Pending { size: None })
            } else {
                ImageLoadResult::Err(LoadError::NotSupported)
            }
        }
    }

    fn forget(&self, uri: &str) {
        self.diffs.lock().remove(uri);
    }

    fn forget_all(&self) {
        self.diffs.lock().clear();
    }

    fn byte_size(&self) -> usize {
        self.diffs
            .lock()
            .values()
            .map(|r| match r {
                ImageLoadResult::Ok(ImagePoll::Ready { image }) => image.as_raw().len(),
                _ => 0,
            })
            .sum()
    }
}

pub fn load_diffs(
    ctx: &Context,
    old_img: Arc<ColorImage>,
    new_img: Arc<ColorImage>,
    size_hint: SizeHint,
    diff_uri: DiffUri,
) -> ImageLoadResult {
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

    if old.dimensions() != new.dimensions() {
        return ImageLoadResult::Err(LoadError::Loading(
            "Images must have the same dimensions".to_string(),
        ));
    }

    let result = dify::diff::get_results(
        old,
        new,
        diff_uri.options.threshold,
        diff_uri.options.detect_aa_pixels,
        None,
        &None,
        &None,
    );

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
