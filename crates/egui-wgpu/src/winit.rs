#![expect(clippy::missing_errors_doc)]
#![expect(clippy::undocumented_unsafe_blocks)]
#![expect(clippy::unwrap_used)] // TODO(emilk): avoid unwraps
#![expect(unsafe_code)]

use crate::{RenderState, SurfaceConfig, SurfaceErrorAction, WgpuConfiguration, renderer};
use crate::{RendererOptions, capture::CaptureState};
use core::num::NonZeroU32;
use egui::{Context, ViewportId, ViewportIdMap, ViewportIdSet};
use std::sync::Arc;

struct SurfaceState {
    surface: wgpu::Surface<'static>,
    alpha_mode: wgpu::CompositeAlphaMode,
    width: u32,
    height: u32,
    // `width` and `height` are the size the surface is configured at,
    // which `surface_fit` keeps within the device's `max_texture_dimension_2d`; these are the
    // window's own size, and `render_scale` is how much smaller the surface is drawn.
    window_width: u32,
    window_height: u32,
    render_scale: f32,
    resizing: bool,
    needs_reconfigure: bool,
    needs_recreate: bool,
}

/// The size to configure a window's surface at, and the factor egui
/// draws it at, so that neither side exceeds `max_side` (the device's
/// `max_texture_dimension_2d`).
///
/// `Surface::configure` rejects a larger surface, and wgpu reports that as an uncaptured
/// validation error, which panics: a window bigger than the GPU's limit (restored at another
/// monitor's scale, stretched across monitors, …) closed the app before it opened. Both sides
/// shrink by the same factor and egui draws at that factor, so the whole window is drawn into
/// the surface. Where the swapchain is stretched over the window (DX12 uses
/// `DXGI_SCALING_STRETCH`) it fills the window and lines up with the pointer, only softer;
/// wgpu's GL backend copies it unscaled into a corner instead. Degraded either way, but not a
/// crash. A window within the limit is returned unchanged, with a factor of 1. The longer side
/// becomes exactly `max_side`, and a side that isn't zero never becomes zero (an empty surface
/// can't be configured either).
pub fn surface_fit(width: u32, height: u32, max_side: u32) -> (u32, u32, f32) {
    let max_side = max_side.max(1);
    if width <= max_side && height <= max_side {
        return (width, height, 1.0);
    }
    let longest = width.max(height);
    let scale = (f64::from(max_side) / f64::from(longest)).min(1.0);
    // `as` saturates and `scale` is finite and in (0, 1]: no overflow, NaN or panic. The longer
    // side is set rather than computed, so floating-point rounding can't leave it a pixel short.
    let fit = |side: u32| {
        if side == 0 {
            0
        } else if side == longest {
            max_side
        } else {
            ((f64::from(side) * scale).floor() as u32).clamp(1, max_side)
        }
    };
    (fit(width), fit(height), scale as f32)
}

/// Runs `f` and returns an error it raises on `device`, instead of
/// handing it to wgpu's uncaptured-error handler, which panics by default.
///
/// eframe configures a window's surface before the app is created, so no app code can install
/// a handler in time, and a surface the device can't configure closed the app before its window
/// appeared: wgpu's GL backend on a 2015 Intel driver timed out waiting for the GPU to go idle
/// (`ConfigureSurfaceError::GpuWaitTimeout`). Returned as an error instead, eframe returns
/// `Error::Wgpu` and the app can fall back (for example to OpenGL).
///
/// Validation, out-of-memory and internal errors are caught, one scope each; each scope keeps
/// the first error of its kind, and a validation error is returned before an out-of-memory one,
/// which is returned before an internal one. Not caught: a lost device (wgpu reports that only
/// to the device-lost callback, so `f` seems to succeed), a panic inside `f`, and errors raised
/// on other threads (wgpu's error scopes belong to the calling thread).
pub async fn catch_errors<T>(
    device: &wgpu::Device,
    f: impl FnOnce() -> T,
) -> Result<T, wgpu::Error> {
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let out_of_memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
    // Should `f` panic, the guards pop their scopes as they drop, innermost first.
    let value = f();
    // Popped innermost first, as wgpu requires; each pop takes effect at once.
    let internal = internal.pop();
    let out_of_memory = out_of_memory.pop();
    let validation = validation.pop();
    let (validation, out_of_memory, internal) =
        (validation.await, out_of_memory.await, internal.await);
    match validation.or(out_of_memory).or(internal) {
        Some(error) => Err(error),
        None => Ok(value),
    }
}

