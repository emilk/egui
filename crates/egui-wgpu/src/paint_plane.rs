//! Painting [`egui::PaintPlane`]s into surfaces over native views.

use std::sync::Arc;

use egui::{Id, IdMap};
use epaint::mutex::Mutex;

use crate::{RenderState, renderer::ScreenDescriptor};

/// A tessellated [`egui::PaintPlane`], ready to paint.
pub struct PaintPlanePrimitives {
    /// Same as [`egui::PaintPlane::id`].
    pub id: Id,

    /// The size of the plane's surface.
    pub size_in_pixels: [u32; 2],

    /// The plane's primitives, with the plane's top left corner at the origin.
    pub primitives: Vec<epaint::ClippedPrimitive>,
}

impl PaintPlanePrimitives {
    /// Tessellate a plane, moving it to the origin of its own surface.
    pub fn tessellate(ctx: &egui::Context, plane: egui::PaintPlane, pixels_per_point: f32) -> Self {
        let egui::PaintPlane {
            id,
            rect,
            mut shapes,
        } = plane;
        let to_origin = egui::emath::TSTransform::from_translation(-rect.min.to_vec2());
        for shape in &mut shapes {
            shape.transform(to_origin);
        }
        let size = (rect.size() * pixels_per_point).round();
        Self {
            id,
            size_in_pixels: [size.x as u32, size.y as u32],
            primitives: ctx.tessellate(shapes, pixels_per_point),
        }
    }
}

struct PlaneSurface {
    surface: wgpu::Surface<'static>,

    /// The size the surface is configured for, if it is.
    configured: Option<[u32; 2]>,

    /// Nothing was painted into it last time, so painting nothing again can be skipped.
    empty: bool,
}

/// The surfaces that [`egui::PaintPlane`]s are painted into, by plane id.
///
/// Whoever owns a native view creates a transparent native view over it, makes a
/// [`wgpu::Surface`] for it with [`RenderState::instance`], and registers it here with the
/// plane's id. The painter then paints the plane into it each frame and presents it
/// together with the frame.
#[derive(Clone, Default)]
pub struct PlaneSurfaces(Arc<Mutex<IdMap<PlaneSurface>>>);

impl PlaneSurfaces {
    /// Paint the plane with this id into `surface` from now on.
    pub fn insert(&self, id: Id, surface: wgpu::Surface<'static>) {
        self.0.lock().insert(
            id,
            PlaneSurface {
                surface,
                configured: None,
                empty: false,
            },
        );
    }

    /// Stop painting the plane with this id, and drop its surface.
    ///
    /// Call this before the native view behind the surface goes away.
    pub fn remove(&self, id: Id) {
        self.0.lock().remove(&id);
    }

    /// Paint each plane that has a surface, and return the frames to present.
    ///
    /// Call it after egui's own frame is submitted, since the planes reuse the renderer's
    /// buffers, and before freeing this frame's textures, since the planes still use them.
    pub fn paint(
        &self,
        render_state: &RenderState,
        pixels_per_point: f32,
        planes: &[PaintPlanePrimitives],
        config: &crate::SurfaceConfig,
    ) -> Vec<wgpu::SurfaceTexture> {
        profiling::function_scope!();

        let mut surfaces = self.0.lock();
        let mut frames = Vec::new();

        for plane in planes {
            let Some(target) = surfaces.get_mut(&plane.id) else {
                continue;
            };
            let [width, height] = plane.size_in_pixels;
            if width == 0 || height == 0 {
                continue;
            }
            if plane.primitives.is_empty() && target.empty && target.configured.is_some() {
                continue;
            }

            if target.configured != Some(plane.size_in_pixels) {
                let caps = target.surface.get_capabilities(&render_state.adapter);
                if !caps.formats.contains(&render_state.target_format) {
                    log::warn!(
                        "Paint plane surface doesn't support {:?}",
                        render_state.target_format
                    );
                    continue;
                }
                // Anything but opaque: the native view has to show through.
                let alpha_mode = [
                    wgpu::CompositeAlphaMode::PreMultiplied,
                    wgpu::CompositeAlphaMode::PostMultiplied,
                    wgpu::CompositeAlphaMode::Inherit,
                ]
                .into_iter()
                .find(|mode| caps.alpha_modes.contains(mode))
                .unwrap_or(caps.alpha_modes[0]);

                let Some(default_config) =
                    target
                        .surface
                        .get_default_config(&render_state.adapter, width, height)
                else {
                    log::warn!("Paint plane surface isn't supported by this adapter");
                    continue;
                };
                let mut surface_config = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: render_state.target_format,
                    present_mode: config.present_mode,
                    alpha_mode,
                    view_formats: vec![render_state.target_format],
                    ..default_config
                };
                if let Some(latency) = config.desired_maximum_frame_latency {
                    surface_config.desired_maximum_frame_latency = latency;
                }
                target
                    .surface
                    .configure(&render_state.device, &surface_config);
                target.configured = Some(plane.size_in_pixels);
            }

            let frame = match target.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame)
                | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                other => {
                    log::debug!("No paint plane frame: {other:?}");
                    target.configured = None;
                    continue;
                }
            };

            let screen_descriptor = ScreenDescriptor {
                size_in_pixels: plane.size_in_pixels,
                pixels_per_point,
            };
            let mut encoder =
                render_state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("egui_paint_plane"),
                    });

            // The renderer has one set of vertex and index buffers. Egui's own frame is
            // submitted by now, so they can be written again, as long as each plane is
            // submitted before the next one writes them.
            let mut renderer = render_state.renderer.write();
            let user_cmd_bufs = renderer.update_buffers(
                &render_state.device,
                &render_state.queue,
                &mut encoder,
                &plane.primitives,
                &screen_descriptor,
            );
            {
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut render_pass = encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("egui_paint_plane"),
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
                    })
                    .forget_lifetime();
                renderer.render(&mut render_pass, &plane.primitives, &screen_descriptor);
            }
            drop(renderer);

            render_state
                .queue
                .submit(core::iter::chain(user_cmd_bufs, [encoder.finish()]));
            target.empty = plane.primitives.is_empty();
            frames.push(frame);
        }

        frames
    }
}
