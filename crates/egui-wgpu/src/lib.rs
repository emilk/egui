//! This crates provides bindings between [`egui`](https://github.com/emilk/egui) and [wgpu](https://crates.io/crates/wgpu).
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![allow(unsafe_code)]

pub use wgpu;

/// Low-level painting of [`egui`] on [`wgpu`].
pub mod renderer;
pub use renderer::CallbackFn;

/// Module for painting [`egui`] with [`wgpu`] on [`winit`].
#[cfg(feature = "winit")]
pub mod winit;

#[cfg(feature = "winit")]
pub use crate::winit::RenderState;
