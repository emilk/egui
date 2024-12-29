use crate::texture_to_image::texture_to_image;
use crate::Harness;
use egui_wgpu::wgpu::{Backends, InstanceDescriptor, StoreOp, TextureFormat};
use egui_wgpu::{wgpu, ScreenDescriptor, WgpuSetup};
use image::RgbaImage;
use std::iter::once;
use std::sync::Arc;
use wgpu::Maintain;

// TODO: Replace this with the setup from https://github.com/emilk/egui/pull/5506
pub fn default_wgpu_setup() -> egui_wgpu::WgpuSetup {
    egui_wgpu::WgpuSetup::CreateNew {
        supported_backends: Backends::all(),
        device_descriptor: Arc::new(|a| wgpu::DeviceDescriptor::default()),
        power_preference: wgpu::PowerPreference::default(),
    }
}


pub(crate) fn create_render_state(setup: WgpuSetup) -> egui_wgpu::RenderState {
    let instance = match &setup {
        WgpuSetup::Existing { instance, .. } => instance.clone(),
        _ => Default::default(),
    };

    pollster::block_on(egui_wgpu::RenderState::create(
        &egui_wgpu::WgpuConfiguration {
            wgpu_setup: setup,
            ..Default::default()
        },
        &instance,
        None,
        None,
        1,
        false,
    ))
        .expect("Failed to create render state")
}

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
    pub fn render<State>(harness: &mut Harness<'_, State>) -> RgbaImage {
        let render_state = harness.render_state.get_or_insert_with(|| {
            create_render_state(default_wgpu_setup())
        }).clone();

        let mut renderer = render_state.renderer.write();

        for delta in &harness.texture_deltas {
            for (id, image_delta) in &delta.set {
                renderer.update_texture(&render_state.device, &render_state.queue, *id, image_delta);
            }
        }

        let mut encoder = render_state
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
            &render_state.device,
            &render_state.queue,
            &mut encoder,
            &tessellated,
            &screen,
        );

        let texture = render_state.device.create_texture(&wgpu::TextureDescriptor {
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

        render_state.queue
            .submit(user_buffers.into_iter().chain(once(encoder.finish())));

        render_state.device.poll(Maintain::Wait);

        texture_to_image(&render_state.device, &render_state.queue, &texture)
    }
}
