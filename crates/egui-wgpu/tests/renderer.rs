//! Regression tests for the error-returning renderer paths.
//!
//! `Renderer::update_texture`, `Renderer::update_egui_texture_from_wgpu_texture*` and
//! `Renderer::update_buffers` must report malformed deltas and failed staging allocations as
//! `WgpuError`s instead of panicking (see the `egui-wgpu` panic guard: a hostile or stale
//! texture delta, or a memory-constrained staging write, must degrade a frame, not abort the
//! UI thread).
//!
//! The tests run on a real wgpu device; on machines without a usable adapter they skip
//! instead of failing.

#![cfg(not(target_arch = "wasm32"))] // tests never run on wasm, but `cargo check` does

use std::sync::Arc;

use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor, StagingBufferKind, WgpuError, wgpu};

/// Adapter/device helper; `None` => skip (no GPU / no backend on this machine).
fn device_and_queue() -> Option<(wgpu::Device, wgpu::Queue)> {
    device_and_queue_with_limits(wgpu::Limits::default())
}

/// Like [`device_and_queue`], but requests the given [`wgpu::Limits`].
fn device_and_queue_with_limits(limits: wgpu::Limits) -> Option<(wgpu::Device, wgpu::Queue)> {
    // `InstanceDescriptor` has no `Default` impl on wgpu 30 (it carries the non-`Clone`
    // display handle) — use its intended constructor.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?; // no adapter on this machine: skip, don't fail
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("egui-wgpu test device"),
        required_limits: limits,
        ..wgpu::DeviceDescriptor::default()
    };
    pollster::block_on(adapter.request_device(&descriptor)).ok()
}

fn renderer(device: &wgpu::Device) -> Renderer {
    Renderer::new(
        device,
        wgpu::TextureFormat::Rgba8Unorm,
        RendererOptions::default(),
    )
}

fn color_image(w: usize, h: usize) -> epaint::ColorImage {
    epaint::ColorImage::from_rgba_premultiplied([w, h], &vec![128u8; 4 * w * h])
}

fn clip_rect() -> epaint::Rect {
    epaint::Rect::from_min_size(
        epaint::emath::Pos2::new(0.0, 0.0),
        epaint::emath::Vec2::splat(100.0),
    )
}

