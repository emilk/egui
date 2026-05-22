use egui::{UserData, ViewportId};
use epaint::ColorImage;
use std::sync::{Arc, mpsc};
use wgpu::{BindGroupLayout, MultisampleState, StoreOp};

/// A texture and a buffer for reading the rendered frame back to the cpu.
///
/// The texture is required since [`wgpu::TextureUsages::COPY_SRC`] is not an allowed
/// flag for the surface texture on all platforms. This means that anytime we want to
/// capture the frame, we first render it to this texture, and then we can copy it to
/// both the surface texture (via a render pass) and the buffer (via a texture to buffer copy),
/// from where we can pull it back
/// to the cpu.
pub struct CaptureState {
    padding: BufferPadding,
    pub texture: wgpu::Texture,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
}

pub type CaptureReceiver = mpsc::Receiver<(ViewportId, Vec<UserData>, ColorImage)>;
pub type CaptureSender = mpsc::Sender<(ViewportId, Vec<UserData>, ColorImage)>;
pub use mpsc::channel as capture_channel;

impl CaptureState {
    pub fn new(device: &wgpu::Device, surface_texture: &wgpu::Texture) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("texture_copy.wgsl"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("texture_copy"),
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
            multiview_mask: None,
            cache: None,
        });

        let bind_group_layout = pipeline.get_bind_group_layout(0);

        let (texture, padding, bind_group) =
            Self::create_texture(device, surface_texture, &bind_group_layout);

        Self {
            padding,
            texture,
            pipeline,
            bind_group,
        }
    }

    fn create_texture(
        device: &wgpu::Device,
        surface_texture: &wgpu::Texture,
        layout: &BindGroupLayout,
    ) -> (wgpu::Texture, BufferPadding, wgpu::BindGroup) {
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

        let view = texture.create_view(&Default::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            }],
            label: None,
        });

        (texture, padding, bind_group)
    }

    /// Updates the [`CaptureState`] if the size of the surface texture has changed
    pub fn update(&mut self, device: &wgpu::Device, texture: &wgpu::Texture) {
        if self.texture.size() != texture.size() {
            let (new_texture, padding, bind_group) =
                Self::create_texture(device, texture, &self.pipeline.get_bind_group_layout(0));
            self.texture = new_texture;
            self.padding = padding;
            self.bind_group = bind_group;
        }
    }

    /// Handles copying from the [`CaptureState`] texture to the surface texture and the buffer.
    /// Pass the returned buffer to [`CaptureState::read_screen_rgba`] to read the data back to the cpu.
    pub fn copy_textures(
        &mut self,
        device: &wgpu::Device,
        output_frame: &wgpu::SurfaceTexture,
        encoder: &mut wgpu::CommandEncoder,
    ) -> wgpu::Buffer {
        debug_assert_eq!(
            self.texture.size(),
            output_frame.texture.size(),
            "Texture sizes must match, `CaptureState::update` was probably not called"
        );

        // It would be more efficient to reuse the Buffer, e.g. via some kind of ring buffer, but
        // for most screenshot use cases this should be fine. When taking many screenshots (e.g. for a video)
        // it might make sense to revisit this and implement a more efficient solution.
        #[allow(clippy::allow_attributes, clippy::arc_with_non_send_sync)] // For wasm
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("egui_screen_capture_buffer"),
            size: (self.padding.padded_bytes_per_row * self.texture.height()) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let padding = self.padding;
        let tex = &mut self.texture;

        let tex_extent = tex.size();

        encoder.copy_texture_to_buffer(
            tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padding.padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            tex_extent,
        );

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("texture_copy"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output_frame.texture.create_view(&Default::default()),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);

        buffer
    }

    /// Handles copying from the [`CaptureState`] texture to the surface texture and the cpu
    /// This function is non-blocking and will send the data to the given sender when it's ready.
    /// Pass in the buffer returned from [`CaptureState::copy_textures`].
    /// Make sure to call this after the encoder has been submitted.
    pub fn read_screen_rgba(
        &self,
        ctx: egui::Context,
        buffer: wgpu::Buffer,
        data: Vec<UserData>,
        tx: CaptureSender,
        viewport_id: ViewportId,
    ) {
        #[allow(clippy::allow_attributes, clippy::arc_with_non_send_sync)] // For wasm
        let buffer = Arc::new(buffer);
        let buffer_clone = Arc::clone(&buffer);
        let buffer_slice = buffer_clone.slice(..);
        let format = self.texture.format();
        let tex_extent = self.texture.size();
        let padding = self.padding;
        // Dispatch on format once outside the hot loop. Using constant indices for the swizzle
        // (instead of a runtime `[usize; 4]` table) lets LLVM auto-vectorize the per-pixel
        // path, and the Rgba8 case skips the swizzle entirely via a row-wise bytemuck copy.
        let is_bgra = match format {
            wgpu::TextureFormat::Rgba8Unorm => false,
            wgpu::TextureFormat::Bgra8Unorm => true,
            _ => {
                log::error!(
                    "Screen can't be captured unless the surface format is Rgba8Unorm or Bgra8Unorm. Current surface format is {format:?}"
                );
                return;
            }
        };
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            if let Err(err) = result {
                log::error!("Failed to map buffer for reading: {err}");
                return;
            }
            // The `map_async` callback is fired by wgpu on the render thread inside
            // `Queue::submit`. The pixel swizzle on a full-screen buffer is CPU-bound and
            // would block the next frame's GPU dispatch. Off-load to a worker thread so
            // submit returns immediately. On wasm we can't move `Arc<Buffer>` across
            // threads (and there are no real threads anyway), so we keep the work inline.
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = std::thread::Builder::new()
                    .name("egui_wgpu_capture_decode".into())
                    .spawn(move || {
                        decode_and_send(
                            buffer, padding, tex_extent, is_bgra, ctx, tx, data, viewport_id,
                        );
                    });
            }
            #[cfg(target_arch = "wasm32")]
            decode_and_send(
                buffer, padding, tex_extent, is_bgra, ctx, tx, data, viewport_id,
            );
        });
    }
}

