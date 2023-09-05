//! Types and traits related to image loading.
//!
//! If you just want to load some images, see [`egui_extras`](https://crates.io/crates/egui_extras/),
//! which contains reasonable default implementations of these traits. You can get started quickly
//! using [`egui_extras::loaders::install`](https://docs.rs/egui_extras/latest/egui_extras/loaders/fn.install.html).
//!
//! ## Loading process
//!
//! There are three kinds of loaders:
//! - [`BytesLoader`]: load the raw bytes of an image
//! - [`ImageLoader`]: decode the bytes into an array of colors
//! - [`TextureLoader`]: ask the backend to put an image onto the GPU
//!
//! The different kinds of loaders represent different layers in the loading process:
//!
//! ```text,ignore
//! ui.image2("file://image.png")
//! └► ctx.try_load_texture("file://image.png", ...)
//! └► TextureLoader::load("file://image.png", ...)
//!    └► ctx.try_load_image("file://image.png", ...)
//!    └► ImageLoader::load("file://image.png", ...)
//!       └► ctx.try_load_bytes("file://image.png", ...)
//!       └► BytesLoader::load("file://image.png", ...)
//! ```
//!
//! As each layer attempts to load the URI, it first asks the layer below it
//! for the data it needs to do its job. But this is not a strict requirement,
//! an implementation could instead generate the data it needs!
//!
//! Loader trait implementations may be registered on a context with:
//! - [`Context::add_bytes_loader`]
//! - [`Context::add_image_loader`]
//! - [`Context::add_texture_loader`]
//!
//! There may be multiple loaders of the same kind registered at the same time.
//! The `try_load` methods on [`Context`] will attempt to call each loader one by one,
//! until one of them returns something other than [`LoadError::NotSupported`].
//!
//! The loaders are stored in the context. This means they may hold state across frames,
//! which they can (and _should_) use to cache the results of the operations they perform.
//!
//! For example, a [`BytesLoader`] that loads file URIs (`file://image.png`)
//! would cache each file read. A [`TextureLoader`] would cache each combination
//! of `(URI, TextureOptions)`, and so on.
//!
//! Each URI will be passed through the loaders as a plain `&str`.
//! The loaders are free to derive as much meaning from the URI as they wish to.
//! For example, a loader may determine that it doesn't support loading a specific URI
//! if the protocol does not match what it expects.

use crate::Context;
use ahash::HashMap;
use epaint::mutex::Mutex;
use epaint::{textures::TextureOptions, ColorImage, TextureId, Vec2};
use std::{error::Error as StdError, fmt::Display, sync::Arc};

#[derive(Clone, Debug)]
pub enum LoadError {
    /// This loader does not support this protocol or image format.
    NotSupported,

    /// A custom error message (e.g. "File not found: foo.png").
    Custom(String),
}

impl Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::NotSupported => f.write_str("not supported"),
            LoadError::Custom(message) => f.write_str(message),
        }
    }
}

impl StdError for LoadError {}

pub type Result<T, E = LoadError> = std::result::Result<T, E>;

/// Given as a hint for image loading requests.
///
/// Used mostly for rendering SVG:s to a good size.
///
/// All variants will preserve the original aspect ratio.
///
/// Similar to `usvg::FitTo`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SizeHint {
    /// Keep original size.
    Original,

    /// Scale to width.
    Width(u32),

    /// Scale to height.
    Height(u32),

    /// Scale to size.
    Size(u32, u32),
}

impl From<Vec2> for SizeHint {
    fn from(value: Vec2) -> Self {
        Self::Size(value.x.round() as u32, value.y.round() as u32)
    }
}

// TODO: API for querying bytes caches in each loader

pub type Size = [usize; 2];

#[derive(Clone)]
pub enum BytesPoll {
    /// Bytes are being loaded.
    Pending {
        /// Set if known (e.g. from a HTTP header, or by parsing the image file header).
        size: Option<Size>,
    },

    /// Bytes are loaded.
    Ready {
        /// Set if known (e.g. from a HTTP header, or by parsing the image file header).
        size: Option<Size>,

        /// File contents, e.g. the contents of a `.png`.
        bytes: Arc<[u8]>,
    },
}

pub type BytesLoadResult = Result<BytesPoll>;

pub trait BytesLoader {
    /// Try loading the bytes from the given uri.
    ///
    /// Implementations should call `ctx.request_repaint` to wake up the ui
    /// once the data is ready.
    ///
    /// # Errors
    /// This may fail with:
    /// - [`LoadError::NotSupported`] if the loader does not support loading `uri`.
    /// - [`LoadError::Custom`] if the loading process failed.
    fn load(&self, ctx: &Context, uri: &str) -> BytesLoadResult;

    /// Forget the given `uri`.
    ///
    /// If `uri` is cached, it should be evicted from cache,
    /// so that it may be fully reloaded.
    fn forget(&self, uri: &str);

    /// Implementations may use this to perform work at the end of a frame,
    /// such as evicting unused entries from a cache.
    fn end_frame(&self, frame_index: usize) {
        let _ = frame_index;
    }

    /// If the loader caches any data, this should return the size of that cache.
    fn byte_size(&self) -> usize;
}

#[derive(Clone)]
pub enum ImagePoll {
    /// Image is loading.
    Pending {
        /// Set if known (e.g. from a HTTP header, or by parsing the image file header).
        size: Option<Size>,
    },

