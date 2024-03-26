//! # Image loading
//!
//! If you just want to display some images, [`egui_extras`](https://crates.io/crates/egui_extras/)
//! will get you up and running quickly with its reasonable default implementations of the traits described below.
//!
//! 1. Add [`egui_extras`](https://crates.io/crates/egui_extras/) as a dependency with the `all_loaders` feature.
//! 2. Add a call to [`egui_extras::install_image_loaders`](https://docs.rs/egui_extras/latest/egui_extras/fn.install_image_loaders.html)
//!    in your app's setup code.
//! 3. Use [`Ui::image`][`crate::ui::Ui::image`] with some [`ImageSource`][`crate::ImageSource`].
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
//! ui.image("file://image.png")
//! └► Context::try_load_texture
//! └► TextureLoader::load
//!    └► Context::try_load_image
//!    └► ImageLoader::load
//!       └► Context::try_load_bytes
//!       └► BytesLoader::load
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

mod bytes_loader;
mod texture_loader;

use std::borrow::Cow;
use std::fmt::Debug;
use std::ops::Deref;
use std::{fmt::Display, sync::Arc};

use ahash::HashMap;

use epaint::mutex::Mutex;
use epaint::util::FloatOrd;
use epaint::util::OrderedFloat;
use epaint::TextureHandle;
use epaint::{textures::TextureOptions, ColorImage, TextureId, Vec2};

use crate::Context;

pub use self::bytes_loader::DefaultBytesLoader;
pub use self::texture_loader::DefaultTextureLoader;

/// Represents a failed attempt at loading an image.
#[derive(Clone, Debug)]
pub enum LoadError {
    /// Programmer error: There are no image loaders installed.
    NoImageLoaders,

    /// A specific loader does not support this scheme, protocol or image format.
    NotSupported,

    /// Programmer error: Failed to find the bytes for this image because
    /// there was no [`BytesLoader`] supporting the scheme.
    NoMatchingBytesLoader,

    /// Programmer error: Failed to parse the bytes as an image because
    /// there was no [`ImageLoader`] supporting the scheme.
    NoMatchingImageLoader,

    /// Programmer error: no matching [`TextureLoader`].
    /// Because of the [`DefaultTextureLoader`], this error should never happen.
    NoMatchingTextureLoader,

    /// Runtime error: Loading was attempted, but failed (e.g. "File not found").
    Loading(String),
}

impl Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoImageLoaders => f.write_str(
                "No image loaders are installed. If you're trying to load some images \
                for the first time, follow the steps outlined in https://docs.rs/egui/latest/egui/load/index.html"),

            Self::NoMatchingBytesLoader => f.write_str("No matching BytesLoader. Either you need to call Context::include_bytes, or install some more bytes loaders, e.g. using egui_extras."),

            Self::NoMatchingImageLoader => f.write_str("No matching ImageLoader. Either you need to call Context::include_bytes, or install some more bytes loaders, e.g. using egui_extras."),

            Self::NoMatchingTextureLoader => f.write_str("No matching TextureLoader. Did you remove the default one?"),

            Self::NotSupported => f.write_str("Image scheme or URI not supported by this loader"),

            Self::Loading(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for LoadError {}

pub type Result<T, E = LoadError> = std::result::Result<T, E>;

/// Given as a hint for image loading requests.
///
/// Used mostly for rendering SVG:s to a good size.
///
/// All variants will preserve the original aspect ratio.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SizeHint {
    /// Scale original size by some factor.
    Scale(OrderedFloat<f32>),

    /// Scale to width.
    Width(u32),

    /// Scale to height.
    Height(u32),

    /// Scale to size.
    Size(u32, u32),
}

impl Default for SizeHint {
    #[inline]
    fn default() -> Self {
        Self::Scale(1.0.ord())
    }
}

impl From<Vec2> for SizeHint {
    #[inline]
    fn from(value: Vec2) -> Self {
        Self::Size(value.x.round() as u32, value.y.round() as u32)
    }
}

