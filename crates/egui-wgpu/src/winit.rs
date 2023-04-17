use std::sync::Arc;

use epaint::{self, mutex::RwLock};

use tracing::error;

use crate::{renderer, RenderState, Renderer, SurfaceErrorAction, WgpuConfiguration};

struct SurfaceState {
    surface: wgpu::Surface,
    alpha_mode: wgpu::CompositeAlphaMode,
    width: u32,
    height: u32,
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
/// Alternatively you can use [`crate::renderer`] directly.
pub struct Painter {
    configuration: WgpuConfiguration,
    msaa_samples: u32,
    support_transparent_backbuffer: bool,
    depth_format: Option<wgpu::TextureFormat>,
    depth_texture_view: Option<wgpu::TextureView>,
    msaa_texture_view: Option<wgpu::TextureView>,
    screen_capture_state: Option<CaptureState>,

    instance: wgpu::Instance,
    adapter: Option<wgpu::Adapter>,
    render_state: Option<RenderState>,
    surface_state: Option<SurfaceState>,
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
        depth_bits: u8,
        support_transparent_backbuffer: bool,
    ) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: configuration.backends,
            dx12_shader_compiler: Default::default(), //
        });

        Self {
            configuration,
            msaa_samples,
            support_transparent_backbuffer,
            depth_format: (depth_bits > 0).then_some(wgpu::TextureFormat::Depth32Float),
            depth_texture_view: None,
            screen_capture_state: None,

            instance,
            adapter: None,
            render_state: None,
            surface_state: None,
            msaa_texture_view: None,
        }
    }

    /// Get the [`RenderState`].
    ///
    /// Will return [`None`] if the render state has not been initialized yet.
    pub fn render_state(&self) -> Option<RenderState> {
        self.render_state.clone()
    }

    async fn init_render_state(
        &self,
        adapter: &wgpu::Adapter,
        target_format: wgpu::TextureFormat,
    ) -> Result<RenderState, wgpu::RequestDeviceError> {
        adapter
            .request_device(&self.configuration.device_descriptor, None)
            .await
            .map(|(device, queue)| {
                let renderer =
                    Renderer::new(&device, target_format, self.depth_format, self.msaa_samples);
                RenderState {
                    device: Arc::new(device),
                    queue: Arc::new(queue),
                    target_format,
                    renderer: Arc::new(RwLock::new(renderer)),
                }
            })
    }

    // We want to defer the initialization of our render state until we have a surface
    // so we can take its format into account.
    //
    // After we've initialized our render state once though we expect all future surfaces
    // will have the same format and so this render state will remain valid.
    async fn ensure_render_state_for_surface(
        &mut self,
        surface: &wgpu::Surface,
    ) -> Result<(), wgpu::RequestDeviceError> {
        if self.adapter.is_none() {
            self.adapter = self
                .instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: self.configuration.power_preference,
                    compatible_surface: Some(surface),
                    force_fallback_adapter: false,
                })
                .await;
        }
        if self.render_state.is_none() {
            match &self.adapter {
                Some(adapter) => {
                    let swapchain_format = crate::preferred_framebuffer_format(
                        &surface.get_capabilities(adapter).formats,
                    );
                    let rs = self.init_render_state(adapter, swapchain_format).await?;
                    self.render_state = Some(rs);
                }
                None => return Err(wgpu::RequestDeviceError {}),
            }
        }
        Ok(())
    }

    fn configure_surface(
        surface_state: &SurfaceState,
        render_state: &RenderState,
        present_mode: wgpu::PresentMode,
    ) {
        surface_state.surface.configure(
            &render_state.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
                format: render_state.target_format,
                width: surface_state.width,
                height: surface_state.height,
                present_mode,
                alpha_mode: surface_state.alpha_mode,
                view_formats: vec![render_state.target_format],
            },
        );
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
    /// # Safety
    ///
    /// The raw Window handle associated with the given `window` must be a valid object to create a
    /// surface upon and must remain valid for the lifetime of the created surface. (The surface may
    /// be cleared by passing `None`).
    ///
    /// # Errors
    /// If the provided wgpu configuration does not match an available device.
    pub async unsafe fn set_window(
        &mut self,
        window: Option<&winit::window::Window>,
    ) -> Result<(), crate::WgpuError> {
        match window {
            Some(window) => {
                let surface = self.instance.create_surface(&window)?;

                self.ensure_render_state_for_surface(&surface).await?;

                let alpha_mode = if self.support_transparent_backbuffer {
                    let supported_alpha_modes = surface
                        .get_capabilities(self.adapter.as_ref().unwrap())
                        .alpha_modes;

                    // Prefer pre multiplied over post multiplied!
                    if supported_alpha_modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied) {
                        wgpu::CompositeAlphaMode::PreMultiplied
                    } else if supported_alpha_modes
                        .contains(&wgpu::CompositeAlphaMode::PostMultiplied)
                    {
                        wgpu::CompositeAlphaMode::PostMultiplied
                    } else {
                        tracing::warn!("Transparent window was requested, but the active wgpu surface does not support a `CompositeAlphaMode` with transparency.");
                        wgpu::CompositeAlphaMode::Auto
                    }
                } else {
                    wgpu::CompositeAlphaMode::Auto
                };

                let size = window.inner_size();
                self.surface_state = Some(SurfaceState {
                    surface,
                    width: size.width,
                    height: size.height,
                    alpha_mode,
                });
                self.resize_and_generate_depth_texture_view_and_msaa_view(size.width, size.height);
            }
            None => {
                self.surface_state = None;
            }
        }
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
        width_in_pixels: u32,
        height_in_pixels: u32,
    ) {
        let render_state = self.render_state.as_ref().unwrap();
        let surface_state = self.surface_state.as_mut().unwrap();

        surface_state.width = width_in_pixels;
        surface_state.height = height_in_pixels;

        Self::configure_surface(surface_state, render_state, self.configuration.present_mode);

        self.depth_texture_view = self.depth_format.map(|depth_format| {
            render_state
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("egui_depth_texture"),
                    size: wgpu::Extent3d {
                        width: width_in_pixels,
                        height: height_in_pixels,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: depth_format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[depth_format],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        });

        self.msaa_texture_view = (self.msaa_samples > 1)
            .then_some(self.render_state.as_ref())
            .flatten()
            .map(|render_state| {
                let texture_format = render_state.target_format;
                render_state
                    .device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some("egui_msaa_texture"),
                        size: wgpu::Extent3d {
                            width: width_in_pixels,
                            height: height_in_pixels,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: self.msaa_samples,
                        dimension: wgpu::TextureDimension::D2,
                        format: texture_format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[texture_format],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default())
            });
    }

    pub fn on_window_resized(&mut self, width_in_pixels: u32, height_in_pixels: u32) {
        if self.surface_state.is_some() {
            self.resize_and_generate_depth_texture_view_and_msaa_view(
                width_in_pixels,
                height_in_pixels,
            );
        } else {
            error!("Ignoring window resize notification with no surface created via Painter::set_window()");
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
                    bytes_per_row: Some(std::num::NonZeroU32::new(padding.padded_bytes_per_row)?),
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
                tracing::error!("Screen can't be captured unless the surface format is Rgba8Unorm or Bgra8Unorm. Current surface format is {:?}", tex.format());
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

    // Returns a vector with the frame's pixel data if it was requested.
    pub fn paint_and_update_textures(
        &mut self,
        pixels_per_point: f32,
        clear_color: [f32; 4],
        clipped_primitives: &[epaint::ClippedPrimitive],
        textures_delta: &epaint::textures::TexturesDelta,
        capture: bool,
    ) -> Option<epaint::ColorImage> {
        crate::profile_function!();

        let render_state = self.render_state.as_mut()?;
        let surface_state = self.surface_state.as_ref()?;

        let output_frame = {
            crate::profile_scope!("get_current_texture");
            // This is what vsync-waiting happens, at least on Mac.
            surface_state.surface.get_current_texture()
        };

        let output_frame = match output_frame {
            Ok(frame) => frame,
            #[allow(clippy::single_match_else)]
            Err(e) => match (*self.configuration.on_surface_error)(e) {
                SurfaceErrorAction::RecreateSurface => {
                    Self::configure_surface(
                        surface_state,
                        render_state,
                        self.configuration.present_mode,
                    );
                    return None;
                }
                SurfaceErrorAction::SkipFrame => {
                    return None;
                }
            },
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

        {
            let renderer = render_state.renderer.read();
            let frame_view = if capture {
                Self::update_capture_state(
                    &mut self.screen_capture_state,
                    &output_frame,
                    render_state,
                );
                self.screen_capture_state
                    .as_ref()?
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default())
            } else {
                output_frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default())
            };

            let (view, resolve_target) = (self.msaa_samples > 1)
                .then_some(self.msaa_texture_view.as_ref())
                .flatten()
                .map_or((&frame_view, None), |texture_view| {
                    (texture_view, Some(&frame_view))
                });

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                        store: true,
                    },
                })],
                depth_stencil_attachment: self.depth_texture_view.as_ref().map(|view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }
                }),
                label: Some("egui_render"),
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
            render_state
                .queue
                .submit(user_cmd_bufs.into_iter().chain(std::iter::once(encoded)));
        };

        let screenshot = if capture {
            let screen_capture_state = self.screen_capture_state.as_ref()?;
            Self::read_screen_rgba(screen_capture_state, render_state, &output_frame)
        } else {
            None
        };
        // Redraw egui
        {
            crate::profile_scope!("present");
            output_frame.present();
        }
        screenshot
    }

    #[allow(clippy::unused_self)]
    pub fn destroy(&mut self) {
        // TODO(emilk): something here?
    }
}
