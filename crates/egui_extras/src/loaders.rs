// TODO: automatic cache eviction

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