/// Represents a byte buffer.
///
/// This is essentially `Cow<'static, [u8]>` but with the `Owned` variant being an `Arc`.
#[derive(Clone)]
pub enum Bytes {
    Static(&'static [u8]),
    Shared(Arc<[u8]>),
}

impl Debug for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(arg0) => f.debug_tuple("Static").field(&arg0.len()).finish(),
            Self::Shared(arg0) => f.debug_tuple("Shared").field(&arg0.len()).finish(),
        }
    }
}

impl From<&'static [u8]> for Bytes {
    #[inline]
    fn from(value: &'static [u8]) -> Self {
        Self::Static(value)
    }
}

impl<const N: usize> From<&'static [u8; N]> for Bytes {
    #[inline]
    fn from(value: &'static [u8; N]) -> Self {
        Self::Static(value)
    }
}

impl From<Arc<[u8]>> for Bytes {
    #[inline]
    fn from(value: Arc<[u8]>) -> Self {
        Self::Shared(value)
    }
}

impl From<Vec<u8>> for Bytes {
    #[inline]
    fn from(value: Vec<u8>) -> Self {
        Self::Shared(value.into())
    }
}

impl AsRef<[u8]> for Bytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Static(bytes) => bytes,
            Self::Shared(bytes) => bytes,
        }
    }
}

impl Deref for Bytes {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

/// Represents bytes which are currently being loaded.
///
/// This is similar to [`std::task::Poll`], but the `Pending` variant
/// contains an optional `size`, which may be used during layout to
/// pre-allocate space the image.
#[derive(Clone)]
pub enum BytesPoll {
    /// Bytes are being loaded.
    Pending {
        /// Set if known (e.g. from a HTTP header, or by parsing the image file header).
        size: Option<Vec2>,
    },

    /// Bytes are loaded.
    Ready {
        /// Set if known (e.g. from a HTTP header, or by parsing the image file header).
        size: Option<Vec2>,

        /// File contents, e.g. the contents of a `.png`.
        bytes: Bytes,

        /// Mime type of the content, e.g. `image/png`.
        ///
        /// Set if known (e.g. from `Content-Type` HTTP header).
        mime: Option<String>,
    },
}

/// Used to get a unique ID when implementing one of the loader traits: [`BytesLoader::id`], [`ImageLoader::id`], and [`TextureLoader::id`].
///
/// This just expands to `module_path!()` concatenated with the given type name.
#[macro_export]
macro_rules! generate_loader_id {
    ($ty:ident) => {
        concat!(module_path!(), "::", stringify!($ty))
    };
}
pub use crate::generate_loader_id;

pub type BytesLoadResult = Result<BytesPoll>;

/// Represents a loader capable of loading raw unstructured bytes from somewhere,
/// e.g. from disk or network.
///
/// It should also provide any subsequent loaders a hint for what the bytes may
/// represent using [`BytesPoll::Ready::mime`], if it can be inferred.
///
/// Implementations are expected to cache at least each `URI`.
pub trait BytesLoader {
    /// Unique ID of this loader.
    ///
    /// To reduce the chance of collisions, use [`generate_loader_id`] for this.
    fn id(&self) -> &str;

    /// Try loading the bytes from the given uri.
    ///
    /// Implementations should call `ctx.request_repaint` to wake up the ui
    /// once the data is ready.
    ///
    /// The implementation should cache any result, so that calling this
    /// is immediate-mode safe.
    ///
    /// # Errors
    /// This may fail with:
    /// - [`LoadError::NotSupported`] if the loader does not support loading `uri`.
    /// - [`LoadError::Loading`] if the loading process failed.
    fn load(&self, ctx: &Context, uri: &str) -> BytesLoadResult;

    /// Forget the given `uri`.
    ///
    /// If `uri` is cached, it should be evicted from cache,
    /// so that it may be fully reloaded.
    fn forget(&self, uri: &str);

