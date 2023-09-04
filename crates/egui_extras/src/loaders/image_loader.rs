use super::*;

#[derive(Default)]
pub struct ImageCrateLoader {
    cache: Mutex<HashMap<String, Result<ColorImage, String>>>,
}

fn is_supported(uri: &str) -> bool {
    let Some(ext) = Path::new(uri).extension().and_then(|ext| ext.to_str()) else {
        // `true` because if there's no extension, assume that we support it
        return true
    };

    !matches!(ext, "svg")
}

impl ImageLoader for ImageCrateLoader {
    fn load(&self, ctx: &egui::Context, uri: &str, _: SizeHint) -> ImageLoadResult {
        if !is_supported(uri) {
            return Err(LoadError::NotSupported);
        }

        let mut cache = self.cache.lock();
        // NOTE: this `clone` may clone the entire image.
        if let Some(entry) = cache.get(uri).cloned() {
            match entry {
                Ok(image) => Ok(ImagePoll::Ready { image }),
                Err(err) => Err(LoadError::Custom(err)),
            }
        } else {
            match ctx.try_load_bytes(uri) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    crate::log_trace!("started loading `{uri}`");
                    let result = crate::image::load_image_bytes(&bytes);
                    crate::log_trace!("finished loading `{uri}`");
                    cache.insert(uri.into(), result.clone()); // cloning the image again
                    match result {
                        Ok(image) => Ok(ImagePoll::Ready { image }),
                        Err(err) => Err(LoadError::Custom(err)),
                    }
                }
                Ok(BytesPoll::Pending { size }) => Ok(ImagePoll::Pending { size }),
                Err(err) => Err(err),
            }
        }
    }
}