fn user_texture_and_view(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("egui-wgpu test user texture"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

/// A1: a partial delta for a texture id that was never allocated is an error, not a panic.
#[test]
fn partial_delta_for_unallocated_texture_is_error() {
    let Some((device, queue)) = device_and_queue() else {
        return;
    };
    let mut renderer = renderer(&device);

    let delta = epaint::ImageDelta::partial(
        [0, 0],
        color_image(2, 2),
        epaint::textures::TextureOptions::LINEAR,
    );
    let err = renderer
        .update_texture(&device, &queue, epaint::TextureId::Managed(42), &delta)
        .expect_err("a partial delta for an unallocated id must error");
    assert!(
        matches!(err, WgpuError::TextureNotAllocated { partial: true, .. }),
        "unexpected error: {err:?}"
    );
}

/// A2: a partial delta aimed at a user/native texture handle is an error, and the map entry
/// survives so its bind group stays usable for rendering.
#[test]
fn partial_delta_on_user_texture_is_error() {
    let Some((device, queue)) = device_and_queue() else {
        return;
    };
    let mut renderer = renderer(&device);

    let (_texture, view) = user_texture_and_view(&device);
    let id = renderer.register_native_texture(&device, &view, wgpu::FilterMode::Nearest);

    let delta = epaint::ImageDelta::partial(
        [0, 0],
        color_image(2, 2),
        epaint::textures::TextureOptions::LINEAR,
    );
    let err = renderer
        .update_texture(&device, &queue, id, &delta)
        .expect_err("a partial delta on a user texture must error");
    assert!(
        matches!(err, WgpuError::UserTextureUpdate(_)),
        "unexpected error: {err:?}"
    );
    assert!(
        renderer.texture(&id).is_some(),
        "the user texture entry must survive the failed delta"
    );
}

/// A3/A7: a partial delta extending past the allocated texture is an error and issues no
/// upload; the texture entry is unchanged; an in-bounds delta still succeeds.
#[test]
fn out_of_bounds_partial_delta_is_error() {
    let Some((device, queue)) = device_and_queue() else {
        return;
    };
    let mut renderer = renderer(&device);

    let id = epaint::TextureId::Managed(1);
    let full =
        epaint::ImageDelta::full(color_image(4, 4), epaint::textures::TextureOptions::LINEAR);
    renderer
        .update_texture(&device, &queue, id, &full)
        .expect("a valid full delta must allocate");

    let out_of_bounds = epaint::ImageDelta::partial(
        [2, 2],
        color_image(4, 4),
        epaint::textures::TextureOptions::LINEAR,
    );
    let err = renderer
        .update_texture(&device, &queue, id, &out_of_bounds)
        .expect_err("an out-of-bounds partial delta must error");
    assert!(
        matches!(err, WgpuError::TextureDeltaOutOfBounds { pos: [2, 2], .. }),
        "unexpected error: {err:?}"
    );

    let stored = renderer
        .texture(&id)
        .expect("the texture entry must survive the failed delta");
    let gpu = stored
        .texture
        .as_ref()
        .expect("managed textures have a GPU texture");
    assert_eq!(
        (gpu.width(), gpu.height()),
        (4, 4),
        "the allocated texture must be unchanged after the failed delta"
    );

    let in_bounds = epaint::ImageDelta::partial(
        [2, 2],
        color_image(2, 2),
        epaint::textures::TextureOptions::LINEAR,
    );
    renderer
        .update_texture(&device, &queue, id, &in_bounds)
        .expect("an in-bounds partial delta must succeed");
}

/// A4: `update_egui_texture_from_wgpu_texture` for an id that was never registered is an
/// error; with a registered id the existing behaviour is unchanged.
#[test]
fn native_texture_update_for_unknown_id_is_error() {
    let Some((device, _queue)) = device_and_queue() else {
        return;
    };
    let mut renderer = renderer(&device);

    let (_texture, view) = user_texture_and_view(&device);

    let err = renderer
        .update_egui_texture_from_wgpu_texture(
            &device,
            &view,
            wgpu::FilterMode::Nearest,
            epaint::TextureId::User(999),
        )
        .expect_err("updating an unregistered id must error");
    assert!(
        matches!(err, WgpuError::TextureNotAllocated { partial: false, .. }),
        "unexpected error: {err:?}"
    );

    let id = renderer.register_native_texture(&device, &view, wgpu::FilterMode::Nearest);
    renderer
        .update_egui_texture_from_wgpu_texture(&device, &view, wgpu::FilterMode::Nearest, id)
        .expect("updating a registered id must succeed");
}

fn mesh_primitive(mesh: epaint::Mesh) -> epaint::ClippedPrimitive {
    epaint::ClippedPrimitive {
        clip_rect: clip_rect(),
        primitive: epaint::Primitive::Mesh(mesh),
    }
}

/// A5: when wgpu cannot provide staging memory, `update_buffers` returns
/// `WgpuError::StagingAllocationFailed` for both the index and the vertex path instead of
/// panicking; a mesh that fits returns `Ok` on a fresh renderer.
///
/// Deterministic via restricted device limits: 65 KiB is above the renderer's start capacities
/// (so `Renderer::new` stays valid) and below the test meshes' requirement. The expected
/// over-limit `create_buffer`/`write_buffer_with` validation errors must be swallowed by a
/// no-op uncaptured-error handler, since wgpu's default handler aborts the process.
#[test]
fn staging_failure_is_error_not_panic() {
    let Some((device, queue)) = device_and_queue_with_limits(wgpu::Limits {
        max_buffer_size: 65_536,
        ..Default::default()
    }) else {
        return;
    };
    device.on_uncaptured_error(Arc::new(|_| {})); // Swallow the expected validation errors.

    // Index path: 20 000 indices = 80 000 bytes > 65 KiB. Each case gets a fresh renderer:
    // a failed resize replaces the buffer with an over-limit (invalid) one and inflates the
    // capacity, so a later small mesh on the same renderer would also fail — only a fresh
    // renderer makes the `Ok` case meaningful.
    {
        let mut renderer = renderer(&device);
        let mut mesh = epaint::Mesh::with_texture(epaint::TextureId::Managed(0));
        mesh.indices = vec![0; 20_000];
        let primitives = vec![mesh_primitive(mesh)];
        let err = renderer
            .update_buffers(
                &device,
                &queue,
                &mut test_encoder(&device),
                &primitives,
                &screen(),
            )
            .expect_err("an over-limit index upload must error");
        assert!(
            matches!(
                err,
                WgpuError::StagingAllocationFailed {
                    kind: StagingBufferKind::Index,
                    required_bytes: 80_000
                }
            ),
            "unexpected error: {err:?}"
        );
    }

    // Vertex path: 0 indices, 4 000 vertices = 80 000 bytes > 65 KiB.
    {
        let mut renderer = renderer(&device);
        let mut mesh = epaint::Mesh::with_texture(epaint::TextureId::Managed(0));
        mesh.vertices = vec![
            epaint::Vertex {
                pos: epaint::emath::Pos2::new(0.0, 0.0),
                uv: epaint::WHITE_UV,
                color: epaint::Color32::WHITE,
            };
            4_000
        ];
        let primitives = vec![mesh_primitive(mesh)];
        let err = renderer
            .update_buffers(
                &device,
                &queue,
                &mut test_encoder(&device),
                &primitives,
                &screen(),
            )
            .expect_err("an over-limit vertex upload must error");
        assert!(
            matches!(
                err,
                WgpuError::StagingAllocationFailed {
                    kind: StagingBufferKind::Vertex,
                    required_bytes: 80_000
                }
            ),
            "unexpected error: {err:?}"
        );
    }

    // Success path: a small mesh fits a fresh renderer's start capacities.
    {
        let mut renderer = renderer(&device);
        let mut mesh = epaint::Mesh::with_texture(epaint::TextureId::Managed(0));
        mesh.vertices = vec![
            epaint::Vertex {
                pos: epaint::emath::Pos2::new(0.0, 0.0),
                uv: epaint::WHITE_UV,
                color: epaint::Color32::WHITE,
            };
            3
        ];
        mesh.indices = vec![0, 1, 2];
        let primitives = vec![mesh_primitive(mesh)];
        let cmd_bufs = renderer
            .update_buffers(
                &device,
                &queue,
                &mut test_encoder(&device),
                &primitives,
                &screen(),
            )
            .expect("a small mesh must fit the restricted device");
        assert!(
            cmd_bufs.is_empty(),
            "callback-free meshes produce no user command buffers"
        );
    }
}

fn screen() -> ScreenDescriptor {
    ScreenDescriptor {
        size_in_pixels: [4, 4],
        pixels_per_point: 1.0,
    }
}

fn test_encoder(device: &wgpu::Device) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("egui-wgpu test encoder"),
    })
}