    /// Forget all URIs ever given to this loader.
    ///
    /// If the loader caches any URIs, the entire cache should be cleared,
    /// so that all of them may be fully reloaded.
    fn forget_all(&self);

    /// Implementations may use this to perform work at the end of a frame,
    /// such as evicting unused entries from a cache.
    fn end_frame(&self, frame_index: usize) {
        let _ = frame_index;
    }

    /// If the loader caches any data, this should return the size of that cache.
    fn byte_size(&self) -> usize;
}

/// Represents an image which is currently being loaded.
///
/// This is similar to [`std::task::Poll`], but the `Pending` variant
/// contains an optional `size`, which may be used during layout to
/// pre-allocate space the image.
#[derive(Clone)]
pub enum ImagePoll {
    /// Image is loading.
    Pending {
        /// Set if known (e.g. from a HTTP header, or by parsing the image file header).
        size: Option<Vec2>,
    },

    /// Image is loaded.
    Ready { image: Arc<ColorImage> },
}

pub type ImageLoadResult = Result<ImagePoll>;

/// An `ImageLoader` decodes raw bytes into a [`ColorImage`].
///
/// Implementations are expected to cache at least each `URI`.
pub trait ImageLoader {
    /// Unique ID of this loader.
    ///
    /// To reduce the chance of collisions, include `module_path!()` as part of this ID.
    ///
    /// For example: `concat!(module_path!(), "::MyLoader")`
    /// for `my_crate::my_loader::MyLoader`.
    fn id(&self) -> &str;

    /// Try loading the image from the given uri.
    ///
    /// Implementations should call `ctx.request_repaint` to wake up the ui
    /// once the image is ready.
    ///
    /// The implementation should cache any result, so that calling this
    /// is immediate-mode safe.
    ///
    /// # Errors
    /// This may fail with:
    /// - [`LoadError::NotSupported`] if the loader does not support loading `uri`.
    /// - [`LoadError::Loading`] if the loading process failed.
    fn load(&self, ctx: &Context, uri: &str, size_hint: SizeHint) -> ImageLoadResult;

    /// Forget the given `uri`.
    ///
    /// If `uri` is cached, it should be evicted from cache,
    /// so that it may be fully reloaded.
    fn forget(&self, uri: &str);

    /// Forget all URIs ever given to this loader.
    ///
    /// If the loader caches any URIs, the entire cache should be cleared,
    /// so that all of them may be fully reloaded.
    fn forget_all(&self);

    /// Implementations may use this to perform work at the end of a frame,
    /// such as evicting unused entries from a cache.
    fn end_frame(&self, frame_index: usize) {
        let _ = frame_index;
    }

    /// If the loader caches any data, this should return the size of that cache.
    fn byte_size(&self) -> usize;
}

/// A texture with a known size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SizedTexture {
    pub id: TextureId,
    pub size: Vec2,
}

impl SizedTexture {
    /// Create a [`SizedTexture`] from a texture `id` with a specific `size`.
    pub fn new(id: impl Into<TextureId>, size: impl Into<Vec2>) -> Self {
        Self {
            id: id.into(),
            size: size.into(),
        }
    }

    /// Fetch the [id][`SizedTexture::id`] and [size][`SizedTexture::size`] from a [`TextureHandle`].
    pub fn from_handle(handle: &TextureHandle) -> Self {
        let size = handle.size();
        Self {
            id: handle.id(),
            size: Vec2::new(size[0] as f32, size[1] as f32),
        }
    }
}

impl From<(TextureId, Vec2)> for SizedTexture {
    #[inline]
    fn from((id, size): (TextureId, Vec2)) -> Self {
        Self { id, size }
    }
}

impl<'a> From<&'a TextureHandle> for SizedTexture {
    #[inline]
    fn from(handle: &'a TextureHandle) -> Self {
        Self::from_handle(handle)
    }
}

