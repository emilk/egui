use std::{num::NonZeroU32, sync::Arc};

use egui::{ViewportId, ViewportIdMap, ViewportIdSet};

use crate::{renderer, RenderState, SurfaceErrorAction, WgpuConfiguration};

struct SurfaceState {
    surface: wgpu::Surface<'static>,
    alpha_mode: wgpu::CompositeAlphaMode,
    width: u32,
    height: u32,
    supports_screenshot: bool,
}

/// A texture and a buffer for reading the rendered frame back to the cpu.
/// The texture is required since [`wgpu::TextureUsages::COPY_DST`] is not an allowed
/// flag for the surface texture on all platforms. This means that anytime we want to
/// capture the frame, we first render it to this texture, and then we can copy it to
/// both the surface texture and the buffer, from where we can pull it back to the cpu.
struct CaptureState {
    texture: wgpu::Texture,
    buffer: wgpu::Buffer,
    padding: BufferPadding,
}

impl CaptureState {
    fn new(device: &Arc<wgpu::Device>, surface_texture: &wgpu::Texture) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("egui_screen_capture_texture"),
            size: surface_texture.size(),
            mip_level_count: surface_texture.mip_level_count(),
            sample_count: surface_texture.sample_count(),
            dimension: surface_texture.dimension(),
            format: surface_texture.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let padding = BufferPadding::new(surface_texture.width());

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("egui_screen_capture_buffer"),
            size: (padding.padded_bytes_per_row * texture.height()) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            texture,
            buffer,
            padding,
        }
    }
}

struct BufferPadding {
    unpadded_bytes_per_row: u32,
    padded_bytes_per_row: u32,
}

impl BufferPadding {
    fn new(width: u32) -> Self {
        let bytes_per_pixel = std::mem::size_of::<u32>() as u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let padded_bytes_per_row =
            wgpu::util::align_to(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        Self {
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }
}

/// Everything you need to paint egui with [`wgpu`] on [`winit`].
///
/// Alternatively you can use [`crate::Renderer`] directly.
///
/// NOTE: all egui viewports share the same painter.
pub struct Painter {
    configuration: WgpuConfiguration,
    msaa_samples: u32,
    support_transparent_backbuffer: bool,
    depth_format: Option<wgpu::TextureFormat>,
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
    pub fn new(
        configuration: WgpuConfiguration,
        msaa_samples: u32,
        depth_format: Option<wgpu::TextureFormat>,
        support_transparent_backbuffer: bool,
    ) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: configuration.supported_backends,
            ..Default::default()
        });

