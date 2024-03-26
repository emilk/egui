// TODO(jprochazk): automatic cache eviction

/// Installs a set of image loaders.
///
/// Calling this enables the use of [`egui::Image`] and [`egui::Ui::image`].
///
/// ⚠ This will do nothing and you won't see any images unless you also enable some feature flags on `egui_extras`:
///
/// - `file` feature: `file://` loader on non-Wasm targets
/// - `http` feature: `http(s)://` loader
/// - `image` feature: Loader of png, jpeg etc using the [`image`] crate
/// - `svg` feature: `.svg` loader
///
/// Calling this multiple times on the same [`egui::Context`] is safe.
/// It will never install duplicate loaders.
///
/// - If you just want to be able to load `file://` and `http://` URIs, enable the `all_loaders` feature.
/// - The supported set of image formats is configured by adding the [`image`](https://crates.io/crates/image)
/// crate as your direct dependency, and enabling features on it:
///
/// ```toml,ignore
/// egui_extras = { version = "*", features = ["all_loaders"] }
/// image = { version = "0.24", features = ["jpeg", "png"] } # Add the types you want support for
/// ```
///
/// ⚠ You have to configure both the supported loaders in `egui_extras` _and_ the supported image formats
/// in `image` to get any output!
///
/// ## Loader-specific information
///
/// ⚠ The exact way bytes, images, and textures are loaded is subject to change,
/// but the supported protocols and file extensions are not.
///
/// The `file` loader is a [`BytesLoader`][`egui::load::BytesLoader`].
/// It will attempt to load `file://` URIs, and infer the content type from the extension.
/// The path will be passed to [`std::fs::read`] after trimming the `file://` prefix,
/// and is resolved the same way as with `std::fs::read(path)`:
/// - Relative paths are relative to the current working directory
/// - Absolute paths are left as is.
///
/// The `http` loader is a [`BytesLoader`][`egui::load::BytesLoader`].
/// It will attempt to load `http://` and `https://` URIs, and infer the content type from the `Content-Type` header.
///
/// The `image` loader is an [`ImageLoader`][`egui::load::ImageLoader`].
/// It will attempt to load any URI with any extension other than `svg`.
/// It will also try to load any URI without an extension.
/// The content type specified by [`BytesPoll::Ready::mime`][`egui::load::BytesPoll::Ready::mime`] always takes precedence.
/// This means that even if the URI has a `png` extension, and the `png` image format is enabled, if the content type is
/// not one of the supported and enabled image formats, the loader will return [`LoadError::NotSupported`][`egui::load::LoadError::NotSupported`],
/// allowing a different loader to attempt to load the image.
///
/// The `svg` loader is an [`ImageLoader`][`egui::load::ImageLoader`].
/// It will attempt to load any URI with an `svg` extension. It will _not_ attempt to load a URI without an extension.
/// The content type specified by [`BytesPoll::Ready::mime`][`egui::load::BytesPoll::Ready::mime`] always takes precedence,
/// and must include `svg` for it to be considered supported. For example, `image/svg+xml` would be loaded by the `svg` loader.
///
/// See [`egui::load`] for more information about how loaders work.
pub fn install_image_loaders(ctx: &egui::Context) {
    #[cfg(all(not(target_arch = "wasm32"), feature = "file"))]
    if !ctx.is_loader_installed(self::file_loader::FileLoader::ID) {
        ctx.add_bytes_loader(std::sync::Arc::new(self::file_loader::FileLoader::default()));
        log::trace!("installed FileLoader");
    }

    #[cfg(feature = "http")]
    if !ctx.is_loader_installed(self::ehttp_loader::EhttpLoader::ID) {
        ctx.add_bytes_loader(std::sync::Arc::new(
            self::ehttp_loader::EhttpLoader::default(),
        ));
        log::trace!("installed EhttpLoader");
    }

    #[cfg(feature = "image")]
    if !ctx.is_loader_installed(self::image_loader::ImageCrateLoader::ID) {
        ctx.add_image_loader(std::sync::Arc::new(
            self::image_loader::ImageCrateLoader::default(),
        ));
        log::trace!("installed ImageCrateLoader");
    }

    #[cfg(feature = "svg")]
    if !ctx.is_loader_installed(self::svg_loader::SvgLoader::ID) {
        ctx.add_image_loader(std::sync::Arc::new(self::svg_loader::SvgLoader::default()));
        log::trace!("installed SvgLoader");
    }

    #[cfg(all(
        any(target_arch = "wasm32", not(feature = "file")),
        not(feature = "http"),
        not(feature = "image"),
        not(feature = "svg")
    ))]
    log::warn!("`install_image_loaders` was called, but no loaders are enabled");

    let _ = ctx;
}

#[cfg(not(target_arch = "wasm32"))]
mod file_loader;

#[cfg(feature = "http")]
mod ehttp_loader;

#[cfg(feature = "image")]
mod image_loader;

#[cfg(feature = "svg")]
mod svg_loader;
