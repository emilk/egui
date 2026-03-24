use std::sync::Arc;

/// A cloneable display handle for use with [`wgpu::InstanceDescriptor`].
///
/// This trait exists so that a [`winit::event_loop::OwnedDisplayHandle`] (or similar platform
/// display handle) can be stored, cloned, and later passed to wgpu.
///
/// wgpu requires an explicit display handle for GLES on some platforms (notably Wayland).
/// Because [`wgpu::InstanceDescriptor`] contains a `Box<dyn WgpuHasDisplayHandle>` which is
/// not cloneable, we wrap the handle in this trait so it can be cloned alongside the rest of
/// the egui wgpu configuration.
///
/// This is automatically implemented for all types that satisfy the bounds (including
/// [`winit::event_loop::OwnedDisplayHandle`]).
pub trait EguiDisplayHandle:
    wgpu::rwh::HasDisplayHandle + std::fmt::Debug + Send + Sync + 'static
{
    /// Clone this handle into a `Box<dyn WgpuHasDisplayHandle>` suitable for setting on
    /// [`wgpu::InstanceDescriptor::display`].
    fn clone_for_wgpu(&self) -> Box<dyn wgpu::wgt::WgpuHasDisplayHandle>;

    /// Clone this handle into a new `Box<dyn EguiDisplayHandle>`.
    fn clone_display_handle(&self) -> Box<dyn EguiDisplayHandle>;
}

impl Clone for Box<dyn EguiDisplayHandle> {
    fn clone(&self) -> Self {
        // We need to deref here, otherwise this causes infinite recursion stack overflow.
        (**self).clone_display_handle()
    }
}

impl<T> EguiDisplayHandle for T
where
    T: wgpu::rwh::HasDisplayHandle + Clone + std::fmt::Debug + Send + Sync + 'static,
{
    fn clone_for_wgpu(&self) -> Box<dyn wgpu::wgt::WgpuHasDisplayHandle> {
        Box::new(self.clone())
    }

    fn clone_display_handle(&self) -> Box<dyn EguiDisplayHandle> {
        Box::new(self.clone())
    }
}

#[derive(Clone)]
pub enum WgpuSetup {
    /// Construct a wgpu setup using some predefined settings & heuristics.
    /// This is the default option. You can customize most behaviours overriding the
    /// supported backends, power preferences, and device description.
    ///
    /// By default can also be configured with various environment variables:
    /// * `WGPU_BACKEND`: `vulkan`, `dx12`, `metal`, `opengl`, `webgpu`
    /// * `WGPU_POWER_PREF`: `low`, `high` or `none`
    /// * `WGPU_TRACE`: Path to a file to output a wgpu trace file.
    ///
    /// Each instance flag also comes with an environment variable (for details see [`wgpu::InstanceFlags`]):
    /// * `WGPU_VALIDATION`: Enables validation (enabled by default in debug builds).
    /// * `WGPU_DEBUG`: Generate debug information in shaders and objects  (enabled by default in debug builds).
    /// * `WGPU_ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER`: Whether wgpu should expose adapters that run on top of non-compliant adapters.
    /// * `WGPU_GPU_BASED_VALIDATION`: Enable GPU-based validation.
    CreateNew(WgpuSetupCreateNew),

    /// Run on an existing wgpu setup.
    Existing(WgpuSetupExisting),
}

impl WgpuSetup {
    /// Creates a new [`WgpuSetup::CreateNew`] with the given display handle.
    ///
    /// This is the recommended constructor. Most platforms (Windows, macOS/iOS, Android, web)
    /// work fine without a display handle, but some (e.g. Wayland on Linux with GLES) require
    /// one. Providing it unconditionally ensures your app works everywhere.
    ///
    /// If you don't have a display handle available, use [`Self::without_display_handle`]
    /// instead — it will still work on the majority of platforms.
    ///
    /// With winit, pass [`EventLoop::owned_display_handle`](winit::event_loop::EventLoop::owned_display_handle).
    pub fn from_display_handle(display_handle: impl EguiDisplayHandle) -> Self {
        Self::CreateNew(WgpuSetupCreateNew::from_display_handle(display_handle))
    }

    /// Creates a new [`WgpuSetup::CreateNew`] without a display handle.
    ///
    /// A display handle is not required for headless operation (offscreen rendering, tests,
    /// compute-only workloads). It also isn't needed on most platforms even when presenting
    /// to a window — only some configurations (e.g. Wayland on Linux with GLES) require one.
    ///
    /// If you do have a display handle available, prefer [`Self::from_display_handle`] for
    /// maximum compatibility. With winit you can obtain one via
    /// [`EventLoop::owned_display_handle`](winit::event_loop::EventLoop::owned_display_handle).
    pub fn without_display_handle() -> Self {
        Self::CreateNew(WgpuSetupCreateNew::without_display_handle())
    }
}