    /// Image is loaded.
    Ready { image: Arc<ColorImage> },
}

pub type ImageLoadResult = Result<ImagePoll>;

pub trait ImageLoader {
    /// Try loading the image from the given uri.
    ///
    /// Implementations should call `ctx.request_repaint` to wake up the ui
    /// once the image is ready.
    ///
    /// # Errors
    /// This may fail with:
    /// - [`LoadError::NotSupported`] if the loader does not support loading `uri`.
    /// - [`LoadError::Custom`] if the loading process failed.
    fn load(&self, ctx: &Context, uri: &str, size_hint: SizeHint) -> ImageLoadResult;

    /// Forget the given `uri`.
    ///
    /// If `uri` is cached, it should be evicted from cache,
    /// so that it may be fully reloaded.
    fn forget(&self, uri: &str);

    /// Implementations may use this to perform work at the end of a frame,
    /// such as evicting unused entries from a cache.
    fn end_frame(&self, frame_index: usize) {
        let _ = frame_index;
    }

    /// If the loader caches any data, this should return the size of that cache.
    fn byte_size(&self) -> usize;
}

/// A texture with a known size.
#[derive(Clone)]
pub struct SizedTexture {
    pub id: TextureId,
    pub size: Size,
}

#[derive(Clone)]
pub enum TexturePoll {
    /// Texture is loading.
    Pending {
        /// Set if known (e.g. from a HTTP header, or by parsing the image file header).
        size: Option<Size>,
    },

    /// Texture is loaded.
    Ready { texture: SizedTexture },
}

pub type TextureLoadResult = Result<TexturePoll>;

pub trait TextureLoader {
    /// Try loading the texture from the given uri.
    ///
    /// Implementations should call `ctx.request_repaint` to wake up the ui
    /// once the texture is ready.
    ///
    /// # Errors
    /// This may fail with:
    /// - [`LoadError::NotSupported`] if the loader does not support loading `uri`.
    /// - [`LoadError::Custom`] if the loading process failed.
    fn load(
        &self,
        ctx: &Context,
        uri: &str,
        texture_options: TextureOptions,
        size_hint: SizeHint,
    ) -> TextureLoadResult;

    /// Forget the given `uri`.
    ///
    /// If `uri` is cached, it should be evicted from cache,
    /// so that it may be fully reloaded.
    fn forget(&self, uri: &str);

    /// Implementations may use this to perform work at the end of a frame,
    /// such as evicting unused entries from a cache.
    fn end_frame(&self, frame_index: usize) {
        let _ = frame_index;
    }

    /// If the loader caches any data, this should return the size of that cache.
    fn byte_size(&self) -> usize;
}

#[derive(Default)]
pub(crate) struct IncludeBytesLoader {
    cache: Mutex<HashMap<&'static str, Arc<[u8]>>>,
}

impl IncludeBytesLoader {
    pub(crate) fn insert(&self, name: &'static str, bytes: &'static [u8]) {
        self.cache
            .lock()
            .entry(name)
            .or_insert_with(|| bytes.into());
    }
}

impl BytesLoader for IncludeBytesLoader {
    fn load(&self, _: &Context, uri: &str) -> BytesLoadResult {
        match self.cache.lock().get(uri).cloned() {
            Some(bytes) => Ok(BytesPoll::Ready { size: None, bytes }),
            None => Err(LoadError::NotSupported),
        }
    }

    fn forget(&self, uri: &str) {
        let _ = self.cache.lock().remove(uri);
    }

    fn byte_size(&self) -> usize {
        self.cache.lock().values().map(|bytes| bytes.len()).sum()
    }
}

struct DefaultTextureLoader;

impl TextureLoader for DefaultTextureLoader {
    fn load(
        &self,
        ctx: &Context,
        uri: &str,
        texture_options: TextureOptions,
        size_hint: SizeHint,
    ) -> TextureLoadResult {
        match ctx.try_load_image(uri, size_hint)? {
            ImagePoll::Pending { size } => Ok(TexturePoll::Pending { size }),
            ImagePoll::Ready { image } => {
                let handle = ctx.load_texture(uri, image, texture_options);
                let texture = SizedTexture {
                    id: handle.id(),
                    size: handle.size(),
                };
                Ok(TexturePoll::Ready { texture })
            }
        }
    }

    fn forget(&self, _: &str) {
        // This loader never evicts any data
    }

    fn end_frame(&self, _: usize) {
        // This loader never evicts any data
    }

    fn byte_size(&self) -> usize {
        0
    }
}

pub(crate) struct Loaders {
    pub include: Arc<IncludeBytesLoader>,
    pub bytes: Vec<Arc<dyn BytesLoader + Send + Sync + 'static>>,
    pub image: Vec<Arc<dyn ImageLoader + Send + Sync + 'static>>,
    pub texture: Vec<Arc<dyn TextureLoader + Send + Sync + 'static>>,
}

impl Default for Loaders {
    fn default() -> Self {
        let include = Arc::new(IncludeBytesLoader::default());
        Self {
            bytes: vec![include.clone()],
            image: Vec::new(),
            // By default we only include `DefaultTextureLoader`.
            texture: vec![Arc::new(DefaultTextureLoader)],
            include,
        }
    }
}
