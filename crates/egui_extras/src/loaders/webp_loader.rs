use ahash::HashMap;
use core::{mem::size_of, task::Poll, time::Duration};
use egui::{
    ColorImage, FrameDurations, Id, decode_animated_image_uri, has_webp_header,
    load::{Bytes, BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
    mutex::Mutex,
};
use image::{AnimationDecoder as _, ColorType, ImageDecoder as _, Rgba, codecs::webp::WebPDecoder};
use std::{io::Cursor, sync::Arc};

#[cfg(not(target_arch = "wasm32"))]
use std::thread;

#[derive(Clone)]
enum WebP {
    Static(Arc<ColorImage>),
    Animated(AnimatedImage),
}

impl WebP {
    fn load(data: &[u8]) -> Result<Self, String> {
        let mut decoder = WebPDecoder::new(Cursor::new(data))
            .map_err(|error| format!("WebP decode failure ({error})"))?;

        if decoder.has_animation() {
            decoder
                .set_background_color(Rgba([0, 0, 0, 0]))
                .map_err(|error| {
                    format!("Failure to set default background color for animated WebP ({error})")
                })?;

            let mut images = vec![];
            let mut durations = vec![];

            for frame in decoder.into_frames() {
                let frame =
                    frame.map_err(|error| format!("WebP frame decode failure ({error})"))?;
                let image = frame.buffer();
                let pixels = image.as_flat_samples();

                images.push(Arc::new(ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize],
                    pixels.as_slice(),
                )));

                let delay: Duration = frame.delay().into();
                durations.push(delay);
            }
            Ok(Self::Animated(AnimatedImage {
                frames: images,
                frame_durations: FrameDurations::new(durations),
            }))
        } else {
            // color_type() of WebPDecoder only returns Rgb8/Rgba8 variants of ColorType
            let create_image = match decoder.color_type() {
                ColorType::Rgb8 => ColorImage::from_rgb,
                ColorType::Rgba8 => ColorImage::from_rgba_unmultiplied,
                unreachable => {
                    return Err(format!(
                        "Unreachable WebP color type, expected Rgb8/Rgba8, got {unreachable:?}"
                    ));
                }
            };

            let (width, height) = decoder.dimensions();
            let size = decoder.total_bytes() as usize;

            let mut data = vec![0; size];
            decoder
                .read_image(&mut data)
                .map_err(|error| format!("WebP image read failure ({error})"))?;

            Ok(Self::Static(Arc::new(create_image(
                [width as usize, height as usize],
                &data,
            ))))
        }
    }

    fn get_image(&self, frame_index: usize) -> Arc<ColorImage> {
        match self {
            Self::Static(image) => Arc::clone(image),
            Self::Animated(animation) => animation.get_image_by_index(frame_index),
        }
    }

    pub fn byte_len(&self) -> usize {
        size_of::<Self>()
            + match self {
                Self::Static(image) => image.pixels.len() * size_of::<egui::Color32>(),
                Self::Animated(animation) => animation.byte_len(),
            }
    }
}

#[derive(Debug, Clone)]
pub struct AnimatedImage {
    frames: Vec<Arc<ColorImage>>,
    frame_durations: FrameDurations,
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

    pub fn get_image_by_index(&self, index: usize) -> Arc<ColorImage> {
        Arc::clone(&self.frames[index % self.frames.len()])
    }
}

fn store_frame_durations(ctx: &egui::Context, image_uri: &str, frame_durations: FrameDurations) {
    ctx.data_mut(|data| {
        *data.get_temp_mut_or_default(Id::new(image_uri)) = frame_durations;
    });
}

type Entry = Poll<Result<WebP, String>>;

#[derive(Default)]
pub struct WebPLoader {
    cache: Arc<Mutex<HashMap<String, Entry>>>,
}

impl WebPLoader {
    pub const ID: &'static str = egui::generate_loader_id!(WebPLoader);
}

impl ImageLoader for WebPLoader {
    fn id(&self) -> &str {
        Self::ID
    }