        Self {
            configuration,
            msaa_samples,
            support_transparent_backbuffer,
            depth_format,
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

    fn configure_surface(
        surface_state: &SurfaceState,
        render_state: &RenderState,
        config: &WgpuConfiguration,
    ) {
        crate::profile_function!();

        let usage = if surface_state.supports_screenshot {
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST
        } else {
            wgpu::TextureUsages::RENDER_ATTACHMENT
        };

        let width = surface_state.width;
        let height = surface_state.height;

        let mut surf_config = wgpu::SurfaceConfiguration {
            usage,
            format: render_state.target_format,
            present_mode: config.present_mode,
            alpha_mode: surface_state.alpha_mode,
            view_formats: vec![render_state.target_format],
            ..surface_state
                .surface
                .get_default_config(&render_state.adapter, width, height)
                .expect("The surface isn't supported by this adapter")
        };

        if let Some(desired_maximum_frame_latency) = config.desired_maximum_frame_latency {
            surf_config.desired_maximum_frame_latency = desired_maximum_frame_latency;
        }

        surface_state
            .surface
            .configure(&render_state.device, &surf_config);
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
        crate::profile_scope!("Painter::set_window"); // profile_function gives bad names for async functions

        if let Some(window) = window {
            let size = window.inner_size();
            if self.surfaces.get(&viewport_id).is_none() {
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
        crate::profile_scope!("Painter::set_window_unsafe"); // profile_function gives bad names for async functions

        if let Some(window) = window {
            let size = window.inner_size();
            if self.surfaces.get(&viewport_id).is_none() {
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
        let render_state = if let Some(render_state) = &self.render_state {
            render_state
        } else {
            let render_state = RenderState::create(
                &self.configuration,
                &self.instance,
                &surface,
                self.depth_format,
                self.msaa_samples,
            )
            .await?;
            self.render_state.get_or_insert(render_state)
        };
        let alpha_mode = if self.support_transparent_backbuffer {
            let supported_alpha_modes = surface.get_capabilities(&render_state.adapter).alpha_modes;

            // Prefer pre multiplied over post multiplied!
            if supported_alpha_modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied) {
                wgpu::CompositeAlphaMode::PreMultiplied
            } else if supported_alpha_modes.contains(&wgpu::CompositeAlphaMode::PostMultiplied) {
                wgpu::CompositeAlphaMode::PostMultiplied
            } else {
                log::warn!("Transparent window was requested, but the active wgpu surface does not support a `CompositeAlphaMode` with transparency.");
                wgpu::CompositeAlphaMode::Auto
            }
        } else {
            wgpu::CompositeAlphaMode::Auto
        };
        let supports_screenshot =
            !matches!(render_state.adapter.get_info().backend, wgpu::Backend::Gl);
        self.surfaces.insert(
            viewport_id,
            SurfaceState {
                surface,
                width: size.width,
                height: size.height,
                alpha_mode,
                supports_screenshot,
            },
        );
        let Some(width) = NonZeroU32::new(size.width) else {
            log::debug!("The window width was zero; skipping generate textures");
            return Ok(());
        };
        let Some(height) = NonZeroU32::new(size.height) else {
            log::debug!("The window height was zero; skipping generate textures");
            return Ok(());
        };
        self.resize_and_generate_depth_texture_view_and_msaa_view(viewport_id, width, height);
        Ok(())
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

    fn resize_and_generate_depth_texture_view_and_msaa_view(
        &mut self,
        viewport_id: ViewportId,
        width_in_pixels: NonZeroU32,
        height_in_pixels: NonZeroU32,
    ) {
        crate::profile_function!();

        let width = width_in_pixels.get();
        let height = height_in_pixels.get();

        let render_state = self.render_state.as_ref().unwrap();
        let surface_state = self.surfaces.get_mut(&viewport_id).unwrap();

        surface_state.width = width;
        surface_state.height = height;

        Self::configure_surface(surface_state, render_state, &self.configuration);

        if let Some(depth_format) = self.depth_format {
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
                        sample_count: self.msaa_samples,
                        dimension: wgpu::TextureDimension::D2,
                        format: depth_format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[depth_format],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            );
        }

        if let Some(render_state) = (self.msaa_samples > 1)
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
                        sample_count: self.msaa_samples,
                        dimension: wgpu::TextureDimension::D2,
                        format: texture_format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[texture_format],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            );
        };
    }

    pub fn on_window_resized(
        &mut self,
        viewport_id: ViewportId,
        width_in_pixels: NonZeroU32,
        height_in_pixels: NonZeroU32,
    ) {
        crate::profile_function!();

        if self.surfaces.contains_key(&viewport_id) {
            self.resize_and_generate_depth_texture_view_and_msaa_view(
                viewport_id,
                width_in_pixels,
                height_in_pixels,
            );
        } else {
            log::warn!("Ignoring window resize notification with no surface created via Painter::set_window()");
        }
    }

    // CaptureState only needs to be updated when the size of the two textures don't match and we want to
    // capture a frame
    fn update_capture_state(
        screen_capture_state: &mut Option<CaptureState>,
        surface_texture: &wgpu::SurfaceTexture,
        render_state: &RenderState,
    ) {
        let surface_texture = &surface_texture.texture;
        match screen_capture_state {
            Some(capture_state) => {
                if capture_state.texture.size() != surface_texture.size() {
                    *capture_state = CaptureState::new(&render_state.device, surface_texture);
                }
            }
            None => {
                *screen_capture_state =
                    Some(CaptureState::new(&render_state.device, surface_texture));
            }
        }
    }

    // Handles copying from the CaptureState texture to the surface texture and the cpu
    fn read_screen_rgba(
        screen_capture_state: &CaptureState,
        render_state: &RenderState,
        output_frame: &wgpu::SurfaceTexture,
    ) -> Option<epaint::ColorImage> {
        let CaptureState {
            texture: tex,
            buffer,
            padding,
        } = screen_capture_state;

        let device = &render_state.device;
        let queue = &render_state.queue;

        let tex_extent = tex.size();

        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            tex.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padding.padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            tex_extent,
        );

        encoder.copy_texture_to_texture(
            tex.as_image_copy(),
            output_frame.texture.as_image_copy(),
            tex.size(),
        );

        let id = queue.submit(Some(encoder.finish()));
        let buffer_slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            drop(sender.send(v));
        });
        device.poll(wgpu::Maintain::WaitForSubmissionIndex(id));
        receiver.recv().ok()?.ok()?;

        let to_rgba = match tex.format() {
            wgpu::TextureFormat::Rgba8Unorm => [0, 1, 2, 3],
            wgpu::TextureFormat::Bgra8Unorm => [2, 1, 0, 3],
            _ => {
                log::error!("Screen can't be captured unless the surface format is Rgba8Unorm or Bgra8Unorm. Current surface format is {:?}", tex.format());
                return None;
            }
        };

        let mut pixels = Vec::with_capacity((tex.width() * tex.height()) as usize);
        for padded_row in buffer_slice
            .get_mapped_range()
            .chunks(padding.padded_bytes_per_row as usize)
        {
            let row = &padded_row[..padding.unpadded_bytes_per_row as usize];
            for color in row.chunks(4) {
                pixels.push(epaint::Color32::from_rgba_premultiplied(
                    color[to_rgba[0]],
                    color[to_rgba[1]],
                    color[to_rgba[2]],
                    color[to_rgba[3]],
                ));
            }
        }
        buffer.unmap();

