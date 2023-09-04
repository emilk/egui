use super::*;

#[derive(Default)]
pub struct SvgLoader {
    cache: Mutex<HashMap<(String, SizeHint), Result<ColorImage, String>>>,
}

fn is_supported(uri: &str) -> bool {
    let Some(ext) = Path::new(uri).extension().and_then(|ext| ext.to_str()) else { return false };

    matches!(ext, "svg")
}

impl ImageLoader for SvgLoader {
    fn load(&self, ctx: &egui::Context, uri: &str, size_hint: SizeHint) -> ImageLoadResult {
        if !is_supported(uri) {
            crate::log_trace!("cannot load `{uri}`, not supported");
            return Err(LoadError::NotSupported);
        }

        let uri = uri.to_owned();

        let mut cache = self.cache.lock();
        // NOTE: this `clone` may clone the entire image.
        // We also can't avoid the `uri` clone without unsafe code.
        if let Some(entry) = cache.get(&(uri.clone(), size_hint)).cloned() {
            match entry {
                Ok(image) => Ok(ImagePoll::Ready { image }),
                Err(err) => Err(LoadError::Custom(err)),
            }
        } else {
            match ctx.try_load_bytes(&uri) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    crate::log_trace!("started loading `{uri}`");
                    let fit_to = match size_hint {
                        SizeHint::Original => usvg::FitTo::Original,
                        SizeHint::Width(w) => usvg::FitTo::Width(w),
                        SizeHint::Height(h) => usvg::FitTo::Height(h),
                        SizeHint::Size(w, h) => usvg::FitTo::Size(w, h),
                    };
                    let result = crate::image::load_svg_bytes_with_size(&bytes, fit_to);
                    crate::log_trace!("finished loading `{uri}`");
                    cache.insert((uri, size_hint), result.clone()); // potentially cloning the image again
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
