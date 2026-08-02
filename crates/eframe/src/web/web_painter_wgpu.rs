use std::sync::Arc;

use egui::{Event, UserData, ViewportId};
use egui_wgpu::{
    BackdropTexture, RenderCursor, RenderProgress, RenderState, Renderer, SurfaceErrorAction,
    capture::{CaptureReceiver, CaptureSender, CaptureState, capture_channel},
};
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use super::web_painter::WebPainter;

pub(crate) struct WebPainterWgpu {
    canvas: HtmlCanvasElement,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    surface_configuration: wgpu::SurfaceConfiguration,
    render_state: Option<RenderState>,
    on_surface_status: Arc<dyn Fn(&wgpu::CurrentSurfaceTexture) -> SurfaceErrorAction>,
    depth_stencil_format: Option<wgpu::TextureFormat>,
    depth_texture_view: Option<wgpu::TextureView>,
    screen_capture_state: Option<CaptureState>,

    /// Somewhere to keep the half-drawn frame while a backdrop effect reads it.
    backdrop_texture: Option<BackdropTexture>,
    capture_tx: CaptureSender,
    capture_rx: CaptureReceiver,
    ctx: egui::Context,
    needs_reconfigure: bool,
    needs_recreate: bool,
}

/// Keep what an earlier pass wrote, or clear if this is the first pass.
///
/// Backdrop effects split the frame into several passes; every pass after the first has to
/// load what the ones before it wrote, colour and depth alike.
fn keep_or_clear<T>(color_load: wgpu::LoadOp<wgpu::Color>, clear_value: T) -> wgpu::LoadOp<T> {
    if matches!(color_load, wgpu::LoadOp::Load) {
        wgpu::LoadOp::Load
    } else {
        wgpu::LoadOp::Clear(clear_value)
    }
}

/// Owned web display handle that is `Send + Sync`.
///
/// `DisplayHandle` from `raw-window-handle` is `!Send`/`!Sync` because the enum
/// contains platform variants with raw pointers. On web the handle is always empty,
/// so this wrapper is safe.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug)]
struct WebDisplay;

#[cfg(target_arch = "wasm32")]
impl egui_wgpu::wgpu::rwh::HasDisplayHandle for WebDisplay {
    fn display_handle(
        &self,
    ) -> Result<egui_wgpu::wgpu::rwh::DisplayHandle<'_>, egui_wgpu::wgpu::rwh::HandleError> {
        Ok(egui_wgpu::wgpu::rwh::DisplayHandle::web())
    }
}

impl WebPainterWgpu {
    pub fn render_state(&self) -> Option<RenderState> {
        self.render_state.clone()
    }

    pub fn generate_depth_texture_view(
        &self,
        render_state: &RenderState,
        width_in_pixels: u32,
        height_in_pixels: u32,
    ) -> Option<wgpu::TextureView> {
        let device = &render_state.device;
        self.depth_stencil_format.map(|depth_stencil_format| {
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
                    format: depth_stencil_format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[depth_stencil_format],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        })
    }

    pub async fn new(
        ctx: egui::Context,
        canvas: web_sys::HtmlCanvasElement,
        options: &crate::WebOptions,
    ) -> Result<Self, String> {
        log::debug!("Creating wgpu painter");

        // Inject the display handle into the wgpu setup so that wgpu can create surfaces on WebGL.
        let mut wgpu_options = options.wgpu_options.clone();
        if let egui_wgpu::WgpuSetup::CreateNew(ref mut create_new) = wgpu_options.wgpu_setup
            && create_new.display_handle.is_none()
        {
            // Force WebGL, useful for quick & dirty testing:
            // create_new.instance_descriptor.backends = wgpu::Backends::GL;
            create_new.display_handle = Some(Box::new(WebDisplay));
        }

        let instance = wgpu_options.wgpu_setup.new_instance().await;
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|err| format!("failed to create wgpu surface: {err}"))?;

        let depth_stencil_format = egui_wgpu::depth_format_from_bits(options.depth_buffer, 0);

