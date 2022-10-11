use std::sync::Arc;

use egui::mutex::RwLock;
use tracing::error;
use wgpu::{Adapter, Instance, Surface};

use crate::{renderer, RenderState, Renderer};

struct SurfaceState {
    surface: Surface,
    width: u32,
    height: u32,
}

/// Everything you need to paint egui with [`wgpu`] on [`winit`].
///
/// Alternatively you can use [`crate::renderer`] directly.
pub struct Painter<'a> {
    power_preference: wgpu::PowerPreference,
    device_descriptor: wgpu::DeviceDescriptor<'a>,
    present_mode: wgpu::PresentMode,
    msaa_samples: u32,
    depth_format: Option<wgpu::TextureFormat>,
    depth_texture_view: Option<wgpu::TextureView>,

    instance: Instance,
    adapter: Option<Adapter>,
    render_state: Option<RenderState>,
    surface_state: Option<SurfaceState>,
}

impl<'a> Painter<'a> {
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
        backends: wgpu::Backends,
        power_preference: wgpu::PowerPreference,
        device_descriptor: wgpu::DeviceDescriptor<'a>,
        present_mode: wgpu::PresentMode,
        msaa_samples: u32,
        depth_bits: u8,
    ) -> Self {
        let instance = wgpu::Instance::new(backends);

        Self {
            power_preference,
            device_descriptor,
            present_mode,
            msaa_samples,
            depth_format: (depth_bits > 0).then(|| wgpu::TextureFormat::Depth32Float),
            depth_texture_view: None,

            instance,
            adapter: None,
            render_state: None,
            surface_state: None,
        }
    }

    /// Get the [`RenderState`].
    ///
    /// Will return [`None`] if the render state has not been initialized yet.
    pub fn render_state(&self) -> Option<RenderState> {
        self.render_state.as_ref().cloned()
    }

    async fn init_render_state(
        &self,
        adapter: &Adapter,
        target_format: wgpu::TextureFormat,
    ) -> RenderState {
        let (device, queue) =
            pollster::block_on(adapter.request_device(&self.device_descriptor, None)).unwrap();

        let renderer = Renderer::new(&device, target_format, self.depth_format, self.msaa_samples);

        RenderState {
            device: Arc::new(device),
            queue: Arc::new(queue),
            target_format,
            renderer: Arc::new(RwLock::new(renderer)),
        }
    }

    // We want to defer the initialization of our render state until we have a surface
    // so we can take its format into account.
    //
    // After we've initialized our render state once though we expect all future surfaces
    // will have the same format and so this render state will remain valid.
    fn ensure_render_state_for_surface(&mut self, surface: &Surface) {
        self.adapter.get_or_insert_with(|| {
            pollster::block_on(self.instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: self.power_preference,
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            }))
            .unwrap()
        });

        if self.render_state.is_none() {
            let adapter = self.adapter.as_ref().unwrap();

            let swapchain_format =
                crate::preferred_framebuffer_format(&surface.get_supported_formats(adapter));

            let rs = pollster::block_on(self.init_render_state(adapter, swapchain_format));
            self.render_state = Some(rs);
        }
    }

    fn configure_surface(&mut self, width_in_pixels: u32, height_in_pixels: u32) {
        let render_state = self
            .render_state
            .as_ref()
            .expect("Render state should exist before surface configuration");
        let format = render_state.target_format;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width_in_pixels,
            height: height_in_pixels,
            present_mode: self.present_mode,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
        };

        let surface_state = self
            .surface_state
            .as_mut()
            .expect("Surface state should exist before surface configuration");
        surface_state
            .surface
            .configure(&render_state.device, &config);
        surface_state.width = width_in_pixels;
        surface_state.height = height_in_pixels;
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
    pub unsafe fn set_window(&mut self, window: Option<&winit::window::Window>) {
        match window {
            Some(window) => {
                let surface = self.instance.create_surface(&window);

                self.ensure_render_state_for_surface(&surface);

                let size = window.inner_size();
                let width = size.width;
                let height = size.height;
                self.surface_state = Some(SurfaceState {
                    surface,
                    width,
                    height,
                });
                self.configure_surface(width, height);
            }
            None => {
                self.surface_state = None;
            }
        }
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

    pub fn on_window_resized(&mut self, width_in_pixels: u32, height_in_pixels: u32) {
        if self.surface_state.is_some() {
            self.configure_surface(width_in_pixels, height_in_pixels);
            let device = &self.render_state.as_ref().unwrap().device;
            self.depth_texture_view = self.depth_format.map(|depth_format| {
                device
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
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default())
            });
        } else {
            error!("Ignoring window resize notification with no surface created via Painter::set_window()");
        }
    }

    pub fn paint_and_update_textures(
        &mut self,
        pixels_per_point: f32,
        clear_color: egui::Rgba,
        clipped_primitives: &[egui::ClippedPrimitive],
        textures_delta: &egui::TexturesDelta,
    ) {
        let render_state = match self.render_state.as_mut() {
            Some(rs) => rs,
            None => return,
        };
        let surface_state = match self.surface_state.as_ref() {
            Some(rs) => rs,
            None => return,
        };

        let output_frame = match surface_state.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => {
                // This error occurs when the app is minimized on Windows.
                // Silently return here to prevent spamming the console with:
                // "The underlying surface has changed, and therefore the swap chain must be updated"
                return;
            }
            Err(e) => {
                tracing::warn!("Dropped frame with error: {e}");
                return;
            }
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

        {
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
            );
        }

        {
            let renderer = render_state.renderer.read();
            let frame_view = output_frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color.r() as f64,
                            g: clear_color.g() as f64,
                            b: clear_color.b() as f64,
                            a: clear_color.a() as f64,
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

        // Submit the commands.
        render_state.queue.submit(std::iter::once(encoder.finish()));

        // Redraw egui
        output_frame.present();
    }

    #[allow(clippy::unused_self)]
    pub fn destroy(&mut self) {
        // TODO(emilk): something here?
    }
}
