use crate::RenderState;
use egui::UserData;
use epaint::ColorImage;
use std::sync::{mpsc, Arc};
use wgpu::{MultisampleState, StoreOp};

/// A texture and a buffer for reading the rendered frame back to the cpu.
/// The texture is required since [`wgpu::TextureUsages::COPY_DST`] is not an allowed
/// flag for the surface texture on all platforms. This means that anytime we want to
/// capture the frame, we first render it to this texture, and then we can copy it to
/// both the surface texture and the buffer, from where we can pull it back to the cpu.
pub struct CaptureState {
    pub texture: wgpu::Texture,
    padding: BufferPadding,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    buffer: Option<wgpu::Buffer>,
}

impl CaptureState {
    pub fn new(device: &Arc<wgpu::Device>, surface_texture: &wgpu::Texture) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("egui_screen_capture_texture"),
            size: surface_texture.size(),
            mip_level_count: surface_texture.mip_level_count(),
            sample_count: surface_texture.sample_count(),
            dimension: surface_texture.dimension(),
            format: surface_texture.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let padding = BufferPadding::new(surface_texture.width());

        let shader = device.create_shader_module(wgpu::include_wgsl!("blit.wgsl"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(surface_texture.format().into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mip"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let view = texture.create_view(&Default::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: None,
        });

        Self {
            texture,
            padding,
            pipeline,
            bind_group,
            buffer: None,
        }
    }

    // CaptureState only needs to be updated when the size of the two textures don't match, and we want to
    // capture a frame
    pub fn update_capture_state(
        screen_capture_state: &mut Option<Self>,
        surface_texture: &wgpu::SurfaceTexture,
        render_state: &RenderState,
    ) {
        let surface_texture = &surface_texture.texture;
        match screen_capture_state {
            Some(capture_state) => {
                if capture_state.texture.size() != surface_texture.size() {
                    *capture_state = Self::new(&render_state.device, surface_texture);
                }
            }
            None => {
                *screen_capture_state = Some(Self::new(&render_state.device, surface_texture));
            }
        }
    }

    // Handles copying from the CaptureState texture to the surface texture and the cpu
    pub fn read_screen_rgba(
        &mut self,
        ctx: egui::Context,
        render_state: &RenderState,
        output_frame: Option<&wgpu::SurfaceTexture>,
        data: Vec<UserData>,
        tx: mpsc::Sender<(Vec<UserData>, ColorImage)>,
    ) {
        // It would be more efficient to reuse the Buffer, e.g. via some kind of ring buffer, but
        // for most screenshot use cases this should be fine. When taking many screenshots (e.g. for a video)
        // it might make sense to revisit this and implement a more efficient solution.
        #[allow(clippy::arc_with_non_send_sync)]
        let buffer = Arc::new(render_state.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("egui_screen_capture_buffer"),
            size: (self.padding.padded_bytes_per_row * self.texture.height()) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }));
        let padding = self.padding;
        let tex = &mut self.texture;

        let device = &render_state.device;
        let queue = &render_state.queue;

        let tex_extent = tex.size();

        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            tex.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padding.padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            tex_extent,
        );

        if let Some(texture) = output_frame {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture.texture.create_view(&Default::default()),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        let id = queue.submit(Some(encoder.finish()));
        let buffer_clone = buffer.clone();
        let buffer_slice = buffer_clone.slice(..);
        let format = tex.format();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            if let Err(err) = result {
                log::error!("Failed to map buffer for reading: {:?}", err);
                return;
            }
            let to_rgba = match format {
                wgpu::TextureFormat::Rgba8Unorm => [0, 1, 2, 3],
                wgpu::TextureFormat::Bgra8Unorm => [2, 1, 0, 3],
                _ => {
                    log::error!("Screen can't be captured unless the surface format is Rgba8Unorm or Bgra8Unorm. Current surface format is {:?}", format);
                    return;
                }
            };
            let buffer_slice = buffer.slice(..);

            let mut pixels = Vec::with_capacity((tex_extent.width * tex_extent.height) as usize);
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

            tx.send((
                data,
                ColorImage {
                    size: [tex_extent.width as usize, tex_extent.height as usize],
                    pixels,
                },
            )).ok();
            ctx.request_repaint();
        });
        device.poll(wgpu::Maintain::WaitForSubmissionIndex(id));
    }

    // Handles copying from the CaptureState texture to the surface texture and the cpu
    pub(crate) fn read_screen_rgba_blocking(
        &mut self,
        render_state: &RenderState,
        output_frame: &wgpu::SurfaceTexture,
    ) -> Option<ColorImage> {
        let buffer = self.buffer.get_or_insert_with(|| {
            render_state.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("egui_screen_capture_buffer"),
                size: (self.padding.padded_bytes_per_row * self.texture.height()) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            })
        });

        let device = &render_state.device;
        let queue = &render_state.queue;

        let tex_extent = self.texture.size();

        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padding.padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            tex_extent,
        );

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_frame.texture.create_view(&Default::default()),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        let id = queue.submit(Some(encoder.finish()));
        let buffer_slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            drop(sender.send(v));
        });
        device.poll(wgpu::Maintain::WaitForSubmissionIndex(id));
        receiver.recv().ok()?.ok()?;

        let to_rgba = match self.texture.format() {
            wgpu::TextureFormat::Rgba8Unorm => [0, 1, 2, 3],
            wgpu::TextureFormat::Bgra8Unorm => [2, 1, 0, 3],
            _ => {
                log::error!("Screen can't be captured unless the surface format is Rgba8Unorm or Bgra8Unorm. Current surface format is {:?}", self.texture.format());
                return None;
            }
        };

        let mut pixels =
            Vec::with_capacity((self.texture.width() * self.texture.height()) as usize);
        for padded_row in buffer_slice
            .get_mapped_range()
            .chunks(self.padding.padded_bytes_per_row as usize)
        {
            let row = &padded_row[..self.padding.unpadded_bytes_per_row as usize];
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

        Some(ColorImage {
            size: [
                self.texture.width() as usize,
                self.texture.height() as usize,
            ],
            pixels,
        })
    }
}

#[derive(Copy, Clone)]
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