/// A5/A7: on the default device a small mesh returns `Ok` with no user command buffers, and a
/// following `render` with those buffers upholds the documented update-then-render contract
/// (no "must call `update_buffers` first" panic).
#[test]
fn update_buffers_success_keeps_contract() {
    let Some((device, queue)) = device_and_queue() else {
        return;
    };
    let mut renderer = renderer(&device);

    let mut mesh = epaint::Mesh::with_texture(epaint::TextureId::Managed(0));
    mesh.vertices = vec![
        epaint::Vertex {
            pos: epaint::emath::Pos2::new(0.0, 0.0),
            uv: epaint::WHITE_UV,
            color: epaint::Color32::WHITE,
        };
        3
    ];
    mesh.indices = vec![0, 1, 2];
    let primitives = vec![mesh_primitive(mesh)];

    let mut encoder = test_encoder(&device);
    let cmd_bufs = renderer
        .update_buffers(&device, &queue, &mut encoder, &primitives, &screen())
        .expect("a small mesh must upload fine");
    assert!(
        cmd_bufs.is_empty(),
        "callback-free meshes produce no user command buffers"
    );

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("egui-wgpu test render target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("egui-wgpu test render pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    // Must not hit the "You must call .update_buffers() before .render()" precondition.
    renderer.render(&mut pass.forget_lifetime(), &primitives, &screen());
}
