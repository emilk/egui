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
pub use renderer::Renderer;

/// Module for painting [`egui`] with [`wgpu`] on [`winit`].
#[cfg(feature = "winit")]
pub mod winit;

use egui::mutex::RwLock;
use std::sync::Arc;

/// Access to the render state for egui, which can be useful in combination with
/// [`egui::PaintCallback`]s for custom rendering using WGPU.
#[derive(Clone)]
pub struct RenderState {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub target_format: wgpu::TextureFormat,
    pub renderer: Arc<RwLock<Renderer>>,
}
