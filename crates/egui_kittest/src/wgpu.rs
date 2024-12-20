use std::{iter::once, sync::Arc};

use image::RgbaImage;

use egui_wgpu::{
    wgpu::{self, StoreOp, TextureFormat},
    ScreenDescriptor,
};

use crate::{texture_to_image::texture_to_image, Harness};

/// Utility to render snapshots from a [`Harness`] using [`egui_wgpu`].
pub struct TestRenderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    dithering: bool,
}

impl TestRenderer {
    /// Create a new [`TestRenderer`] using a [`egui_wgpu::WgpuSetup`].
    pub fn new(wgpu_setup: &egui_wgpu::WgpuSetup) -> Self {
        let (device, queue) = match wgpu_setup {
            egui_wgpu::WgpuSetup::CreateNew(egui_wgpu::WgpuSetupCreateNew {
                supported_backends,
                power_preference,
                device_descriptor,
                trace_path,
                native_adapter_selector,
            }) => {
                let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                    backends: *supported_backends,
                    ..Default::default()
                });

                let adapter = if let Some(native_adapter_selector) = native_adapter_selector {
                    let adapters = instance
                        .enumerate_adapters(*supported_backends)
                        .into_iter()
                        .map(Arc::new)
                        .collect::<Vec<_>>();
                    native_adapter_selector(&adapters, None).expect("No adapter found.")
                } else {
                    Arc::new(
                        pollster::block_on(instance.request_adapter(
                            &wgpu::RequestAdapterOptions {
                                power_preference: *power_preference,
                                force_fallback_adapter: false,
                                compatible_surface: None,
                            },
                        ))
                        .expect("No adapter found using `request_adapter`"),
                    )
                };

                let device_descriptor = device_descriptor(&adapter);
                let (device, queue) = pollster::block_on(
                    adapter.request_device(&device_descriptor, trace_path.as_deref()),
                )
                .expect("Failed to request device");

                (Arc::new(device), Arc::new(queue))
            }
            egui_wgpu::WgpuSetup::Existing { device, queue, .. } => (device.clone(), queue.clone()),
        };

        Self::create(device, queue)
    }

    /// Create a new [`TestRenderer`] using the provided [`wgpu::Device`] and [`wgpu::Queue`].
    pub fn create(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
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
                    ..Default::default()
                })
                .forget_lifetime();

            renderer.render(&mut pass, &tessellated, &screen);
        }

        self.queue
            .submit(user_buffers.into_iter().chain(once(encoder.finish())));

        self.device.poll(wgpu::Maintain::Wait);

        texture_to_image(&self.device, &self.queue, &texture)
    }
}
