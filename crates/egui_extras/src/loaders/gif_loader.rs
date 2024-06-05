use egui::{
    ahash::HashMap,
    load::{Bytes, BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
    ColorImage, Id,
};
use image::AnimationDecoder as _;
use std::{io::Cursor, mem::size_of, sync::Arc, time::Duration};

/// Array of Frames and the duration for how long each frame should be shown
#[derive(Debug, Clone)]
pub struct AnimatedImage {
    frames: Vec<(Arc<ColorImage>, usize, Duration)>,
}

impl AnimatedImage {
    pub fn byte_len(&self) -> usize {
        size_of::<Self>()
            + self
                .frames
                .iter()
                .map(|(image, _, _)| {
                    image.pixels.len() * size_of::<egui::Color32>()
                        + size_of::<usize>()
                        + size_of::<Duration>()
                })
                .sum::<usize>()
    }

    /// Gets image at index
    pub fn get_image(&self, index: usize) -> Arc<ColorImage> {
        self.frames
            .get(index % self.frames.len())
            .cloned()
            .unwrap()
            .0
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

fn is_supported_uri(uri: &str) -> bool {
    uri.starts_with("gif://")
}

pub fn gif_to_sources(data: Bytes) -> Result<AnimatedImage, String> {
    let decoder = image::codecs::gif::GifDecoder::new(Cursor::new(data))
        .map_err(|_err| "Couldnt decode gif".to_owned())?;
    let mut res = vec![];
    for (index, frame) in decoder.into_frames().enumerate() {
        let frame = frame.map_err(|_err| "Couldnt decode gif".to_owned())?;
        let img = frame.buffer();
        let pixels = img.as_flat_samples();

        let delay: std::time::Duration = frame.delay().into();

        res.push((
            Arc::new(ColorImage::from_rgba_unmultiplied(
                [img.width() as usize, img.height() as usize],
                pixels.as_slice(),
            )),
            index,
            delay,
        ));
    }
    Ok(AnimatedImage { frames: res })
}

impl ImageLoader for GifLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, uri_data: &str, _: SizeHint) -> ImageLoadResult {
        if !is_supported_uri(uri_data) {
            return Err(LoadError::NotSupported);
        }
        let (uri, index) = uri_data
            .rsplit_once('-')
            .ok_or(LoadError::Loading("No -{index} at end of uri".to_owned()))?;
        let index: usize = index
            .parse()
            .map_err(|_err| LoadError::Loading("Failed to parse index".to_owned()))?;
        let mut cache = self.cache.lock();
        if let Some(entry) = cache.get(uri).cloned() {
            match entry {
                Ok(image) => Ok(ImagePoll::Ready {
                    image: image.get_image(index),
                }),
                Err(err) => Err(LoadError::Loading(err)),
            }
        } else {
            match ctx.try_load_bytes(uri_data) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    log::trace!("started loading {uri:?}");
                    let result = gif_to_sources(bytes).map(Arc::new);
                    if let Ok(v) = &result {
                        let v = v.frames.iter().map(|v| v.2).collect::<Vec<_>>();
                        ctx.data_mut(|data| {
                            *data.get_temp_mut_or_default(Id::new(format!("{uri}-index"))) = v;
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
