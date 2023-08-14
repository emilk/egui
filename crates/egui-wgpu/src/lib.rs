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

#[derive(thiserror::Error, Debug)]
pub enum WgpuError {
    #[error("Failed to create wgpu adapter, no suitable adapter found.")]
    NoSuitableAdapterFound,

    #[error("There was no valid format for the surface at all.")]
    NoSurfaceFormatsAvailable,

    #[error(transparent)]
    RequestDeviceError(#[from] wgpu::RequestDeviceError),

    #[error(transparent)]
    CreateSurfaceError(#[from] wgpu::CreateSurfaceError),
}

/// Access to the render state for egui.
#[derive(Clone)]
pub struct RenderState {
    /// Wgpu adapter used for rendering.
    pub adapter: Arc<wgpu::Adapter>,

    /// Wgpu device used for rendering, created from the adapter.
    pub device: Arc<wgpu::Device>,

    /// Wgpu queue used for rendering, created from the adapter.
    pub queue: Arc<wgpu::Queue>,

    /// The target texture format used for presenting to the window.
    pub target_format: wgpu::TextureFormat,

    /// Egui renderer responsible for drawing the UI.
    pub renderer: Arc<RwLock<Renderer>>,
}

impl RenderState {
    /// Creates a new `RenderState`, containing everything needed for drawing egui with wgpu.
    ///
    /// # Errors
    /// Wgpu initialization may fail due to incompatible hardware or driver for a given config.
    pub async fn create(
        config: &WgpuConfiguration,
        instance: &wgpu::Instance,
        surface: &wgpu::Surface,
        depth_format: Option<wgpu::TextureFormat>,
        msaa_samples: u32,
    ) -> Result<Self, WgpuError> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: config.power_preference,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(WgpuError::NoSuitableAdapterFound)?;

        let target_format =
            crate::preferred_framebuffer_format(&surface.get_capabilities(&adapter).formats)?;

        let (device, queue) = adapter
            .request_device(&(*config.device_descriptor)(&adapter), None)
            .await?;

        let renderer = Renderer::new(&device, target_format, depth_format, msaa_samples);

        Ok(RenderState {
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
            target_format,
            renderer: Arc::new(RwLock::new(renderer)),
        })
    }
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
}

impl Default for WgpuConfiguration {
    fn default() -> Self {
        Self {
            // Add GL backend, primarily because WebGPU is not stable enough yet.
            // (note however, that the GL backend needs to be opted-in via a wgpu feature flag)
            supported_backends: wgpu::util::backend_bits_from_env()
                .unwrap_or(wgpu::Backends::PRIMARY | wgpu::Backends::GL),
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
///
/// # Errors
/// Returns [`WgpuError::NoSurfaceFormatsAvailable`] if the given list of formats is empty.
pub fn preferred_framebuffer_format(
    formats: &[wgpu::TextureFormat],
) -> Result<wgpu::TextureFormat, WgpuError> {
    for &format in formats {
        if matches!(
            format,
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
        ) {
            return Ok(format);
        }
    }

    formats
        .get(0)
        .copied()
        .ok_or(WgpuError::NoSurfaceFormatsAvailable)
}

/// Take's epi's depth/stencil bits and returns the corresponding wgpu format.
pub fn depth_format_from_bits(depth_buffer: u8, stencil_buffer: u8) -> Option<wgpu::TextureFormat> {
    match (depth_buffer, stencil_buffer) {
        (0, 8) => Some(wgpu::TextureFormat::Stencil8),
        (16, 0) => Some(wgpu::TextureFormat::Depth16Unorm),
        (24, 0) => Some(wgpu::TextureFormat::Depth24Plus),
        (24, 8) => Some(wgpu::TextureFormat::Depth24PlusStencil8),
        (32, 0) => Some(wgpu::TextureFormat::Depth32Float),
        (32, 8) => Some(wgpu::TextureFormat::Depth32FloatStencil8),
        _ => None,
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
