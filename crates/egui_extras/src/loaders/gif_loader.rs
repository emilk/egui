use egui::{
    ahash::HashMap,
    decode_gif_uri,
    load::{Bytes, BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
    ColorImage, GifFrameDurations, Id,
};
use image::AnimationDecoder as _;
use std::{io::Cursor, mem::size_of, sync::Arc, time::Duration};

/// Array of Frames and the duration for how long each frame should be shown
#[derive(Debug, Clone)]
pub struct AnimatedImage {
    frames: Vec<Arc<ColorImage>>,
    frame_durations: GifFrameDurations,
}

impl AnimatedImage {
    pub fn byte_len(&self) -> usize {
        size_of::<Self>()
            + self
                .frames
                .iter()
                .map(|image| {
                    image.pixels.len() * size_of::<egui::Color32>() + size_of::<Duration>()
                })
                .sum::<usize>()
    }

    /// Gets image at index
    pub fn get_image(&self, index: usize) -> Arc<ColorImage> {
        self.frames[index % self.frames.len()].clone()
    }
}
type Entry = Result<Arc<AnimatedImage>, String>;

#[derive(Default)]
pub struct GifLoader {
    cache: Mutex<HashMap<String, Entry>>,
}

impl GifLoader {
    pub const ID: &'static str = egui::generate_loader_id!(GifLoader);
}

pub fn gif_to_sources(data: Bytes) -> Result<AnimatedImage, String> {
    let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(data))
        .map_err(|_err| "Couldnt decode gif".to_owned())?;
    let mut images = vec![];
    let mut durations = vec![];
    for frame in decoder.into_frames() {
        let frame = frame.map_err(|_err| "Couldnt decode gif".to_owned())?;
        let img = frame.buffer();
        let pixels = img.as_flat_samples();

        let delay: std::time::Duration = frame.delay().into();
        images.push(Arc::new(ColorImage::from_rgba_unmultiplied(
            [img.width() as usize, img.height() as usize],
            pixels.as_slice(),
        )));
        durations.push(delay);
    }
    Ok(AnimatedImage {
        frames: images,
        frame_durations: GifFrameDurations(Arc::new(durations)),
    })
}

impl ImageLoader for GifLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri_data: &str, _: SizeHint) -> ImageLoadResult {
        let uri_index = decode_gif_uri(uri_data).map_err(LoadError::Loading);
        let uri = uri_index.as_ref().map(|v| v.0).unwrap_or(uri_data);
        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(uri).cloned() {
            let index = uri_index?.1;
            match entry {
                Ok(image) => Ok(ImagePoll::Ready {
                    image: image.get_image(index),
                }),
                Err(err) => Err(LoadError::Loading(err)),
            }
        } else {
            match ctx.try_load_bytes(uri_data) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    let is_gif = bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a");
                    if !is_gif {
                        return Err(LoadError::NotSupported);
                    }
                    let index = uri_index?.1;
                    log::trace!("started loading {uri:?}");
                    let result = gif_to_sources(bytes).map(Arc::new);
                    if let Ok(v) = &result {
                        ctx.data_mut(|data| {
                            *data.get_temp_mut_or_default(Id::new(uri)) = v.frame_durations.clone()
                        });
                    }
                    log::trace!("finished loading {uri:?}");
                    cache.insert(uri.into(), result.clone());
                    match result {
                        Ok(image) => Ok(ImagePoll::Ready {
                            image: image.get_image(index),
                        }),
                        Err(err) => Err(LoadError::Loading(err)),
                    }
                }
                Ok(BytesPoll::Pending { size }) => Ok(ImagePoll::Pending { size }),
                Err(err) => Err(err),
            }
        }
    }

    fn forget(&self, uri: &str) {
        let _ = self.cache.lock().remove(uri);
    }

    fn forget_all(&self) {
        self.cache.lock().clear();
    }

    fn byte_size(&self) -> usize {
        self.cache
            .lock()
            .values()
            .map(|v| match v {
                Ok(v) => v.byte_len(),
                Err(e) => e.len(),
            })
            .sum()
    }
}