/// Map the captured buffer, swizzle BGRA → RGBA if needed, ship the resulting
/// [`ColorImage`] through `tx`, and wake the UI thread. Called either inline (wasm) or on
/// a dedicated worker thread (native) — never on the render thread, so the per-pixel work
/// can't backpressure GPU submission.
fn decode_and_send(
    buffer: Arc<wgpu::Buffer>,
    padding: BufferPadding,
    tex_extent: wgpu::Extent3d,
    is_bgra: bool,
    ctx: egui::Context,
    tx: CaptureSender,
    data: Vec<UserData>,
    viewport_id: ViewportId,
) {
    let buffer_slice = buffer.slice(..);
    let total_pixels = (tex_extent.width * tex_extent.height) as usize;
    let mut pixels: Vec<epaint::Color32> = Vec::with_capacity(total_pixels);
    let padded_bpr = padding.padded_bytes_per_row as usize;
    let unpadded_bpr = padding.unpadded_bytes_per_row as usize;
    let mapped = buffer_slice.get_mapped_range();
    if is_bgra {
        // Byte-by-byte swizzle BGRA → RGBA. `chunks_exact` is faster than `chunks`
        // (no remainder/Option dance) and literal indices vectorize well.
        for padded_row in mapped.chunks_exact(padded_bpr) {
            let row = &padded_row[..unpadded_bpr];
            for px in row.chunks_exact(4) {
                pixels.push(epaint::Color32::from_rgba_premultiplied(
                    px[2], px[1], px[0], px[3],
                ));
            }
        }
    } else {
        // RGBA already → pure memcpy per row into the Vec<Color32> via bytemuck.
        // `Color32` is `#[repr(C)] [u8; 4]` with a `bytemuck::Pod` impl, so this is a
        // straight reinterpretation.
        for padded_row in mapped.chunks_exact(padded_bpr) {
            let row = &padded_row[..unpadded_bpr];
            pixels.extend_from_slice(bytemuck::cast_slice::<u8, epaint::Color32>(row));
        }
    }
    drop(mapped);
    buffer.unmap();

    tx.send((
        viewport_id,
        data,
        ColorImage::new(
            [tex_extent.width as usize, tex_extent.height as usize],
            pixels,
        ),
    ))
    .ok();
    ctx.request_repaint();
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
