# egui_extras

[![Latest version](https://img.shields.io/crates/v/egui_extras.svg)](https://crates.io/crates/egui_extras)
[![Documentation](https://docs.rs/egui_extras/badge.svg)](https://docs.rs/egui_extras)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

This is a crate that adds some features on top top of [`egui`](https://github.com/emilk/egui). This crate is for experimental features, and features that require big dependencies that do not belong in `egui`.

## Images
One thing `egui_extras` is commonly used for is to install image loaders for `egui`:

```toml
egui_extras = { version = "*", features = ["all_loaders"] }
image = { version = "0.24", features = ["jpeg", "png"] } # Add the types you want support for
```

```rs
egui_extras::install_image_loaders(egui_ctx);
```