/// Everything you need to paint egui with [`wgpu`] on [`winit`].
///
/// Alternatively you can use [`crate::Renderer`] directly.
///
/// NOTE: all egui viewports share the same painter.
pub struct Painter {
    context: Context,
    config: WgpuConfiguration,
    options: RendererOptions,
    support_transparent_backbuffer: bool,
    screen_capture_state: Option<CaptureState>,

    instance: wgpu::Instance,
    render_state: Option<RenderState>,

    // Per viewport/window:
    depth_texture_view: ViewportIdMap<wgpu::TextureView>,
    msaa_texture_view: ViewportIdMap<wgpu::TextureView>,
    surfaces: ViewportIdMap<SurfaceState>,
}

impl Painter {
    /// Manages [`wgpu`] state, including surface state, required to render egui.
    ///
    /// Only the [`wgpu::Instance`] is initialized here. Device selection and the initialization
    /// of render + surface state is deferred until the painter is given its first window target
    /// via [`set_window()`](Self::set_window). (Ensuring that a device that's compatible with the
    /// native window is chosen)
    ///
    /// Before calling [`paint_and_update_textures()`](Self::paint_and_update_textures) a
    /// [`wgpu::Surface`] must be initialized (and corresponding render state) by calling
    /// [`set_window()`](Self::set_window) once you have
    /// a [`winit::window::Window`] with a valid `.raw_window_handle()`
    /// associated.
    pub async fn new(
        context: Context,
        config: WgpuConfiguration,
        support_transparent_backbuffer: bool,
        options: RendererOptions,
    ) -> Self {
        let instance = config.wgpu_setup.new_instance().await;

        Self {
            context,
            config,
            options,
            support_transparent_backbuffer,
            screen_capture_state: None,

            instance,
            render_state: None,

            depth_texture_view: Default::default(),
            surfaces: Default::default(),
            msaa_texture_view: Default::default(),
        }
    }

    /// Get the [`RenderState`].
    ///
    /// Will return [`None`] if the render state has not been initialized yet.
    pub fn render_state(&self) -> Option<RenderState> {
        self.render_state.clone()
    }

    /// The surface configuration (present mode, frame latency) the surfaces are configured with.
    pub fn surface_config(&self) -> SurfaceConfig {
        self.config.surface
    }

    /// Change the surface configuration (present mode, frame latency) at runtime.
    ///
    /// All surfaces are reconfigured before the next paint.
    ///
    /// This is cheap to call every frame.
    pub fn set_surface_config(&mut self, surface_config: SurfaceConfig) {
        if self.config.surface == surface_config {
            return;
        }

        self.config.surface = surface_config;

        #[expect(clippy::iter_over_hash_type)]
        for surface in self.surfaces.values_mut() {
            surface.needs_reconfigure = true;
        }
    }

    fn configure_surface(
        surface_state: &SurfaceState,
        render_state: &RenderState,
        config: &SurfaceConfig,
    ) {
        profiling::function_scope!();

        let SurfaceConfig {
            present_mode,
            desired_maximum_frame_latency,
        } = *config;

        // Transaction presentation can hold a drawable during AppKit live resize. Keep the
        // configured low-latency path normally, but use three Metal drawables while resizing.
        #[cfg(all(target_os = "macos", feature = "macos-window-resize-jitter-fix"))]
        let desired_maximum_frame_latency = if surface_state.resizing {
            Some(desired_maximum_frame_latency.unwrap_or(2).max(2))
        } else {
            desired_maximum_frame_latency
        };

        let width = surface_state.width;
        let height = surface_state.height;

        let mut surf_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: render_state.target_format,
            present_mode,
            alpha_mode: surface_state.alpha_mode,
            view_formats: vec![render_state.target_format],
            ..surface_state
                .surface
                .get_default_config(&render_state.adapter, width, height)
                .expect("The surface isn't supported by this adapter")
        };

