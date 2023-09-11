// TODO: automatic cache eviction

/// Installs the default set of loaders:
/// - `file` loader on non-Wasm targets
/// - `http` loader (with the `ehttp` feature)
/// - `image` loader (with the `image` feature)
/// - `svg` loader with the `svg` feature
///
/// ⚠ This will do nothing and you won't see any images unless you enable some features!
/// If you just want to be able to load `file://` and `http://` images, enable the `all-loaders` feature.
///
/// ⚠ The supported set of image formats is configured by adding the [`image`](https://crates.io/crates/image)
/// crate as your direct dependency, and enabling features on it:
///
/// ```toml,ignore
/// image = { version = "0.24", features = ["jpeg", "png"] }
/// ```
///
/// See [`egui::load`] for more information about how loaders work.
pub fn install(ctx: &egui::Context) {
    #[cfg(all(not(target_arch = "wasm32"), feature = "file"))]
    if !ctx.is_loader_installed(self::file_loader::FileLoader::ID) {
        ctx.add_bytes_loader(std::sync::Arc::new(self::file_loader::FileLoader::default()));
        crate::log_trace!("installed FileLoader");
    }

    #[cfg(feature = "http")]
    if !ctx.is_loader_installed(self::ehttp_loader::EhttpLoader::ID) {
        ctx.add_bytes_loader(std::sync::Arc::new(
            self::ehttp_loader::EhttpLoader::default(),
        ));
        crate::log_trace!("installed EhttpLoader");
    }

    #[cfg(feature = "image")]
    if !ctx.is_loader_installed(self::image_loader::ImageCrateLoader::ID) {
        ctx.add_image_loader(std::sync::Arc::new(
            self::image_loader::ImageCrateLoader::default(),
        ));
        crate::log_trace!("installed ImageCrateLoader");
    }

    #[cfg(feature = "svg")]
    if !ctx.is_loader_installed(self::svg_loader::SvgLoader::ID) {
        ctx.add_image_loader(std::sync::Arc::new(self::svg_loader::SvgLoader::default()));
        crate::log_trace!("installed SvgLoader");
    }

    #[cfg(all(
        any(target_arch = "wasm32", not(feature = "file")),
        not(feature = "http"),
        not(feature = "image"),
        not(feature = "svg")
    ))]
    crate::log_warn!("`loaders::install` was called, but no loaders are enabled");

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
