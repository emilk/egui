use crate::renderer;

/// Everything you need to paint egui with [`wgpu`] on [`winit`].
///
/// Alternatively you can use [`crate::renderer`] directly.
pub struct Painter {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface,
    egui_rpass: renderer::RenderPass,
}

impl Painter {
    /// Creates a [`wgpu`] surface for the given window, and things required to render egui onto it.
    ///
    /// # Safety
    /// The given `window` must outlive the returned [`Painter`].
    pub unsafe fn new(window: &winit::window::Window, msaa_samples: u32) -> Self {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY | wgpu::Backends::GL);
        let surface = instance.create_surface(&window);

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::default(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ))
        .unwrap();

        let size = window.inner_size();
        let surface_format = surface.get_preferred_format(&adapter).unwrap();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width as u32,
            height: size.height as u32,
            present_mode: wgpu::PresentMode::Fifo, // TODO: make vsync configurable
        };
        surface.configure(&device, &surface_config);

        let egui_rpass = renderer::RenderPass::new(&device, surface_format, msaa_samples);

        Self {
            device,
            queue,
            surface_config,
            surface,
            egui_rpass,
        }
    }

    pub fn max_texture_side(&self) -> usize {
        self.device.limits().max_texture_dimension_2d as usize
    }

    pub fn on_window_resized(&mut self, width_in_pixels: u32, height_in_pixels: u32) {
        self.surface_config.width = width_in_pixels;
        self.surface_config.height = height_in_pixels;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn paint_and_update_textures(
        &mut self,
        pixels_per_point: f32,
        clear_color: egui::Rgba,
        clipped_primitives: &[egui::ClippedPrimitive],
        textures_delta: &egui::TexturesDelta,
    ) {
        let output_frame = match self.surface.get_current_texture() {
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
        let output_view = output_frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        // Upload all resources for the GPU.
        let screen_descriptor = renderer::ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point,
        };

        for (id, image_delta) in &textures_delta.set {
            self.egui_rpass
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }
        for id in &textures_delta.free {
            self.egui_rpass.free_texture(id);
        }

        self.egui_rpass.update_buffers(
            &self.device,
            &self.queue,
            clipped_primitives,
            &screen_descriptor,
        );

        // Record all render passes.
        self.egui_rpass.execute(
            &mut encoder,
            &output_view,
            clipped_primitives,
            &screen_descriptor,
            Some(wgpu::Color {
                r: clear_color.r() as f64,
                g: clear_color.g() as f64,
                b: clear_color.b() as f64,
                a: clear_color.a() as f64,
            }),
        );

        // Submit the commands.
        self.queue.submit(std::iter::once(encoder.finish()));

        // Redraw egui
        output_frame.present();
    }

    #[allow(clippy::unused_self)]
    pub fn destroy(&mut self) {
        // TODO: something here?
    }
}
