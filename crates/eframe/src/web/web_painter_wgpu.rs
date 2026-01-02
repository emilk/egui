use std::sync::Arc;

use egui::{Event, UserData, ViewportId};
use egui_wgpu::{
    RenderState, SurfaceErrorAction,
    capture::{CaptureReceiver, CaptureSender, CaptureState, capture_channel},
};
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use super::web_painter::WebPainter;

pub(crate) struct WebPainterWgpu {
    canvas: HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    surface_configuration: wgpu::SurfaceConfiguration,
    render_state: Option<RenderState>,
    on_surface_error: Arc<dyn Fn(wgpu::SurfaceError) -> SurfaceErrorAction>,
    depth_stencil_format: Option<wgpu::TextureFormat>,
    depth_texture_view: Option<wgpu::TextureView>,
    screen_capture_state: Option<CaptureState>,
    capture_tx: CaptureSender,
    capture_rx: CaptureReceiver,
    ctx: egui::Context,
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

        let instance = options.wgpu_options.wgpu_setup.new_instance().await;
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|err| format!("failed to create wgpu surface: {err}"))?;

        let depth_stencil_format = egui_wgpu::depth_format_from_bits(options.depth_buffer, 0);

        let render_state = RenderState::create(
            &options.wgpu_options,
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
            present_mode: options.wgpu_options.present_mode,
            view_formats: vec![render_state.target_format],
            ..default_configuration
        };

        log::debug!("wgpu painter initialized.");

        let (capture_tx, capture_rx) = capture_channel();

        Ok(Self {
            canvas,
            render_state: Some(render_state),
            surface,
            surface_configuration,
            depth_stencil_format,
            depth_texture_view: None,
            on_surface_error: Arc::clone(&options.wgpu_options.on_surface_error) as _,
            screen_capture_state: None,
            capture_tx,
            capture_rx,
            ctx,
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
        textures_delta: &egui::TexturesDelta,
        capture_data: Vec<UserData>,
    ) -> Result<(), JsValue> {
        let capture = !capture_data.is_empty();

        let size_in_pixels = [self.canvas.width(), self.canvas.height()];

        let Some(render_state) = &self.render_state else {
            return Err(JsValue::from_str(
                "Can't paint, wgpu renderer was already disposed",
            ));
        };

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
            for (id, image_delta) in &textures_delta.set {
                renderer.update_texture(
                    &render_state.device,
                    &render_state.queue,
                    *id,
                    image_delta,
                );
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

            let output_frame = match self.surface.get_current_texture() {
                Ok(frame) => frame,
                Err(err) => match (*self.on_surface_error)(err) {
                    SurfaceErrorAction::RecreateSurface => {
                        self.surface
                            .configure(&render_state.device, &self.surface_configuration);
                        return Ok(());
                    }
                    SurfaceErrorAction::SkipFrame => {
                        return Ok(());
                    }
                },
            };

            {
                let renderer = render_state.renderer.read();

                let target_texture = if capture {
                    let capture_state = self.screen_capture_state.get_or_insert_with(|| {
                        CaptureState::new(&render_state.device, &output_frame.texture)
                    });
                    capture_state.update(&render_state.device, &output_frame.texture);

                    &capture_state.texture
                } else {
                    &output_frame.texture
                };
                let target_view =
                    target_texture.create_view(&wgpu::TextureViewDescriptor::default());

                let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: clear_color[0] as f64,
                                g: clear_color[1] as f64,
                                b: clear_color[2] as f64,
                                a: clear_color[3] as f64,
                            }),
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
                                    load: wgpu::LoadOp::Clear(1.0),
                                    // It is very unlikely that the depth buffer is needed after egui finished rendering
                                    // so no need to store it. (this can improve performance on tiling GPUs like mobile chips or Apple Silicon)
                                    store: wgpu::StoreOp::Discard,
                                }),
                            stencil_ops: self
                                .depth_stencil_format
                                .is_some_and(|depth_stencil_format| {
                                    depth_stencil_format.has_stencil_aspect()
                                })
                                .then_some(wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(0),
                                    store: wgpu::StoreOp::Discard,
                                }),
                        }
                    }),
                    label: Some("egui_render"),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                // Forgetting the pass' lifetime means that we are no longer compile-time protected from
                // runtime errors caused by accessing the parent encoder before the render pass is dropped.
                // Since we don't pass it on to the renderer, we should be perfectly safe against this mistake here!
                renderer.render(
                    &mut render_pass.forget_lifetime(),
                    clipped_primitives,
                    &screen_descriptor,
                );
            }

            let mut capture_buffer = None;

            if capture && let Some(capture_state) = &mut self.screen_capture_state {
                capture_buffer = Some(capture_state.copy_textures(
                    &render_state.device,
                    &output_frame,
                    &mut encoder,
                ));
            }

            Some((output_frame, capture_buffer))
        };

        // Submit the commands: both the main buffer and user-defined ones.
        render_state
            .queue
            .submit(user_cmd_bufs.into_iter().chain([encoder.finish()]));

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

            frame.present();
        }

        // Free textures marked for destruction **after** queue submit since they might still be used in the current frame.
        // Calling `wgpu::Texture::destroy` on a texture that is still in use would invalidate the command buffer(s) it is used in.
        // However, once we called `wgpu::Queue::submit`, it is up for wgpu to determine how long the underlying gpu resource has to live.
        {
            let mut renderer = render_state.renderer.write();
            for id in &textures_delta.free {
                renderer.free_texture(id);
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
