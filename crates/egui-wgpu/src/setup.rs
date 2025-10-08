use std::sync::Arc;

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

impl Default for WgpuSetup {
    fn default() -> Self {
        Self::CreateNew(WgpuSetupCreateNew::default())
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
                #[allow(unused_mut, clippy::allow_attributes)]
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
                wgpu::util::new_instance_with_webgpu_detection(&create_new.instance_descriptor)
                    .await
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
pub struct WgpuSetupCreateNew {
    /// Instance descriptor for creating a wgpu instance.
    ///
    /// The most important field is [`wgpu::InstanceDescriptor::backends`], which
    /// controls which backends are supported (wgpu will pick one of these).
    /// If you only want to support WebGL (and not WebGPU),
    /// you can set this to [`wgpu::Backends::GL`].
    /// By default on web, WebGPU will be used if available.
    /// WebGL will only be used as a fallback,
    /// and only if you have enabled the `webgl` feature of crate `wgpu`.
    pub instance_descriptor: wgpu::InstanceDescriptor,

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

impl Clone for WgpuSetupCreateNew {
    fn clone(&self) -> Self {
        Self {
            instance_descriptor: self.instance_descriptor.clone(),
            power_preference: self.power_preference,
            native_adapter_selector: self.native_adapter_selector.clone(),
            device_descriptor: self.device_descriptor.clone(),
        }
    }
}

impl std::fmt::Debug for WgpuSetupCreateNew {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WgpuSetupCreateNew")
            .field("instance_descriptor", &self.instance_descriptor)
            .field("power_preference", &self.power_preference)
            .field(
                "native_adapter_selector",
                &self.native_adapter_selector.is_some(),
            )
            .finish()
    }
}

impl Default for WgpuSetupCreateNew {
    fn default() -> Self {
        Self {
            instance_descriptor: wgpu::InstanceDescriptor {
                // Add GL backend, primarily because WebGPU is not stable enough yet.
                // (note however, that the GL backend needs to be opted-in via the wgpu feature flag "webgl")
                backends: wgpu::Backends::from_env()
                    .unwrap_or(wgpu::Backends::PRIMARY | wgpu::Backends::GL),
                flags: wgpu::InstanceFlags::from_build_config().with_env(),
                backend_options: wgpu::BackendOptions::from_env_or_default(),
                memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            },

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