impl std::fmt::Debug for WgpuSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateNew(create_new) => f
                .debug_tuple("WgpuSetup::CreateNew")
                .field(create_new)
                .finish(),
            Self::Existing { .. } => f.debug_tuple("WgpuSetup::Existing").finish(),
        }
    }
}

impl WgpuSetup {
    /// Creates a new [`wgpu::Instance`] or clones the existing one.
    ///
    /// Does *not* store the wgpu instance, so calling this repeatedly may
    /// create a new instance every time!
    pub async fn new_instance(&self) -> wgpu::Instance {
        match self {
            Self::CreateNew(create_new) => {
                #[allow(clippy::allow_attributes, unused_mut)]
                let mut backends = create_new.instance_descriptor.backends;

                // Don't try WebGPU if we're not in a secure context.
                #[cfg(target_arch = "wasm32")]
                if backends.contains(wgpu::Backends::BROWSER_WEBGPU) {
                    let is_secure_context =
                        wgpu::web_sys::window().is_some_and(|w| w.is_secure_context());
                    if !is_secure_context {
                        log::info!(
                            "WebGPU is only available in secure contexts, i.e. on HTTPS and on localhost."
                        );
                        backends.remove(wgpu::Backends::BROWSER_WEBGPU);
                    }
                }

                log::debug!("Creating wgpu instance with backends {backends:?}");
                let desc = &create_new.instance_descriptor;
                let descriptor = wgpu::InstanceDescriptor {
                    backends: desc.backends,
                    flags: desc.flags,
                    backend_options: desc.backend_options.clone(),
                    memory_budget_thresholds: desc.memory_budget_thresholds,
                    display: create_new
                        .display_handle
                        .as_ref()
                        .map(|handle| handle.clone_for_wgpu()),
                };
                wgpu::util::new_instance_with_webgpu_detection(descriptor).await
            }
            Self::Existing(existing) => existing.instance.clone(),
        }
    }
}

impl From<WgpuSetupCreateNew> for WgpuSetup {
    fn from(create_new: WgpuSetupCreateNew) -> Self {
        Self::CreateNew(create_new)
    }
}

impl From<WgpuSetupExisting> for WgpuSetup {
    fn from(existing: WgpuSetupExisting) -> Self {
        Self::Existing(existing)
    }
}

