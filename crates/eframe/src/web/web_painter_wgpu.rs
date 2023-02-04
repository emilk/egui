use std::sync::Arc;

use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use egui::mutex::RwLock;
use egui_wgpu::{renderer::ScreenDescriptor, RenderState, SurfaceErrorAction};

use crate::WebOptions;

use super::web_painter::WebPainter;

pub(crate) struct WebPainterWgpu {
    canvas: HtmlCanvasElement,
    canvas_id: String,
    surface: wgpu::Surface,
    surface_configuration: wgpu::SurfaceConfiguration,
    limits: wgpu::Limits,
    render_state: Option<RenderState>,
    on_surface_error: Arc<dyn Fn(wgpu::SurfaceError) -> SurfaceErrorAction>,
    depth_format: Option<wgpu::TextureFormat>,
    depth_texture_view: Option<wgpu::TextureView>,
}

impl WebPainterWgpu {
    #[allow(unused)] // only used if `wgpu` is the only active feature.
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
        self.depth_format.map(|depth_format| {
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
                    format: depth_format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[depth_format],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        })
    }

    #[allow(unused)] // only used if `wgpu` is the only active feature.
    pub async fn new(canvas_id: &str, options: &WebOptions) -> Result<Self, String> {
        tracing::debug!("Creating wgpu painter");

        let canvas = super::canvas_element_or_die(canvas_id);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: options.wgpu_options.backends,
            dx12_shader_compiler: Default::default(),
        });
        let surface = instance
            .create_surface_from_canvas(&canvas)
            .map_err(|err| format!("failed to create wgpu surface: {err}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: options.wgpu_options.power_preference,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .ok_or_else(|| "No suitable GPU adapters found on the system".to_owned())?;

        let (device, queue) = adapter
            .request_device(
                &options.wgpu_options.device_descriptor,
                None, // Capture doesn't work in the browser environment.
            )
            .await
            .map_err(|err| format!("Failed to find wgpu device: {}", err))?;

        let target_format =
            egui_wgpu::preferred_framebuffer_format(&surface.get_capabilities(&adapter).formats);

        let depth_format = options.wgpu_options.depth_format;
        let renderer = egui_wgpu::Renderer::new(&device, target_format, depth_format, 1);
        let render_state = RenderState {
            device: Arc::new(device),
            queue: Arc::new(queue),
            target_format,
            renderer: Arc::new(RwLock::new(renderer)),
        };

        let surface_configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: target_format,
            width: 0,
            height: 0,
            present_mode: options.wgpu_options.present_mode,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![target_format],
        };

        tracing::debug!("wgpu painter initialized.");

        Ok(Self {
            canvas,
            canvas_id: canvas_id.to_owned(),
            render_state: Some(render_state),
            surface,
            surface_configuration,
            depth_format,
            depth_texture_view: None,
            limits: options.wgpu_options.device_descriptor.limits.clone(),
            on_surface_error: options.wgpu_options.on_surface_error.clone(),
        })
    }
}

impl WebPainter for WebPainterWgpu {
    fn canvas_id(&self) -> &str {
        &self.canvas_id
    }

    fn max_texture_side(&self) -> usize {
        self.limits.max_texture_dimension_2d as _
    }

    fn paint_and_update_textures(
        &mut self,
        clear_color: [f32; 4],
        clipped_primitives: &[egui::ClippedPrimitive],
        pixels_per_point: f32,
        textures_delta: &egui::TexturesDelta,
    ) -> Result<(), JsValue> {
        let size_in_pixels = [self.canvas.width(), self.canvas.height()];

        let render_state = if let Some(render_state) = &self.render_state {
            render_state
        } else {
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
        let screen_descriptor = ScreenDescriptor {
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
        let frame = if is_zero_sized_surface {
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

            let frame = match self.surface.get_current_texture() {
                Ok(frame) => frame,
                #[allow(clippy::single_match_else)]
                Err(e) => match (*self.on_surface_error)(e) {
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
                let frame_view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: clear_color[0] as f64,
                                g: clear_color[1] as f64,
                                b: clear_color[2] as f64,
                                a: clear_color[3] as f64,
                            }),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: self.depth_texture_view.as_ref().map(|view| {
                        wgpu::RenderPassDepthStencilAttachment {
                            view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: false,
                            }),
                            stencil_ops: None,
                        }
                    }),
                    label: Some("egui_render"),
                });

                renderer.render(&mut render_pass, clipped_primitives, &screen_descriptor);
            }

            Some(frame)
        };

        {
            let mut renderer = render_state.renderer.write();
            for id in &textures_delta.free {
                renderer.free_texture(id);
            }
        }

        // Submit the commands: both the main buffer and user-defined ones.
        render_state.queue.submit(
            user_cmd_bufs
                .into_iter()
                .chain(std::iter::once(encoder.finish())),
        );

        if let Some(frame) = frame {
            frame.present();
        }

        Ok(())
    }

    fn destroy(&mut self) {
        self.render_state = None;
    }
}
