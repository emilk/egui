use std::sync::Arc;

use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use egui::{mutex::RwLock, Rgba};
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
}

impl WebPainterWgpu {
    #[allow(unused)] // only used if `wgpu` is the only active feature.
    pub fn render_state(&self) -> Option<RenderState> {
        self.render_state.clone()
    }

    #[allow(unused)] // only used if `wgpu` is the only active feature.
    pub async fn new(canvas_id: &str, options: &WebOptions) -> Result<Self, String> {
        tracing::debug!("Creating wgpu painter");

        let canvas = super::canvas_element_or_die(canvas_id);

        let instance = wgpu::Instance::new(options.wgpu_options.backends);
        let surface = instance.create_surface_from_canvas(&canvas);

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
            egui_wgpu::preferred_framebuffer_format(&surface.get_supported_formats(&adapter));

        let renderer = egui_wgpu::Renderer::new(&device, target_format, None, 1);
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
        };

        tracing::debug!("wgpu painter initialized.");

        Ok(Self {
            canvas,
            canvas_id: canvas_id.to_owned(),
            render_state: Some(render_state),
            surface,
            surface_configuration,
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
        clear_color: Rgba,
        clipped_primitives: &[egui::ClippedPrimitive],
        pixels_per_point: f32,
        textures_delta: &egui::TexturesDelta,
    ) -> Result<(), JsValue> {
        let render_state = if let Some(render_state) = &self.render_state {
            render_state
        } else {
            return Err(JsValue::from_str(
                "Can't paint, wgpu renderer was already disposed",
            ));
        };

        // Resize surface if needed
        let size_in_pixels = [self.canvas.width(), self.canvas.height()];
        if size_in_pixels[0] != self.surface_configuration.width
            || size_in_pixels[1] != self.surface_configuration.height
        {
            self.surface_configuration.width = size_in_pixels[0];
            self.surface_configuration.height = size_in_pixels[1];
            self.surface
                .configure(&render_state.device, &self.surface_configuration);
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
        {
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
            );
        }

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
                            r: clear_color.r() as f64,
                            g: clear_color.g() as f64,
                            b: clear_color.b() as f64,
                            a: clear_color.a() as f64,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
                label: Some("egui_render"),
            });

            renderer.render(&mut render_pass, clipped_primitives, &screen_descriptor);
        }

        {
            let mut renderer = render_state.renderer.write();
            for id in &textures_delta.free {
                renderer.free_texture(id);
            }
        }

        // Submit the commands.
        render_state.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    fn destroy(&mut self) {
        self.render_state = None;
    }
}