/// Represents a texture is currently being loaded.
///
/// This is similar to [`std::task::Poll`], but the `Pending` variant
/// contains an optional `size`, which may be used during layout to
/// pre-allocate space the image.
#[derive(Clone, Copy)]
pub enum TexturePoll {
    /// Texture is loading.
    Pending {
        /// Set if known (e.g. from a HTTP header, or by parsing the image file header).
        size: Option<Vec2>,
    },

    /// Texture is loaded.
    Ready { texture: SizedTexture },
}

impl TexturePoll {
    #[inline]
    pub fn size(&self) -> Option<Vec2> {
        match self {
            Self::Pending { size } => *size,
            Self::Ready { texture } => Some(texture.size),
        }
    }

    #[inline]
    pub fn texture_id(&self) -> Option<TextureId> {
        match self {
            Self::Pending { .. } => None,
            Self::Ready { texture } => Some(texture.id),
        }
    }
}

pub type TextureLoadResult = Result<TexturePoll>;

/// A `TextureLoader` uploads a [`ColorImage`] to the GPU, returning a [`SizedTexture`].
///
/// `egui` comes with an implementation that uses [`Context::load_texture`],
/// which just asks the egui backend to upload the image to the GPU.
///
/// You can implement this trait if you do your own uploading of images to the GPU.
/// For instance, you can use this to refer to textures in a game engine that egui
/// doesn't otherwise know about.
///
/// Implementations are expected to cache each combination of `(URI, TextureOptions)`.
pub trait TextureLoader {
    /// Unique ID of this loader.
    ///
    /// To reduce the chance of collisions, include `module_path!()` as part of this ID.
    ///
    /// For example: `concat!(module_path!(), "::MyLoader")`
    /// for `my_crate::my_loader::MyLoader`.
    fn id(&self) -> &str;

    /// Try loading the texture from the given uri.
    ///
    /// Implementations should call `ctx.request_repaint` to wake up the ui
    /// once the texture is ready.
    ///
    /// The implementation should cache any result, so that calling this
    /// is immediate-mode safe.
    ///
    /// # Errors
    /// This may fail with:
    /// - [`LoadError::NotSupported`] if the loader does not support loading `uri`.
    /// - [`LoadError::Loading`] if the loading process failed.
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

    /// Forget all URIs ever given to this loader.
    ///
    /// If the loader caches any URIs, the entire cache should be cleared,
    /// so that all of them may be fully reloaded.
    fn forget_all(&self);

    /// Implementations may use this to perform work at the end of a frame,
    /// such as evicting unused entries from a cache.
    fn end_frame(&self, frame_index: usize) {
        let _ = frame_index;
    }

    /// If the loader caches any data, this should return the size of that cache.
    fn byte_size(&self) -> usize;
}

type BytesLoaderImpl = Arc<dyn BytesLoader + Send + Sync + 'static>;
type ImageLoaderImpl = Arc<dyn ImageLoader + Send + Sync + 'static>;
type TextureLoaderImpl = Arc<dyn TextureLoader + Send + Sync + 'static>;

#[derive(Clone)]
/// The loaders of bytes, images, and textures.
pub struct Loaders {
    pub include: Arc<DefaultBytesLoader>,
    pub bytes: Mutex<Vec<BytesLoaderImpl>>,
    pub image: Mutex<Vec<ImageLoaderImpl>>,
    pub texture: Mutex<Vec<TextureLoaderImpl>>,
}

impl Default for Loaders {
    fn default() -> Self {
        let include = Arc::new(DefaultBytesLoader::default());
        Self {
            bytes: Mutex::new(vec![include.clone()]),
            image: Mutex::new(Vec::new()),
            // By default we only include `DefaultTextureLoader`.
            texture: Mutex::new(vec![Arc::new(DefaultTextureLoader::default())]),
            include,
        }
    }
}