        if let Some(desired_maximum_frame_latency) = desired_maximum_frame_latency {
            surf_config.desired_maximum_frame_latency = desired_maximum_frame_latency;
        }

        surface_state
            .surface
            .configure(&render_state.device, &surf_config);
    }

    /// Drop the existing [`wgpu::Surface`] for `viewport_id` and create a fresh one for the
    /// given window via [`wgpu::Instance::create_surface`], then configure it.
    ///
    /// Used to recover from [`wgpu::CurrentSurfaceTexture::Lost`], where reconfiguring the
    /// existing surface object cannot recover.
    fn recreate_surface(
        &mut self,
        viewport_id: ViewportId,
        window: &Arc<winit::window::Window>,
    ) -> Result<(), crate::WgpuError> {
        profiling::function_scope!();

        let Some(old_state) = self.surfaces.remove(&viewport_id) else {
            return Ok(());
        };

        let surface = self.instance.create_surface(Arc::clone(window))?;
        // The window's size; `width` and `height` may have been fitted.
        self.install_surface(
            surface,
            viewport_id,
            old_state.window_width,
            old_state.window_height,
            old_state.resizing,
        );
        Ok(())
    }

    /// Updates (or clears) the [`winit::window::Window`] associated with the [`Painter`]
    ///
    /// This creates a [`wgpu::Surface`] for the given Window (as well as initializing render
    /// state if needed) that is used for egui rendering.
    ///
    /// This must be called before trying to render via
    /// [`paint_and_update_textures`](Self::paint_and_update_textures)
    ///
    /// # Portability
    ///
    /// _In particular it's important to note that on Android a it's only possible to create
    /// a window surface between `Resumed` and `Paused` lifecycle events, and Winit will panic on
    /// attempts to query the raw window handle while paused._
    ///
    /// On Android [`set_window`](Self::set_window) should be called with `Some(window)` for each
    /// `Resumed` event and `None` for each `Paused` event. Currently, on all other platforms
    /// [`set_window`](Self::set_window) may be called with `Some(window)` as soon as you have a
    /// valid [`winit::window::Window`].
    ///
    /// # Errors
    /// If the provided wgpu configuration does not match an available device.
    pub async fn set_window(
        &mut self,
        viewport_id: ViewportId,
        window: Option<Arc<winit::window::Window>>,
    ) -> Result<(), crate::WgpuError> {
        profiling::scope!("Painter::set_window"); // profile_function gives bad names for async functions

        if let Some(window) = window {
            let size = window.inner_size();
            if !self.surfaces.contains_key(&viewport_id) {
                let surface = self.instance.create_surface(window)?;
                self.add_surface(surface, viewport_id, size).await?;
            }
        } else {
            log::warn!("No window - clearing all surfaces");
            self.surfaces.clear();
        }
        Ok(())
    }

    /// Updates (or clears) the [`winit::window::Window`] associated with the [`Painter`] without taking ownership of the window.
    ///
    /// Like [`set_window`](Self::set_window) except:
    ///
    /// # Safety
    /// The user is responsible for ensuring that the window is alive for as long as it is set.
    pub async unsafe fn set_window_unsafe(
        &mut self,
        viewport_id: ViewportId,
        window: Option<&winit::window::Window>,
    ) -> Result<(), crate::WgpuError> {
        profiling::scope!("Painter::set_window_unsafe"); // profile_function gives bad names for async functions

        if let Some(window) = window {
            let size = window.inner_size();
            if !self.surfaces.contains_key(&viewport_id) {
                let surface = unsafe {
                    self.instance
                        .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window)?)?
                };
                self.add_surface(surface, viewport_id, size).await?;
            }
        } else {
            log::warn!("No window - clearing all surfaces");
            self.surfaces.clear();
        }
        Ok(())
    }

    async fn add_surface(
        &mut self,
        surface: wgpu::Surface<'static>,
        viewport_id: ViewportId,
        size: winit::dpi::PhysicalSize<u32>,
    ) -> Result<(), crate::WgpuError> {
        // The device is kept for `catch_errors` below.
        let render_state = match self.render_state.take() {
            Some(render_state) => render_state,
            None => {
                RenderState::create(&self.config, &self.instance, Some(&surface), self.options)
                    .await?
            }
        };
        let device = render_state.device.clone();
        self.render_state = Some(render_state);
        // A surface the device can't configure is returned as an error
        // instead of wgpu's default handler panicking; for the root window at start-up, eframe
        // returns it as `Error::Wgpu` before the app is created, and the app can fall back.
        // Nothing is kept for the window. A window whose size is zero here is first configured
        // later, on resize, outside this.
        let installed = catch_errors(&device, || {
            self.install_surface(surface, viewport_id, size.width, size.height, false);
        })
        .await;
        if let Err(error) = installed {
            self.surfaces.remove(&viewport_id);
            self.depth_texture_view.remove(&viewport_id);
            self.msaa_texture_view.remove(&viewport_id);
            return Err(crate::WgpuError::ConfigureSurface(error));
        }
        Ok(())
    }

    /// Inserts a freshly created surface into [`Self::surfaces`] and configures it.
    ///
    /// Render state must already be initialised before calling this.
    // NOTE: The same assumption is already required by `resize_and_generate_depth_texture_view_and_msaa_view`.
    fn install_surface(
        &mut self,
        surface: wgpu::Surface<'static>,
        viewport_id: ViewportId,
        width: u32,
        height: u32,
        resizing: bool,
    ) {
        let alpha_mode = {
            // Panic: We use the same failure mode as `resize_and_generate_depth_texture_view_and_msaa_view`
            let render_state = self
                .render_state
                .as_ref()
                .expect("install_surface called before render_state initialization");
            if self.support_transparent_backbuffer {
                let supported_alpha_modes =
                    surface.get_capabilities(&render_state.adapter).alpha_modes;
                // Prefer pre multiplied over post multiplied!
                if supported_alpha_modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied) {
                    wgpu::CompositeAlphaMode::PreMultiplied
                } else if supported_alpha_modes.contains(&wgpu::CompositeAlphaMode::PostMultiplied)
                {
                    wgpu::CompositeAlphaMode::PostMultiplied
                } else {
                    log::warn!(
                        "Transparent window was requested, but the active wgpu surface does not support a `CompositeAlphaMode` with transparency."
                    );
                    wgpu::CompositeAlphaMode::Auto
                }
            } else {
                wgpu::CompositeAlphaMode::Auto
            }
        };
        // Store the fitted size, so nothing configures the window's own.
        // `render_scale` starts at 1 so that the resize below, which sets it, logs the fit once.
        let (fit_width, fit_height, _) = surface_fit(width, height, self.max_surface_side());
        self.surfaces.insert(
            viewport_id,
            SurfaceState {
                surface,
                width: fit_width,
                height: fit_height,
                window_width: width,
                window_height: height,
                render_scale: 1.0,
                alpha_mode,
                resizing,
                needs_reconfigure: false,
                needs_recreate: false,
            },
        );
        let Some(width) = NonZeroU32::new(width) else {
            log::debug!("The window width was zero; skipping generate textures");
            return;
        };
        let Some(height) = NonZeroU32::new(height) else {
            log::debug!("The window height was zero; skipping generate textures");
            return;
        };
        self.resize_and_generate_depth_texture_view_and_msaa_view(viewport_id, width, height);
    }

    /// Returns the maximum texture dimension supported if known
    ///
    /// This API will only return a known dimension after `set_window()` has been called
    /// at least once, since the underlying device and render state are initialized lazily
    /// once we have a window (that may determine the choice of adapter/device).
    pub fn max_texture_side(&self) -> Option<usize> {
        self.render_state
            .as_ref()
            .map(|rs| rs.device.limits().max_texture_dimension_2d as usize)
    }

    /// The largest side a surface may be configured at, or no limit
    /// before the device exists (nothing is configured then).
    fn max_surface_side(&self) -> u32 {
        self.render_state
            .as_ref()
            .map_or(u32::MAX, |rs| rs.device.limits().max_texture_dimension_2d)
    }

    fn resize_and_generate_depth_texture_view_and_msaa_view(
        &mut self,
        viewport_id: ViewportId,
        width_in_pixels: NonZeroU32,
        height_in_pixels: NonZeroU32,
    ) {
        profiling::function_scope!();

        // Configure the surface, and size the depth and MSAA textures,
        // within the device's limit; `surface_fit` keeps a non-zero side non-zero.
        let max_side = self.max_surface_side();
        let (window_width, window_height) = (width_in_pixels.get(), height_in_pixels.get());
        let (width, height, render_scale) = surface_fit(window_width, window_height, max_side);

        let render_state = self.render_state.as_ref().unwrap();
        let surface_state = self.surfaces.get_mut(&viewport_id).unwrap();

        if render_scale < 1.0 && surface_state.render_scale >= 1.0 {
            log::warn!(
                "the window is {window_width} × {window_height} pixels, more than this GPU draws \
                 ({max_side} a side): drawing it at {width} × {height} and scaling it up"
            );
        }
        surface_state.width = width;
        surface_state.height = height;
        surface_state.window_width = window_width;
        surface_state.window_height = window_height;
        surface_state.render_scale = render_scale;

        Self::configure_surface(surface_state, render_state, &self.config.surface);

        if let Some(depth_format) = self.options.depth_stencil_format {
            self.depth_texture_view.insert(
                viewport_id,
                render_state
                    .device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("egui_depth_texture"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: self.options.msaa_samples.max(1),
                        dimension: wgpu::TextureDimension::D2,
                        format: depth_format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[depth_format],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            );
        }

        if let Some(render_state) = (self.options.msaa_samples > 1)
            .then_some(self.render_state.as_ref())
            .flatten()
        {
            let texture_format = render_state.target_format;
            self.msaa_texture_view.insert(
                viewport_id,
                render_state
                    .device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("egui_msaa_texture"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: self.options.msaa_samples.max(1),
                        dimension: wgpu::TextureDimension::D2,
                        format: texture_format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[texture_format],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            );
        }
    }

    /// Handles changes of the resizing state.
    ///
    /// Should be called prior to the first [`Painter::on_window_resized`] call and after the last in
    /// the chain. Used to apply platform-specific logic, e.g. OSX Metal window resize jitter fix.
    pub fn on_window_resize_state_change(&mut self, viewport_id: ViewportId, resizing: bool) {
        profiling::function_scope!();

        let Some(state) = self.surfaces.get_mut(&viewport_id) else {
            return;
        };
        if state.resizing == resizing {
            if resizing {
                log::debug!(
                    "Painter::on_window_resize_state_change() redundant call while resizing"
                );
            } else {
                log::debug!(
                    "Painter::on_window_resize_state_change() redundant call after resizing"
                );
            }
            return;
        }

        // Set before reconfiguring so macOS live resize uses the temporary latency bump above.
        state.resizing = resizing;

        // Resizing is a bit tricky on macOS.
        // It requires enabling ["present_with_transaction"](https://developer.apple.com/documentation/quartzcore/cametallayer/presentswithtransaction)
        // flag to avoid jittering during the resize. Even though resize jittering on macOS
        // is common across rendering backends, the solution for wgpu/metal is known.
        //
        // See https://github.com/emilk/egui/issues/903
        #[cfg(all(target_os = "macos", feature = "macos-window-resize-jitter-fix"))]
        {
            // SAFETY: `as_hal::<Metal>()` returns `None` unless this surface is backed by wgpu's
            // Metal backend.
            unsafe {
                if let (Some(render_state), Some(hal_surface)) = (
                    self.render_state.as_ref(),
                    state.surface.as_hal::<wgpu::hal::api::Metal>(),
                ) {
                    hal_surface
                        .render_layer()
                        .lock()
                        .setPresentsWithTransaction(resizing);

                    Self::configure_surface(state, render_state, &self.config.surface);
                }
            }
        }
    }

    pub fn on_window_resized(
        &mut self,
        viewport_id: ViewportId,
        width_in_pixels: NonZeroU32,
        height_in_pixels: NonZeroU32,
    ) {
        profiling::function_scope!();

        if self.surfaces.contains_key(&viewport_id) {
            self.resize_and_generate_depth_texture_view_and_msaa_view(
                viewport_id,
                width_in_pixels,
                height_in_pixels,
            );
        } else {
            log::warn!(
                "Ignoring window resize notification with no surface created via Painter::set_window()"
            );
        }
    }

    /// Returns two things:
    ///
    /// The approximate number of seconds spent on vsync-waiting (if any),
    /// and the captures captured screenshot if it was requested.
    ///
    /// If `capture_data` isn't empty, a screenshot will be captured.
    #[expect(clippy::too_many_arguments)]
    pub fn paint_and_update_textures(
        &mut self,
        viewport_id: ViewportId,
        pixels_per_point: f32,
        clear_color: [f32; 4],
        clipped_primitives: &[epaint::ClippedPrimitive],
        textures_delta: &mut epaint::textures::TexturesDelta,
        capture_data: Vec<egui::ScreenshotCallback>,
        window: &Arc<winit::window::Window>,
    ) -> f32 {
        profiling::function_scope!();

        /// Guard to ensure that commands are always submitted to the renderer queue
        /// so that calls to [`write_buffer()`](https://docs.rs/wgpu/latest/wgpu/struct.Queue.html#method.write_buffer)
        /// are completed even if we take a codepath which doesn't submit commands and avoids
        /// internal buffers growing indefinitely.
        ///
        /// This may happen, for example, if no output frame is resolved.
        /// See <https://github.com/emilk/egui/pull/7928> for full context.
        struct RendererQueueGuard<'q> {
            queue: &'q wgpu::Queue,
            commands_submitted: bool,
        }

        impl Drop for RendererQueueGuard<'_> {
            fn drop(&mut self) {
                // Only submit an empty command buffer array if no commands were
                // explicitly submitted.
                if !self.commands_submitted {
                    self.queue.submit([]);
                }
            }
        }

        let capture = !capture_data.is_empty();
        let mut vsync_sec = 0.0;

        // If the previous frame produced `CurrentSurfaceTexture::Lost`, the action match
        // below set `needs_recreate`. Recreate the surface now, before re-borrowing
        // `self.render_state` / `self.surfaces` for the rest of the paint.
        if self
            .surfaces
            .get(&viewport_id)
            .is_some_and(|s| s.needs_recreate)
            && let Err(err) = self.recreate_surface(viewport_id, window)
        {
            log::error!("Failed to recreate surface for {viewport_id:?}: {err}");
            return vsync_sec;
        }

        let Some(render_state) = self.render_state.as_mut() else {
            return vsync_sec;
        };

        let mut render_queue_guard = RendererQueueGuard {
            queue: &render_state.queue,
            commands_submitted: false,
        };

        let Some(surface_state) = self.surfaces.get_mut(&viewport_id) else {
            return vsync_sec;
        };

        let mut encoder =
            render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });

        // Upload all resources for the GPU.
        // A surface fitted within the GPU's limit is drawn smaller.
        let screen_descriptor = renderer::ScreenDescriptor {
            size_in_pixels: [surface_state.width, surface_state.height],
            pixels_per_point: pixels_per_point * surface_state.render_scale,
        };

        let user_cmd_bufs = {
            let mut renderer = render_state.renderer.write();
            #[expect(clippy::iter_over_hash_type)] // Order doesn't matter here
            for (id, image_deltas) in textures_delta.set.drain() {
                for image_delta in image_deltas {
                    renderer.update_texture(
                        &render_state.device,
                        &render_state.queue,
                        id,
                        &image_delta,
                    );
                }
            }

            renderer.update_buffers(
                &render_state.device,
                &render_state.queue,
                &mut encoder,
                clipped_primitives,
                &screen_descriptor,
            )
        };

        if surface_state.needs_reconfigure {
            Self::configure_surface(surface_state, render_state, &self.config.surface);
            surface_state.needs_reconfigure = false;
        }

        let output_frame = {
            profiling::scope!("get_current_texture");
            // This is what vsync-waiting happens on my Mac.
            let start = web_time::Instant::now();
            let output_frame = surface_state.surface.get_current_texture();
            vsync_sec += start.elapsed().as_secs_f32();
            output_frame
        };

        // `None` when there is no frame to present to. A capture is rendered to its own
        // texture and copied back to the cpu, so it is served even then — that is how a hidden
        // window, which the compositor gives no surface texture, is screenshotted at all.
        let output_frame = match output_frame {
            wgpu::CurrentSurfaceTexture::Success(frame) => Some(frame),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                surface_state.needs_reconfigure = true;
                Some(frame)
            }
            other => {
                match (*self.config.on_surface_status)(&other) {
                    SurfaceErrorAction::Reconfigure => {
                        Self::configure_surface(surface_state, render_state, &self.config.surface);
                        self.context.request_repaint_of(viewport_id);
                    }
                    SurfaceErrorAction::RecreateSurface => {
                        // Because of ownership, I could not find an easy way to do a full recovery here,
                        // as that would involve dropping the old surface and creating a new one.
                        // For now, we defer the recreation to the beginning of the next frame (which
                        // we ensure to arrive via `request_repaint_of`). A cleaner solution would be
                        // to untangle the ownership of `RenderState`.
                        surface_state.needs_recreate = true;
                        self.context.request_repaint_of(viewport_id);
                    }
                    SurfaceErrorAction::SkipFrame => {}
                }
                if !capture {
                    return vsync_sec;
                }
                None
            }
        };

        let mut capture_buffer = None;
        {
            let renderer = render_state.renderer.read();

            let target_texture = if capture {
                // The frame we would have presented decides the capture, when there is one.
                // Without it we have only the surface we configured, which is the same thing
                // as long as no resize is in flight.
                let (size, format) = match &output_frame {
                    Some(output_frame) => {
                        (output_frame.texture.size(), output_frame.texture.format())
                    }
                    None => (
                        wgpu::Extent3d {
                            width: surface_state.width,
                            height: surface_state.height,
                            depth_or_array_layers: 1,
                        },
                        render_state.target_format,
                    ),
                };
                if size.width == 0 || size.height == 0 {
                    log::warn!("Cannot capture a screenshot of a zero-sized surface");
                    return vsync_sec;
                }

                let capture_state = self
                    .screen_capture_state
                    .get_or_insert_with(|| CaptureState::new(&render_state.device, size, format));
                capture_state.update(&render_state.device, size);

                &capture_state.texture
            } else if let Some(output_frame) = &output_frame {
                &output_frame.texture
            } else {
                // Unreachable: without a frame to present to, and with nothing to capture,
                // we returned above.
                return vsync_sec;
            };
            let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

            let (view, resolve_target) = (self.options.msaa_samples > 1)
                .then_some(self.msaa_texture_view.get(&viewport_id))
                .flatten()
                .map_or((&target_view, None), |texture_view| {
                    (texture_view, Some(&target_view))
                });

            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui_render"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color[0] as f64,
                            g: clear_color[1] as f64,
                            b: clear_color[2] as f64,
                            a: clear_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: self.depth_texture_view.get(&viewport_id).map(|view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: self
                            .options
                            .depth_stencil_format
                            .is_some_and(|depth_stencil_format| {
                                depth_stencil_format.has_depth_aspect()
                            })
                            .then_some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                // It is very unlikely that the depth buffer is needed after egui finished rendering
                                // so no need to store it. (this can improve performance on tiling GPUs like mobile chips or Apple Silicon)
                                store: wgpu::StoreOp::Discard,
                            }),
                        stencil_ops: self
                            .options
                            .depth_stencil_format
                            .is_some_and(|depth_stencil_format| {
                                depth_stencil_format.has_stencil_aspect()
                            })
                            .then_some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(0),
                                store: wgpu::StoreOp::Discard,
                            }),
                    }
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Forgetting the pass' lifetime means that we are no longer compile-time protected from
            // runtime errors caused by accessing the parent encoder before the render pass is dropped.
            // Since we don't pass it on to the renderer, we should be perfectly safe against this mistake here!
            renderer.render(
                &mut render_pass.forget_lifetime(),
                clipped_primitives,
                &screen_descriptor,
            );

            if capture && let Some(capture_state) = &mut self.screen_capture_state {
                capture_buffer = Some(capture_state.copy_textures(
                    &render_state.device,
                    output_frame.as_ref(),
                    &mut encoder,
                ));
            }
        }

        let encoded = {
            profiling::scope!("CommandEncoder::finish");
            encoder.finish()
        };

        // Submit the commands: both the main buffer and user-defined ones.
        {
            profiling::scope!("Queue::submit");
            // wgpu doesn't document where vsync can happen. Maybe here?
            let start = web_time::Instant::now();
            render_state
                .queue
                .submit(core::iter::chain(user_cmd_bufs, [encoded]));
            vsync_sec += start.elapsed().as_secs_f32();
        };

        // Ensure that the queue guard does not do unnecessary work when dropped
        render_queue_guard.commands_submitted = true;

        // Free textures marked for destruction **after** queue submit since they might still be used in the current frame.
        // Calling `wgpu::Texture::destroy` on a texture that is still in use would invalidate the command buffer(s) it is used in.
        // However, once we called `wgpu::Queue::submit`, it is up for wgpu to determine how long the underlying gpu resource has to live.
        {
            let mut renderer = render_state.renderer.write();
            #[expect(clippy::iter_over_hash_type)] // Order doesn't matter here
            for id in textures_delta.free.drain() {
                renderer.free_texture(&id);
            }
        }

        if let Some(capture_buffer) = capture_buffer
            && let Some(screen_capture_state) = &mut self.screen_capture_state
        {
            screen_capture_state.read_screen_rgba(capture_buffer, capture_data);
        }

        if let Some(output_frame) = output_frame {
            window.pre_present_notify();

            profiling::scope!("present");
            // wgpu doesn't document where vsync can happen. Maybe here?
            let start = web_time::Instant::now();
            render_state.queue.present(output_frame);
            vsync_sec += start.elapsed().as_secs_f32();
        }

        vsync_sec
    }

    pub fn gc_viewports(&mut self, active_viewports: &ViewportIdSet) {
        self.surfaces.retain(|id, _| active_viewports.contains(id));
        self.depth_texture_view
            .retain(|id, _| active_viewports.contains(id));
        self.msaa_texture_view
            .retain(|id, _| active_viewports.contains(id));
    }

    #[expect(clippy::needless_pass_by_ref_mut, clippy::unused_self)]
    pub fn destroy(&mut self) {
        // TODO(emilk): something here?
    }
}

#[cfg(test)]
mod tests {
    use super::surface_fit;

    #[test]
    fn surface_fit_keeps_windows_within_the_limit() {
        assert_eq!(surface_fit(1920, 1080, 8192), (1920, 1080, 1.0));
        assert_eq!(surface_fit(8192, 100, 8192), (8192, 100, 1.0));

        let (w, h, scale) = surface_fit(16384, 4000, 8192);
        assert_eq!((w, h), (8192, 2000));
        assert!((scale - 0.5).abs() < 1e-6);

        // A thin side never becomes zero, and an empty side stays empty.
        assert_eq!(surface_fit(100_000, 1, 8192).1, 1);
        assert_eq!(surface_fit(100_000, 0, 8192).1, 0);
        // A zero limit is treated as one pixel.
        assert_eq!(surface_fit(10, 10, 0), (1, 1, 0.1));
    }
}
