//! This is a crate that adds some features on top top of [`egui`](https://github.com/emilk/egui). This crate are for experimental features, and features that require big dependencies that does not belong in `egui`.

#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

pub mod image;

pub use crate::image::RetainedImage;
