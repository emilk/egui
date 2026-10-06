use core::{mem::size_of, task::Poll, time::Duration};
use egui::{
    ColorImage, FrameDurations, Id, decode_animated_image_uri, has_webp_header,
    load::{BytesPoll, ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint},
};
use image::{AnimationDecoder as _, ColorType, ImageDecoder as _, Rgba, codecs::webp::WebPDecoder};
use std::{io::Cursor, sync::Arc};

use super::background_decode::DecodeCache;

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

pub struct WebPLoader {
    cache: DecodeCache<WebP>,
}

impl Default for WebPLoader {
    fn default() -> Self {
        Self {
            cache: DecodeCache::new("WebPLoader"),
        }
    }
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

        let entry = match self.cache.get(image_uri) {
            Some(entry) => entry,
            None => match ctx.try_load_bytes(image_uri) {
                Ok(BytesPoll::Ready { bytes, .. }) => {
                    if !has_webp_header(&bytes) {
                        return Err(LoadError::NotSupported);
                    }
                    let ctx_clone = ctx.clone();
                    let id = Id::unique(image_uri);
                    self.cache.decode(ctx, image_uri, &bytes, move |bytes| {
                        let result = WebP::load(bytes);
                        // Store the frame durations before the image is marked as ready,
                        // so that an animated WebP is never shown without its durations.
                        if let Ok(WebP::Animated(animated_image)) = &result {
                            ctx_clone.data_mut(|data| {
                                *data.get_temp_mut_or_default(id) =
                                    animated_image.frame_durations.clone();
                            });
                        }
                        result
                    })
                }
                Ok(BytesPoll::Pending { size }) => return Ok(ImagePoll::Pending { size }),
                Err(error) => return Err(error),
            },
        };

        match entry {
            Poll::Ready(Ok(image)) => Ok(ImagePoll::Ready {
                image: image.get_image(frame_index),
            }),
            Poll::Ready(Err(error)) => Err(LoadError::Loading(error)),
            Poll::Pending => Ok(ImagePoll::Pending { size: None }),
        }
    }

    fn forget(&self, uri: &str) {
        self.cache.forget(uri);
    }

    fn forget_all(&self) {
        self.cache.forget_all();
    }

    fn byte_size(&self) -> usize {
        self.cache.byte_size(WebP::byte_len)
    }

    fn has_pending(&self) -> bool {
        self.cache.has_pending()
    }
}
