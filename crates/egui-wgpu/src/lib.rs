//! This crates provides bindings between [`egui`](https://github.com/emilk/egui) and [wgpu](https://crates.io/crates/wgpu).
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![allow(unsafe_code)]

pub use wgpu;

/// Low-level painting of [`egui`](https://github.com/emilk/egui) on [`wgpu`].
pub mod renderer;
pub use renderer::CallbackFn;
pub use renderer::Renderer;

/// Module for painting [`egui`](https://github.com/emilk/egui) with [`wgpu`] on [`winit`].
#[cfg(feature = "winit")]
pub mod winit;

use std::sync::Arc;

use epaint::mutex::RwLock;

/// Access to the render state for egui.
#[derive(Clone)]
pub struct RenderState {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub target_format: wgpu::TextureFormat,
    pub renderer: Arc<RwLock<Renderer>>,
}

/// Specifies which action should be taken as consequence of a [`wgpu::SurfaceError`]
pub enum SurfaceErrorAction {
    /// Do nothing and skip the current frame.
    SkipFrame,

    /// Instructs egui to recreate the surface, then skip the current frame.
    RecreateSurface,
}

/// Configuration for using wgpu with eframe or the egui-wgpu winit feature.
#[derive(Clone)]
pub struct WgpuConfiguration {
    /// Backends that should be supported (wgpu will pick one of these)
    pub supported_backends: wgpu::Backends,

    /// Configuration passed on device request, given an adapter
    pub device_descriptor: Arc<dyn Fn(&wgpu::Adapter) -> wgpu::DeviceDescriptor<'static>>,

    /// Present mode used for the primary surface.
    pub present_mode: wgpu::PresentMode,

    /// Power preference for the adapter.
    pub power_preference: wgpu::PowerPreference,

    /// Callback for surface errors.
    pub on_surface_error: Arc<dyn Fn(wgpu::SurfaceError) -> SurfaceErrorAction>,

    pub depth_format: Option<wgpu::TextureFormat>,
}

impl Default for WgpuConfiguration {
    fn default() -> Self {
        Self {
            // Add GL backend, primarily because WebGPU is not stable enough yet.
            // (note however, that the GL backend needs to be opted-in via a wgpu feature flag)
            supported_backends: wgpu::Backends::PRIMARY | wgpu::Backends::GL,
            device_descriptor: Arc::new(|adapter| {
                let base_limits = if adapter.get_info().backend == wgpu::Backend::Gl {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                };

                wgpu::DeviceDescriptor {
                    label: Some("egui wgpu device"),
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits {
                        // When using a depth buffer, we have to be able to create a texture
                        // large enough for the entire surface, and we want to support 4k+ displays.
                        max_texture_dimension_2d: 8192,
                        ..base_limits
                    },
                }
            }),
            present_mode: wgpu::PresentMode::AutoVsync,
            power_preference: wgpu::util::power_preference_from_env()
                .unwrap_or(wgpu::PowerPreference::HighPerformance),
            depth_format: None,

            on_surface_error: Arc::new(|err| {
                if err == wgpu::SurfaceError::Outdated {
                    // This error occurs when the app is minimized on Windows.
                    // Silently return here to prevent spamming the console with:
                    // "The underlying surface has changed, and therefore the swap chain must be updated"
                } else {
                    log::warn!("Dropped frame with error: {err}");
                }
                SurfaceErrorAction::SkipFrame
            }),
        }
    }
}

/// Find the framebuffer format that egui prefers
pub fn preferred_framebuffer_format(formats: &[wgpu::TextureFormat]) -> wgpu::TextureFormat {
    for &format in formats {
        if matches!(
            format,
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
        ) {
            return format;
        }
    }
    formats[0] // take the first
}
// maybe use this-error?
#[derive(Debug)]
pub enum WgpuError {
    DeviceError(wgpu::RequestDeviceError),
    SurfaceError(wgpu::CreateSurfaceError),
}

impl std::fmt::Display for WgpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for WgpuError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WgpuError::DeviceError(e) => e.source(),
            WgpuError::SurfaceError(e) => e.source(),
        }
    }
}

impl From<wgpu::RequestDeviceError> for WgpuError {
    fn from(e: wgpu::RequestDeviceError) -> Self {
        Self::DeviceError(e)
    }
}

impl From<wgpu::CreateSurfaceError> for WgpuError {
    fn from(e: wgpu::CreateSurfaceError) -> Self {
        Self::SurfaceError(e)
    }
}
// ---------------------------------------------------------------------------

/// Profiling macro for feature "puffin"
macro_rules! profile_function {
    ($($arg: tt)*) => {
        #[cfg(feature = "puffin")]
        #[cfg(not(target_arch = "wasm32"))]
        puffin::profile_function!($($arg)*);
    };
}
pub(crate) use profile_function;

/// Profiling macro for feature "puffin"
macro_rules! profile_scope {
    ($($arg: tt)*) => {
        #[cfg(feature = "puffin")]
        #[cfg(not(target_arch = "wasm32"))]
        puffin::profile_scope!($($arg)*);
    };
}
pub(crate) use profile_scope;
