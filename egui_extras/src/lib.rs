//! This is a crate that adds some features on top top of [`egui`](https://github.com/emilk/egui). This crate are for experimental features, and features that require big dependencies that does not belong in `egui`.

#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

#[cfg(feature = "chrono")]
mod datepicker;

pub mod image;
mod layout;
mod sizing;
mod strip;
mod table;

#[cfg(feature = "chrono")]
pub use crate::datepicker::DatePickerButton;

pub use crate::image::RetainedImage;
pub(crate) use crate::layout::Layout;
pub use crate::sizing::Size;
pub use crate::strip::*;
pub use crate::table::*;
