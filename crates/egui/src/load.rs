use crate::{ahash, Context};
use epaint::{mutex::RwLock, textures::TextureOptions, ColorImage, TextureId, Vec2};
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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        Self::Size(value.x as u32, value.y as u32)
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
}

#[derive(Default)]
pub struct StaticBytesLoader {
    data: RwLock<ahash::HashMap<String, Arc<[u8]>>>,
}

impl StaticBytesLoader {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(ahash::HashMap::default()),
        }
    }

    pub fn add(&self, id: impl Into<String>, bytes: &'static [u8]) {
        let _ = self.data.write().insert(id.into(), bytes.into());
    }
}

impl BytesLoader for StaticBytesLoader {
    fn load(&self, _: &Context, uri: &str) -> BytesLoadResult {
        match self.data.read().get(uri).cloned() {
            Some(bytes) => Ok(BytesPoll::Ready { size: None, bytes }),
            // The loading didn't actually "fail", we just couldn't find the data.
            // Let a different bytes loader attempt to load `uri`.
            None => Err(LoadError::NotSupported),
        }
    }

    fn forget(&self, _: &str) {
        // This loader never evicts any data
    }

    fn end_frame(&self, _: usize) {
        // This loader never evicts any data
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
}

pub(crate) struct Loaders {
    pub bytes: Vec<Arc<dyn BytesLoader + Send + Sync + 'static>>,
    pub image: Vec<Arc<dyn ImageLoader + Send + Sync + 'static>>,
    pub texture: Vec<Arc<dyn TextureLoader + Send + Sync + 'static>>,
}

impl Default for Loaders {
    fn default() -> Self {
        Self {
            bytes: Vec::new(),
            image: Vec::new(),
            // By default we only include `DefaultTextureLoader`.
            texture: vec![Arc::new(DefaultTextureLoader)],
        }
    }
}
