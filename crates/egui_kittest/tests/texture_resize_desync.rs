//! Reproduces the "font atlas grew but the GPU texture didn't" desync at the
//! `egui-wgpu` `Renderer` level, without needing a window/surface.
//!
//! Models the real font-atlas case: a texel lives at a fixed *row*, and the UV
//! that samples it is normalized by the *intended* (grown) texture height. If
//! the GPU texture's height lags behind (because a full texture delta was
//! dropped), that same UV silently samples the wrong row — exactly the
//! "atlas looks twice as high, old glyph UVs not invalidated" symptom.
//!
//! It also confirms that wgpu does *not* error on this (the wrong-row read is
//! silent), and that simply applying the grow delta restores correctness.

#![cfg(all(feature = "wgpu", not(target_arch = "wasm32")))]

use egui::{
    Color32, Pos2, Rect,
    epaint::{ClippedPrimitive, Mesh, Primitive, TextureId, Vertex, image::ColorImage},
    pos2,
};
use egui_kittest::wgpu::{create_render_state, default_wgpu_setup};
use egui_wgpu::{ScreenDescriptor, wgpu};

const TARGET: u32 = 8;

/// A texture whose row 1 is green; the rest are other colors.
/// `height` lets us simulate the texture *before* and *after* growth.
fn image_with_height(height: usize) -> ColorImage {
    let rows = [
        Color32::RED,
        Color32::GREEN, // <- the "glyph" we keep sampling, always at row 1
        Color32::BLUE,
        Color32::WHITE,
    ];
    let mut pixels = Vec::with_capacity(height);
    for y in 0..height {
        pixels.push(rows[y.min(rows.len() - 1)]);
    }
    ColorImage {
        size: [1, height],
        pixels,
        source_size: egui::Vec2::new(1.0, height as f32),
    }
}

/// A full-screen quad that samples a constant `v`, i.e. one row of the texture.
fn quad_sampling_v(tex_id: TextureId, v: f32) -> ClippedPrimitive {
    let uv = pos2(0.5, v);
    let corners = [
        pos2(0.0, 0.0),
        pos2(TARGET as f32, 0.0),
        pos2(0.0, TARGET as f32),
        pos2(TARGET as f32, TARGET as f32),
    ];
    let vertices = corners
        .into_iter()
        .map(|pos| Vertex {
            pos,
            uv,
            color: Color32::WHITE,
        })
        .collect();
    let mesh = Mesh {
        indices: vec![0, 1, 2, 2, 1, 3],
        vertices,
        texture_id: tex_id,
    };
    ClippedPrimitive {
        clip_rect: Rect::from_min_max(Pos2::ZERO, pos2(TARGET as f32, TARGET as f32)),
        primitive: Primitive::Mesh(mesh),
    }
}

struct Fixture {
    rs: egui_wgpu::RenderState,
    target: wgpu::Texture,
}

impl Fixture {
    fn new() -> Self {
        // Default options => hardware sampling (no manual/predictable filtering),
        // and our texture uses Nearest, so each row is sampled crisply.
        let rs = create_render_state(default_wgpu_setup(), egui_wgpu::RendererOptions::default());
        let target = rs.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("desync_target"),
            size: wgpu::Extent3d {
                width: TARGET,
                height: TARGET,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: rs.target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Self { rs, target }
    }

    fn upload(&self, tex_id: TextureId, image: ColorImage) {
        let delta = egui::epaint::ImageDelta::full(
            image,
            egui::TextureOptions::NEAREST, // crisp rows, no filtering blend
        );
        self.rs
            .renderer
            .write()
            .update_texture(&self.rs.device, &self.rs.queue, tex_id, &delta);
    }

    /// Render the quad and read back the center pixel.
    fn render_center_pixel(&self, prim: &ClippedPrimitive) -> [u8; 4] {
        let screen = ScreenDescriptor {
            size_in_pixels: [TARGET, TARGET],
            pixels_per_point: 1.0,
        };
        let jobs = std::slice::from_ref(prim);

        let mut encoder = self
            .rs
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let user_bufs = {
            let mut r = self.rs.renderer.write();
            r.update_buffers(&self.rs.device, &self.rs.queue, &mut encoder, jobs, &screen)
        };

        let view = self
            .target
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            self.rs.renderer.read().render(&mut pass, jobs, &screen);
        }

        // Read back into a 256-aligned buffer.
        let bytes_per_row = 256;
        let buffer = self.rs.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (bytes_per_row * TARGET) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(TARGET),
                },
            },
            wgpu::Extent3d {
                width: TARGET,
                height: TARGET,
                depth_or_array_layers: 1,
            },
        );

        self.rs.queue.submit(
            user_bufs
                .into_iter()
                .chain(std::iter::once(encoder.finish())),
        );

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map failed"));
        self.rs
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .expect("poll failed");
        let data = slice.get_mapped_range();

        // Center pixel (row TARGET/2, col TARGET/2).
        let y = TARGET / 2;
        let x = TARGET / 2;
        let off = (y * bytes_per_row + x * 4) as usize;
        let px = [data[off], data[off + 1], data[off + 2], data[off + 3]];
        drop(data);
        buffer.unmap();

        // Normalize BGRA -> RGBA if needed so the channel checks below are stable.
        if self.rs.target_format.remove_srgb_suffix() == wgpu::TextureFormat::Bgra8Unorm {
            [px[2], px[1], px[0], px[3]]
        } else {
            px
        }
    }
}

/// `v` that samples the green row (row 1) assuming the texture is 4 tall:
/// center of row 1 = 1.5 / 4 = 0.375.
const V_FOR_HEIGHT_4: f32 = 1.5 / 4.0;

#[test]
fn dropped_grow_delta_silently_samples_wrong_row() {
    let fx = Fixture::new();
    let tex = TextureId::Managed(1);

    // Atlas is currently 2 tall (rows: red, green). The UV, however, is computed
    // for the *intended* grown height of 4. On the small texture, v=0.375 maps to
    // row floor(0.375 * 2) = 0 => RED. This is the bug: the glyph row is missed.
    fx.upload(tex, image_with_height(2));
    let quad = quad_sampling_v(tex, V_FOR_HEIGHT_4);
    let px = fx.render_center_pixel(&quad);

    // wgpu does *not* error — it silently samples the wrong row.
    assert!(
        px[0] > 200 && px[1] < 80,
        "expected the wrong (red) row to be sampled when the grow delta is dropped, got {px:?}"
    );
}

#[test]
fn applying_grow_delta_fixes_sampling() {
    let fx = Fixture::new();
    let tex = TextureId::Managed(1);

    // Same starting point...
    fx.upload(tex, image_with_height(2));
    let quad = quad_sampling_v(tex, V_FOR_HEIGHT_4);
    let before = fx.render_center_pixel(&quad);
    assert!(
        before[0] > 200 && before[1] < 80,
        "precondition: {before:?}"
    );

    // ...now apply the grow delta (texture becomes 4 tall). This is exactly what
    // `Painter::paint_and_update_textures` must do even when the surface is gone.
    fx.upload(tex, image_with_height(4));
    let after = fx.render_center_pixel(&quad);

    // Now v=0.375 maps to row floor(0.375 * 4) = 1 => GREEN. Fixed.
    assert!(
        after[1] > 200 && after[0] < 80,
        "expected the correct (green) row after applying the grow delta, got {after:?}"
    );
}