        let render_state = RenderState::create(
            &wgpu_options,
            &instance,
            Some(&surface),
            egui_wgpu::RendererOptions {
                dithering: options.dithering,
                depth_stencil_format,
                ..Default::default()
            },
        )
        .await
        .map_err(|err| err.to_string())?;

        let default_configuration = surface
            .get_default_config(&render_state.adapter, 0, 0) // Width/height is set later.
            .ok_or("The surface isn't supported by this adapter")?;

        let surface_configuration = wgpu::SurfaceConfiguration {
            format: render_state.target_format,
            present_mode: wgpu_options.surface.present_mode,
            view_formats: vec![render_state.target_format],
            ..default_configuration
        };

        log::debug!("wgpu painter initialized.");

        let (capture_tx, capture_rx) = capture_channel();

        Ok(Self {
            canvas,
            instance,
            render_state: Some(render_state),
            surface,
            surface_configuration,
            depth_stencil_format,
            depth_texture_view: None,
            on_surface_status: Arc::clone(&wgpu_options.on_surface_status) as _,
            screen_capture_state: None,
            backdrop_texture: None,
            capture_tx,
            capture_rx,
            ctx,
            needs_reconfigure: false,
            needs_recreate: false,
        })
    }
}

impl WebPainter for WebPainterWgpu {
    fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }

    fn max_texture_side(&self) -> usize {
        self.render_state.as_ref().map_or(0, |state| {
            state.device.limits().max_texture_dimension_2d as _
        })
    }

    fn paint_and_update_textures(
        &mut self,
        clear_color: [f32; 4],
        clipped_primitives: &[egui::ClippedPrimitive],
        pixels_per_point: f32,
        textures_delta: &mut egui::TexturesDelta,
        capture_data: Vec<UserData>,
    ) -> Result<(), JsValue> {
        let capture = !capture_data.is_empty();

        let size_in_pixels = [self.canvas.width(), self.canvas.height()];

        let Some(render_state) = &self.render_state else {
            return Err(JsValue::from_str(
                "Can't paint, wgpu renderer was already disposed",
            ));
        };

        // If the previous frame produced `CurrentSurfaceTexture::Lost`, drop and recreate the
        // surface from the canvas before re-borrowing `self.render_state` for the rest of paint.
        if self.needs_recreate {
            self.needs_recreate = false;
            match self
                .instance
                .create_surface(wgpu::SurfaceTarget::Canvas(self.canvas.clone()))
            {
                Ok(new_surface) => {
                    new_surface.configure(&render_state.device, &self.surface_configuration);
                    self.surface = new_surface;
                }
                Err(err) => {
                    log::error!("Failed to recreate wgpu surface for canvas: {err}");
                }
            }
        }

        let mut encoder =
            render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui_webpainter_paint_and_update_textures"),
                });

        // Upload all resources for the GPU.
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels,
            pixels_per_point,
        };

        let user_cmd_bufs = {
            let mut renderer = render_state.renderer.write();
            #[expect(clippy::iter_over_hash_type)] // Order doesn't matter here
            for (id, image_deltas) in textures_delta.set.drain() {
                for image_delta in image_deltas {
                    renderer.update_texture(
                        &render_state.device,
                        &render_state.queue,
                        id,
                        &image_delta,
                    );
                }
            }

            renderer.update_buffers(
                &render_state.device,
                &render_state.queue,
                &mut encoder,
                clipped_primitives,
                &screen_descriptor,
            )
        };

        // Resize surface if needed
        let is_zero_sized_surface = size_in_pixels[0] == 0 || size_in_pixels[1] == 0;
        let frame_and_capture_buffer = if is_zero_sized_surface {
            None
        } else {
            if size_in_pixels[0] != self.surface_configuration.width
                || size_in_pixels[1] != self.surface_configuration.height
            {
                self.surface_configuration.width = size_in_pixels[0];
                self.surface_configuration.height = size_in_pixels[1];
                self.surface
                    .configure(&render_state.device, &self.surface_configuration);
                self.depth_texture_view = self.generate_depth_texture_view(
                    render_state,
                    size_in_pixels[0],
                    size_in_pixels[1],
                );
            }

            if self.needs_reconfigure {
                self.surface
                    .configure(&render_state.device, &self.surface_configuration);
                self.needs_reconfigure = false;
            }

            let output_frame = match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => frame,
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    self.needs_reconfigure = true;
                    frame
                }
                other => {
                    match (*self.on_surface_status)(&other) {
                        SurfaceErrorAction::Reconfigure => {
                            self.surface
                                .configure(&render_state.device, &self.surface_configuration);
                        }
                        SurfaceErrorAction::RecreateSurface => {
                            // Full recovery needs `&mut self`, which conflicts with the live
                            // `render_state` / `self.surface` borrows here. Defer to the top
                            // of the next paint via the `needs_recreate` flag, and request a
                            // repaint so the next frame actually invokes `paint` to consume it.
                            self.needs_recreate = true;
                            self.ctx.request_repaint();
                        }
                        SurfaceErrorAction::SkipFrame => {}
                    }
                    return Ok(());
                }
            };

            // Backdrop effects have to read what egui has already drawn, and the surface
            // texture cannot be read, so render into our own texture and blit it onto the
            // surface afterwards. That is the same thing a screenshot needs, so reuse the
            // machinery.
            let needs_backdrop = Renderer::needs_backdrop(clipped_primitives);
            let render_to_own_texture = capture || needs_backdrop;

            {
                let renderer = render_state.renderer.read();

                if render_to_own_texture {
                    let capture_state = self.screen_capture_state.get_or_insert_with(|| {
                        CaptureState::new(&render_state.device, &output_frame.texture)
                    });
                    capture_state.update(&render_state.device, &output_frame.texture);
                }
                let target_texture = self
                    .screen_capture_state
                    .as_ref()
                    .filter(|_| render_to_own_texture)
                    .map_or(&output_frame.texture, |capture_state| {
                        &capture_state.texture
                    });
                let target_view =
                    target_texture.create_view(&wgpu::TextureViewDescriptor::default());

                // Size the backdrop texture up front, so that the borrow of it can be held
                // for the whole render loop below.
                if needs_backdrop {
                    let size = [target_texture.width(), target_texture.height()];
                    let format = target_texture.format();
                    match &mut self.backdrop_texture {
                        Some(backdrop) => backdrop.update(&render_state.device, size, format),
                        None => {
                            self.backdrop_texture =
                                Some(BackdropTexture::new(&render_state.device, size, format));
                        }
                    }
                }
                let backdrop_texture = self.backdrop_texture.as_ref().filter(|_| needs_backdrop);

                // Usually one pass is enough. A backdrop effect needs to read what egui has
                // drawn so far, and nothing can read the texture it is drawing into, so the
                // pass has to be ended and a new one started for each of those.
                let mut cursor = RenderCursor::default();
                let mut backdrop = None;
                let mut color_load = wgpu::LoadOp::Clear(wgpu::Color {
                    r: clear_color[0] as f64,
                    g: clear_color[1] as f64,
                    b: clear_color[2] as f64,
                    a: clear_color[3] as f64,
                });

                loop {
                    let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &target_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: color_load,
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        depth_stencil_attachment: self.depth_texture_view.as_ref().map(|view| {
                            wgpu::RenderPassDepthStencilAttachment {
                                view,
                                depth_ops: self
                                    .depth_stencil_format
                                    .is_some_and(|depth_stencil_format| {
                                        depth_stencil_format.has_depth_aspect()
                                    })
                                    .then_some(wgpu::Operations {
                                        load: keep_or_clear(color_load, 1.0),
                                        // It is very unlikely that the depth buffer is needed after egui finished rendering
                                        // so no need to store it. (this can improve performance on tiling GPUs like mobile chips or Apple Silicon)
                                        // Backdrop effects split the render pass, and the later passes load what the earlier ones wrote.
                                        store: if needs_backdrop {
                                            wgpu::StoreOp::Store
                                        } else {
                                            wgpu::StoreOp::Discard
                                        },
                                    }),
                                stencil_ops: self
                                    .depth_stencil_format
                                    .is_some_and(|depth_stencil_format| {
                                        depth_stencil_format.has_stencil_aspect()
                                    })
                                    .then_some(wgpu::Operations {
                                        load: keep_or_clear(color_load, 0),
                                        store: if needs_backdrop {
                                            wgpu::StoreOp::Store
                                        } else {
                                            wgpu::StoreOp::Discard
                                        },
                                    }),
                            }
                        }),
                        label: Some("egui_render"),
                        occlusion_query_set: None,
                        timestamp_writes: None,
                        multiview_mask: None,
                    });

                    // Forgetting the pass' lifetime means that we are no longer compile-time protected from
                    // runtime errors caused by accessing the parent encoder before the render pass is dropped.
                    // Since we don't pass it on to the renderer, we should be perfectly safe against this mistake here!
                    let mut render_pass = render_pass.forget_lifetime();
                    let progress = renderer.render_from(
                        &mut render_pass,
                        clipped_primitives,
                        &screen_descriptor,
                        &mut cursor,
                        backdrop.as_ref(),
                    );
                    // Ends the pass, so that the texture we drew into can be read.
                    drop(render_pass);

                    if progress == RenderProgress::Done {
                        break;
                    }

                    let Some(backdrop_texture) = backdrop_texture else {
                        // `needs_backdrop` said there were none, so `render_from` should
                        // never have stopped.
                        debug_assert!(false, "Bug in egui-wgpu: no backdrop texture was prepared");
                        break;
                    };
                    backdrop = renderer.capture_backdrop(
                        &render_state.device,
                        &render_state.queue,
                        &mut encoder,
                        clipped_primitives,
                        cursor,
                        target_texture,
                        backdrop_texture,
                    );
                    // Keep what we have already drawn.
                    color_load = wgpu::LoadOp::Load;
                }
            }

            let capture_buffer =
                if capture && let Some(capture_state) = &mut self.screen_capture_state {
                    Some(capture_state.copy_to_buffer(&render_state.device, &mut encoder))
                } else {
                    None
                };
            if render_to_own_texture && let Some(capture_state) = &self.screen_capture_state {
                capture_state.blit_to_surface(&output_frame, &mut encoder);
            }

            Some((output_frame, capture_buffer))
        };

        // Submit the commands: both the main buffer and user-defined ones.
        render_state
            .queue
            .submit(std::iter::chain(user_cmd_bufs, [encoder.finish()]));

        if let Some((frame, capture_buffer)) = frame_and_capture_buffer {
            if let Some(capture_buffer) = capture_buffer
                && let Some(capture_state) = &self.screen_capture_state
            {
                capture_state.read_screen_rgba(
                    self.ctx.clone(),
                    capture_buffer,
                    capture_data,
                    self.capture_tx.clone(),
                    ViewportId::ROOT,
                );
            }

            render_state.queue.present(frame);
        }

        // Free textures marked for destruction **after** queue submit since they might still be used in the current frame.
        // Calling `wgpu::Texture::destroy` on a texture that is still in use would invalidate the command buffer(s) it is used in.
        // However, once we called `wgpu::Queue::submit`, it is up for wgpu to determine how long the underlying gpu resource has to live.
        {
            let mut renderer = render_state.renderer.write();
            #[expect(clippy::iter_over_hash_type)] // Order doesn't matter here
            for id in textures_delta.free.drain() {
                renderer.free_texture(&id);
            }
        }

        Ok(())
    }

    fn handle_screenshots(&mut self, events: &mut Vec<Event>) {
        for (viewport_id, user_data, screenshot) in self.capture_rx.try_iter() {
            let screenshot = Arc::new(screenshot);
            for data in user_data {
                events.push(Event::Screenshot {
                    viewport_id,
                    user_data: data,
                    image: Arc::clone(&screenshot),
                });
            }
        }
    }

    fn destroy(&mut self) {
        self.render_state = None;
    }
}