/// Method for selecting an adapter on native.
///
/// This can be used for fully custom adapter selection.
/// If available, `wgpu::Surface` is passed to allow checking for surface compatibility.
pub type NativeAdapterSelectorMethod = Arc<
    dyn Fn(&[wgpu::Adapter], Option<&wgpu::Surface<'_>>) -> Result<wgpu::Adapter, String>
        + Send
        + Sync,
>;

/// Configuration for creating a new wgpu setup.
///
/// Used for [`WgpuSetup::CreateNew`].
///
/// Use [`Self::from_display_handle`] when you have a display handle available — this is the
/// recommended constructor. With winit you can obtain one via
/// [`EventLoop::owned_display_handle`](winit::event_loop::EventLoop::owned_display_handle).
/// Most platforms (Windows, macOS/iOS, Android, web) work fine without one, but some
/// (e.g. Wayland on Linux with GLES) require it. Providing it unconditionally ensures your
/// app works everywhere.
///
/// If you don't have a display handle, use [`Self::without_display_handle`] — it will still
/// work on the majority of platforms, and is appropriate for headless rendering, tests, or
/// web targets.
///
/// Note: The [`wgpu::InstanceDescriptor::display`] field is always stored as `None` in
/// [`Self::instance_descriptor`]. The display handle is stored separately so it can be cloned
/// (since [`wgpu::InstanceDescriptor`] itself does not implement `Clone`), and is injected
/// into the descriptor at instance creation time.
pub struct WgpuSetupCreateNew {
    /// Instance descriptor for creating a wgpu instance.
    ///
    /// The [`wgpu::InstanceDescriptor::display`] field should be left as `None`; use the
    /// [`Self::display_handle`] field instead (it will be injected when the instance is created).
    ///
    /// The most important field is [`wgpu::InstanceDescriptor::backends`], which
    /// controls which backends are supported (wgpu will pick one of these).
    /// If you only want to support WebGL (and not WebGPU),
    /// you can set this to [`wgpu::Backends::GL`].
    /// By default on web, WebGPU will be used if available.
    /// WebGL will only be used as a fallback,
    /// and only if you have enabled the `webgl` feature of crate `wgpu`.
    pub instance_descriptor: wgpu::InstanceDescriptor,

    /// The display handle to pass to wgpu when creating the instance.
    ///
    /// Most platforms (Windows, macOS/iOS, Android, web) work without this, but some
    /// (e.g. Wayland on Linux with GLES) require it. If you have a display handle
    /// available, providing it ensures maximum compatibility.
    ///
    /// When using winit, this is typically the
    /// [`winit::event_loop::OwnedDisplayHandle`] obtained from the event loop.
    pub display_handle: Option<Box<dyn EguiDisplayHandle>>,

    /// Power preference for the adapter if [`Self::native_adapter_selector`] is not set or targeting web.
    pub power_preference: wgpu::PowerPreference,

    /// Optional selector for native adapters.
    ///
    /// This field has no effect when targeting web!
    /// Otherwise, if set [`Self::power_preference`] is ignored and the adapter is instead selected by this method.
    /// Note that [`Self::instance_descriptor`]'s [`wgpu::InstanceDescriptor::backends`]
    /// are still used to filter the adapter enumeration in the first place.
    ///
    /// Defaults to `None`.
    pub native_adapter_selector: Option<NativeAdapterSelectorMethod>,

    /// Configuration passed on device request, given an adapter
    pub device_descriptor:
        Arc<dyn Fn(&wgpu::Adapter) -> wgpu::DeviceDescriptor<'static> + Send + Sync>,
}

impl WgpuSetupCreateNew {
    /// Creates a new configuration with the given display handle.
    ///
    /// This is the recommended constructor. Most platforms (Windows, macOS/iOS, Android, web)
    /// work fine without a display handle, but some (e.g. Wayland on Linux with GLES) require
    /// one. Providing it unconditionally ensures your app works everywhere.
    ///
    /// If you don't have a display handle available, use [`Self::without_display_handle`]
    /// instead — it will still work on the majority of platforms.
    ///
    /// With winit, pass [`EventLoop::owned_display_handle`](winit::event_loop::EventLoop::owned_display_handle).
    pub fn from_display_handle(display_handle: impl EguiDisplayHandle) -> Self {
        Self {
            display_handle: Some(Box::new(display_handle)),
            ..Self::without_display_handle()
        }
    }

    /// Creates a new configuration without a display handle.
    ///
    /// A display handle is not required for headless operation (offscreen rendering, tests,
    /// compute-only workloads). It also isn't needed on most platforms even when presenting
    /// to a window — only some configurations (e.g. Wayland on Linux with GLES) require one.
    ///
    /// If you do have a display handle available, prefer [`Self::from_display_handle`] for
    /// maximum compatibility. With winit you can obtain one via
    /// [`EventLoop::owned_display_handle`](winit::event_loop::EventLoop::owned_display_handle).
    pub fn without_display_handle() -> Self {
        Self {
            instance_descriptor: wgpu::InstanceDescriptor {
                // Add GL backend, primarily because WebGPU is not stable enough yet.
                // (note however, that the GL backend needs to be opted-in via the wgpu feature flag "webgl")
                backends: wgpu::Backends::from_env()
                    .unwrap_or(wgpu::Backends::PRIMARY | wgpu::Backends::GL),
                flags: wgpu::InstanceFlags::from_build_config().with_env(),
                backend_options: wgpu::BackendOptions::from_env_or_default(),
                memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
                display: None,
            },

            display_handle: None,

            power_preference: wgpu::PowerPreference::from_env()
                .unwrap_or(wgpu::PowerPreference::HighPerformance),

            native_adapter_selector: None,

            device_descriptor: Arc::new(|adapter| {
                let base_limits = if adapter.get_info().backend == wgpu::Backend::Gl {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                };

                wgpu::DeviceDescriptor {
                    label: Some("egui wgpu device"),
                    required_limits: wgpu::Limits {
                        // When using a depth buffer, we have to be able to create a texture
                        // large enough for the entire surface, and we want to support 4k+ displays.
                        max_texture_dimension_2d: 8192,
                        ..base_limits
                    },
                    ..Default::default()
                }
            }),
        }
    }
}

impl Clone for WgpuSetupCreateNew {
    fn clone(&self) -> Self {
        let desc = &self.instance_descriptor;
        Self {
            instance_descriptor: wgpu::InstanceDescriptor {
                backends: desc.backends,
                flags: desc.flags,
                backend_options: desc.backend_options.clone(),
                memory_budget_thresholds: desc.memory_budget_thresholds,
                display: None,
            },
            display_handle: self.display_handle.clone(),
            power_preference: self.power_preference,
            native_adapter_selector: self.native_adapter_selector.clone(),
            device_descriptor: Arc::clone(&self.device_descriptor),
        }
    }
}

impl std::fmt::Debug for WgpuSetupCreateNew {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WgpuSetupCreateNew")
            .field("instance_descriptor", &self.instance_descriptor)
            .field("display_handle", &self.display_handle)
            .field("power_preference", &self.power_preference)
            .field(
                "native_adapter_selector",
                &self.native_adapter_selector.is_some(),
            )
            .finish()
    }
}

/// Configuration for using an existing wgpu setup.
///
/// Used for [`WgpuSetup::Existing`].
#[derive(Clone)]
pub struct WgpuSetupExisting {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}
