//! [`egui`] bindings for [`glow`](https://github.com/grovesNL/glow).
//!
//! The main type you want to use is [`EguiGlow`].
//!
//! This library is an [`epi`] backend.
//! If you are writing an app, you may want to look at [`eframe`](https://docs.rs/eframe) instead.

#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

pub mod painter;
pub use glow;
pub use painter::Painter;
mod misc_util;
mod post_process;
mod shader_version;
mod vao_emulate;

#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
pub mod winit;
#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
pub use winit::*;

#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
mod epi_backend;

#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
pub use epi_backend::{run, NativeOptions};
