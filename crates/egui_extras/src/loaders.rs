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
    #[cfg(not(target_arch = "wasm32"))]
    ctx.add_bytes_loader(std::sync::Arc::new(self::file_loader::FileLoader::default()));

    #[cfg(feature = "http")]
    ctx.add_bytes_loader(std::sync::Arc::new(
        self::ehttp_loader::EhttpLoader::default(),
    ));

    #[cfg(feature = "image")]
    ctx.add_image_loader(std::sync::Arc::new(
        self::image_loader::ImageCrateLoader::default(),
    ));

    #[cfg(feature = "svg")]
    ctx.add_image_loader(std::sync::Arc::new(self::svg_loader::SvgLoader::default()));

    #[cfg(all(
        target_arch = "wasm32",
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