        Some(epaint::ColorImage {
            size: [tex.width() as usize, tex.height() as usize],
            pixels,
        })
    }

    /// Returns two things:
    ///
    /// The approximate number of seconds spent on vsync-waiting (if any),
    /// and the captures captured screenshot if it was requested.
    pub fn paint_and_update_textures(
        &mut self,
        viewport_id: ViewportId,
        pixels_per_point: f32,
        clear_color: [f32; 4],
        clipped_primitives: &[epaint::ClippedPrimitive],
        textures_delta: &epaint::textures::TexturesDelta,
        capture: bool,
    ) -> (f32, Option<epaint::ColorImage>) {
        crate::profile_function!();

        let mut vsync_sec = 0.0;

        let Some(render_state) = self.render_state.as_mut() else {
            return (vsync_sec, None);
        };
        let Some(surface_state) = self.surfaces.get(&viewport_id) else {
            return (vsync_sec, None);
        };

        let mut encoder =
            render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });

        // Upload all resources for the GPU.
        let screen_descriptor = renderer::ScreenDescriptor {
            size_in_pixels: [surface_state.width, surface_state.height],
            pixels_per_point,
        };

        let user_cmd_bufs = {
            let mut renderer = render_state.renderer.write();
            for (id, image_delta) in &textures_delta.set {
                renderer.update_texture(
                    &render_state.device,
                    &render_state.queue,
                    *id,
                    image_delta,
                );
            }

            renderer.update_buffers(
                &render_state.device,
                &render_state.queue,
                &mut encoder,
                clipped_primitives,
                &screen_descriptor,
            )
        };

        let capture = match (capture, surface_state.supports_screenshot) {
            (false, _) => false,
            (true, true) => true,
            (true, false) => {
                log::error!("The active render surface doesn't support taking screenshots.");
                false
            }
        };

        let output_frame = {
            crate::profile_scope!("get_current_texture");
            // This is what vsync-waiting happens on my Mac.
            let start = web_time::Instant::now();
            let output_frame = surface_state.surface.get_current_texture();
            vsync_sec += start.elapsed().as_secs_f32();
            output_frame
        };

        let output_frame = match output_frame {
            Ok(frame) => frame,
            Err(err) => match (*self.configuration.on_surface_error)(err) {
                SurfaceErrorAction::RecreateSurface => {
                    Self::configure_surface(surface_state, render_state, &self.configuration);
                    return (vsync_sec, None);
                }
                SurfaceErrorAction::SkipFrame => {
                    return (vsync_sec, None);
                }
            },
        };

        {
            let renderer = render_state.renderer.read();
            let frame_view = if capture {
                Self::update_capture_state(
                    &mut self.screen_capture_state,
                    &output_frame,
                    render_state,
                );
                self.screen_capture_state
                    .as_ref()
                    .map_or_else(
                        || &output_frame.texture,
                        |capture_state| &capture_state.texture,
                    )
                    .create_view(&wgpu::TextureViewDescriptor::default())
            } else {
                output_frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default())
            };

            let (view, resolve_target) = (self.msaa_samples > 1)
                .then_some(self.msaa_texture_view.get(&viewport_id))
                .flatten()
                .map_or((&frame_view, None), |texture_view| {
                    (texture_view, Some(&frame_view))
                });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                })],
                depth_stencil_attachment: self.depth_texture_view.get(&viewport_id).map(|view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            // It is very unlikely that the depth buffer is needed after egui finished rendering
                            // so no need to store it. (this can improve performance on tiling GPUs like mobile chips or Apple Silicon)
                            store: wgpu::StoreOp::Discard,
                        }),
                        stencil_ops: None,
                    }
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            renderer.render(&mut render_pass, clipped_primitives, &screen_descriptor);
        }

        {
            let mut renderer = render_state.renderer.write();
            for id in &textures_delta.free {
                renderer.free_texture(id);
            }
        }

        let encoded = {
            crate::profile_scope!("CommandEncoder::finish");
            encoder.finish()
        };

        // Submit the commands: both the main buffer and user-defined ones.
        {
            crate::profile_scope!("Queue::submit");
            // wgpu doesn't document where vsync can happen. Maybe here?
            let start = web_time::Instant::now();
            render_state
                .queue
                .submit(user_cmd_bufs.into_iter().chain([encoded]));
            vsync_sec += start.elapsed().as_secs_f32();
        };

        let screenshot = if capture {
            self.screen_capture_state
                .as_ref()
                .and_then(|screen_capture_state| {
                    Self::read_screen_rgba(screen_capture_state, render_state, &output_frame)
                })
        } else {
            None
        };

        {
            crate::profile_scope!("present");
            // wgpu doesn't document where vsync can happen. Maybe here?
            let start = web_time::Instant::now();
            output_frame.present();
            vsync_sec += start.elapsed().as_secs_f32();
        }

        (vsync_sec, screenshot)
    }

    pub fn gc_viewports(&mut self, active_viewports: &ViewportIdSet) {
        self.surfaces.retain(|id, _| active_viewports.contains(id));
        self.depth_texture_view
            .retain(|id, _| active_viewports.contains(id));
        self.msaa_texture_view
            .retain(|id, _| active_viewports.contains(id));
    }

    #[allow(clippy::unused_self)]
    pub fn destroy(&mut self) {
        // TODO(emilk): something here?
    }
}