    fn load(&self, ctx: &egui::Context, frame_uri: &str, _: SizeHint) -> ImageLoadResult {
        let (image_uri, frame_index) =
            decode_animated_image_uri(frame_uri).map_err(|_error| LoadError::NotSupported)?;

        #[cfg(not(target_arch = "wasm32"))]
        #[expect(clippy::unnecessary_wraps)] // needed here to match other return types
        fn load_image(
            ctx: &egui::Context,
            image_uri: &str,
            _frame_index: usize,
            cache: &Arc<Mutex<HashMap<String, Entry>>>,
            bytes: &Bytes,
        ) -> ImageLoadResult {
            let image_uri = image_uri.to_owned();
            cache.lock().insert(image_uri.clone(), Poll::Pending);

            // Do the image parsing on a bg thread
            thread::Builder::new()
                .name(format!("egui_extras::WebPLoader::load({image_uri:?}"))
                .spawn({
                    let ctx = ctx.clone();
                    let cache = Arc::clone(cache);
                    let bytes = bytes.clone();
                    move || {
                        log::trace!("WebPLoader - started loading {image_uri:?}");
                        let result = WebP::load(&bytes);
                        let frame_durations = match &result {
                            Ok(WebP::Animated(animated_image)) => {
                                Some(animated_image.frame_durations.clone())
                            }
                            _ => None,
                        };
                        let found = {
                            let mut cache = cache.lock();

                            if let std::collections::hash_map::Entry::Occupied(mut entry) =
                                cache.entry(image_uri.clone())
                            {
                                let entry = entry.get_mut();
                                *entry = Poll::Ready(result);
                                log::trace!("WebPLoader - finished loading {image_uri:?}");
                                true
                            } else {
                                log::trace!("WebPLoader - canceled loading {image_uri:?}\nNote: This can happen if `forget_image` is called while the image is still loading");
                                false
                            }
                        };
                        // We may not lock Context while the cache lock is held, since this can
                        // deadlock.
                        // Example deadlock scenario:
                        // - loader thread: lock cache
                        // - main thread: lock ctx (e.g. in `Context::has_pending_images`)
                        // - loader thread: try to lock ctx (in `request_repaint`)
                        // - main thread: try to lock cache (from `Self::has_pending`)
                        if found {
                            if let Some(frame_durations) = frame_durations {
                                store_frame_durations(&ctx, &image_uri, frame_durations);
                            }
                            ctx.request_repaint();
                        }
                    }
                })
                .expect("failed to spawn thread");

            Ok(ImagePoll::Pending { size: None })
        }

        #[cfg(target_arch = "wasm32")]
        fn load_image(
            ctx: &egui::Context,
            image_uri: &str,
            frame_index: usize,
            cache: &Arc<Mutex<HashMap<String, Entry>>>,
            bytes: &Bytes,
        ) -> ImageLoadResult {
            log::trace!("WebPLoader - started loading {image_uri:?}");

            let result = WebP::load(bytes);

            if let Ok(WebP::Animated(animated_image)) = &result {
                store_frame_durations(ctx, image_uri, animated_image.frame_durations.clone());
            }

            log::trace!("WebPLoader - finished loading {image_uri:?}");

            let image_result = match &result {
                Ok(image) => Ok(ImagePoll::Ready {
                    image: image.get_image(frame_index),
                }),
                Err(error) => Err(LoadError::Loading(error.clone())),
            };
            cache.lock().insert(image_uri.into(), Poll::Ready(result));
            image_result
        }

        let entry = self.cache.lock().get(image_uri).cloned();
        if let Some(entry) = entry {
            match entry {
                Poll::Ready(res) => match res {
                    Ok(image) => Ok(ImagePoll::Ready {
                        image: image.get_image(frame_index),
                    }),
                    Err(error) => Err(LoadError::Loading(error)),
                },
                Poll::Pending => Ok(ImagePoll::Pending { size: None }),
            }
        } else {
            match ctx.try_load_bytes(image_uri) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    if !has_webp_header(&bytes) {
                        return Err(LoadError::NotSupported);
                    }
                    load_image(ctx, image_uri, frame_index, &self.cache, &bytes)
                }
                Ok(BytesPoll::Pending { size }) => Ok(ImagePoll::Pending { size }),
                Err(error) => Err(error),
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
            .map(|entry| match entry {
                Poll::Ready(res) => match res {
                    Ok(entry_value) => entry_value.byte_len(),
                    Err(error) => error.len(),
                },
                Poll::Pending => 0,
            })
            .sum()
    }

    fn has_pending(&self) -> bool {
        self.cache.lock().values().any(|result| result.is_pending())
    }
}
