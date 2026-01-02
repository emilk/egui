use ahash::HashMap;
use egui::{
    ColorImage, FrameDurations, Id, decode_animated_image_uri, has_gif_magic_header,
    load::{BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
};
use image::AnimationDecoder as _;
use std::{io::Cursor, mem::size_of, sync::Arc, time::Duration};

/// Array of Frames and the duration for how long each frame should be shown
#[derive(Debug, Clone)]
pub struct AnimatedImage {
    frames: Vec<Arc<ColorImage>>,
    frame_durations: FrameDurations,
}

impl AnimatedImage {
    fn load_gif(data: &[u8]) -> Result<Self, String> {
        let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(data))
            .map_err(|err| format!("Failed to decode gif: {err}"))?;
        let mut images = vec![];
        let mut durations = vec![];
        for frame in decoder.into_frames() {
            let frame = frame.map_err(|err| format!("Failed to decode gif: {err}"))?;
            let img = frame.buffer();
            let pixels = img.as_flat_samples();

            let delay: Duration = frame.delay().into();
            images.push(Arc::new(ColorImage::from_rgba_unmultiplied(
                [img.width() as usize, img.height() as usize],
                pixels.as_slice(),
            )));
            durations.push(delay);
        }
        Ok(Self {
            frames: images,
            frame_durations: FrameDurations::new(durations),
        })
    }
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
        Arc::clone(&self.frames[index % self.frames.len()])
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

impl ImageLoader for GifLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, frame_uri: &str, _: SizeHint) -> ImageLoadResult {
        let (image_uri, frame_index) =
            decode_animated_image_uri(frame_uri).map_err(|_err| LoadError::NotSupported)?;
        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(image_uri).cloned() {
            match entry {
                Ok(image) => Ok(ImagePoll::Ready {
                    image: image.get_image(frame_index),
                }),
                Err(err) => Err(LoadError::Loading(err)),
            }
        } else {
            match ctx.try_load_bytes(image_uri) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    if !has_gif_magic_header(&bytes) {
                        return Err(LoadError::NotSupported);
                    }
                    log::trace!("started loading {image_uri:?}");
                    let result = AnimatedImage::load_gif(&bytes).map(Arc::new);
                    if let Ok(v) = &result {
                        ctx.data_mut(|data| {
                            *data.get_temp_mut_or_default(Id::new(image_uri)) =
                                v.frame_durations.clone();
                        });
                    }
                    log::trace!("finished loading {image_uri:?}");
                    cache.insert(image_uri.into(), result.clone());
                    match result {
                        Ok(image) => Ok(ImagePoll::Ready {
                            image: image.get_image(frame_index),
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
