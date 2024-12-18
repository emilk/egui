use crate::texture_to_image::texture_to_image;
use crate::Harness;
use egui_wgpu::wgpu::{Backends, InstanceDescriptor, StoreOp, TextureFormat};
use egui_wgpu::{wgpu, ScreenDescriptor};
use image::RgbaImage;
use std::iter::once;
use wgpu::Maintain;

/// Utility to render snapshots from a [`Harness`] using [`egui_wgpu`].
pub struct TestRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    dithering: bool,
}

impl Default for TestRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TestRenderer {
    /// Create a new [`TestRenderer`] using a default [`wgpu::Instance`].
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(InstanceDescriptor::default());

        let adapters = instance.enumerate_adapters(Backends::all());
        let adapter = adapters.first().expect("No adapter found");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Egui Device"),
                memory_hints: Default::default(),
                required_limits: Default::default(),
                required_features: Default::default(),
            },
            None,
        ))
        .expect("Failed to create device");

        Self::create(device, queue)
    }

    /// Create a new [`TestRenderer`] using the provided [`wgpu::Device`] and [`wgpu::Queue`].
    pub fn create(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self {
            device,
            queue,
            dithering: false,
        }
    }

    /// Enable or disable dithering.
    ///
    /// Disabled by default.
    #[inline]
    pub fn with_dithering(mut self, dithering: bool) -> Self {
        self.dithering = dithering;
        self
    }

    /// Render the [`Harness`] and return the resulting image.
    pub fn render<State>(&self, harness: &Harness<'_, State>) -> RgbaImage {
        // We need to create a new renderer each time we render, since the renderer stores
        // textures related to the Harnesses' egui Context.
        // Calling the renderer from different Harnesses would cause problems if we store the renderer.
        let mut renderer = egui_wgpu::Renderer::new(
            &self.device,
            TextureFormat::Rgba8Unorm,
            None,
            1,
            self.dithering,
        );

        for delta in &harness.texture_deltas {
            for (id, image_delta) in &delta.set {
                renderer.update_texture(&self.device, &self.queue, *id, image_delta);
            }
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Egui Command Encoder"),
            });

        let size = harness.ctx.screen_rect().size() * harness.ctx.pixels_per_point();
        let screen = ScreenDescriptor {
            pixels_per_point: harness.ctx.pixels_per_point(),
            size_in_pixels: [size.x.round() as u32, size.y.round() as u32],
        };

        let tessellated = harness.ctx.tessellate(
            harness.output().shapes.clone(),
            harness.ctx.pixels_per_point(),
        );

        let user_buffers = renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &tessellated,
            &screen,
        );

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Egui Texture"),
            size: wgpu::Extent3d {
                width: screen.size_in_pixels[0],
                height: screen.size_in_pixels[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Egui Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                })
                .forget_lifetime();

            renderer.render(&mut pass, &tessellated, &screen);
        }

        self.queue
            .submit(user_buffers.into_iter().chain(once(encoder.finish())));

        self.device.poll(Maintain::Wait);

        texture_to_image(&self.device, &self.queue, &texture)
    }
}
