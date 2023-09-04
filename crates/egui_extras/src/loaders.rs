// TODO: automatic cache eviction

/// Install the default set of loaders:
/// - `file` loader on non-Wasm targets
/// - `http` loader with the `ehttp` feature
/// - `image` loader with the `image` feature
///   - the supported set of image formats is configured by enabling `image` crate features.
/// - `svg` loader with the `svg` feature
///
/// The `file` and `http` loaders are bytes loaders, they do not know how to turn those
/// bytes into an image. `svg` is for loading `.svg` files, and `image` can load any
/// other image format enabled on the `image` crate.
pub fn install(ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    ctx.add_bytes_loader(std::sync::Arc::new(self::file_loader::FileLoader::default()));

    #[cfg(feature = "ehttp")]
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
        not(feature = "ehttp"),
        not(feature = "image"),
        not(feature = "svg")
    ))]
    crate::log_warn!("`loaders::install` was called, but no loaders are enabled");

    let _ = ctx;
}

#[cfg(not(target_arch = "wasm32"))]
mod file_loader;

#[cfg(feature = "ehttp")]
mod ehttp_loader;

#[cfg(feature = "image")]
mod image_loader;

#[cfg(feature = "svg")]
mod svg_loader;
