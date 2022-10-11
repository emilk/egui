use std::sync::Arc;

use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use egui::{mutex::RwLock, Rgba};
use egui_wgpu::{renderer::ScreenDescriptor, RenderState};

use crate::WebOptions;

use super::web_painter::WebPainter;

pub(crate) struct WebPainterWgpu {
    canvas: HtmlCanvasElement,
    canvas_id: String,
    surface: wgpu::Surface,
    surface_size: [u32; 2],
    limits: wgpu::Limits,
    render_state: Option<RenderState>,
}

impl WebPainterWgpu {
    #[allow(unused)] // only used if `wgpu` is the only active feature.
    pub fn render_state(&self) -> Option<RenderState> {
        self.render_state.clone()
    }

    #[allow(unused)] // only used if `wgpu` is the only active feature.
    pub async fn new(canvas_id: &str, _options: &WebOptions) -> Result<Self, String> {
        tracing::debug!("Creating wgpu painter with WebGL backend…");

        let canvas = super::canvas_element_or_die(canvas_id);
        let limits = wgpu::Limits::downlevel_webgl2_defaults(); // TODO(Wumpf): Expose to eframe user

        // TODO(Wumpf): Should be able to switch between WebGL & WebGPU (only)
        let backends = wgpu::Backends::GL; //wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
        let instance = wgpu::Instance::new(backends);
        let surface = instance.create_surface_from_canvas(&canvas);

        let adapter =
            wgpu::util::initialize_adapter_from_env_or_default(&instance, backends, Some(&surface))
                .await
                .ok_or_else(|| "No suitable GPU adapters found on the system".to_owned())?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("egui_webpainter"),
                    features: wgpu::Features::empty(),
                    limits: limits.clone(),
                },
                None, // No capture exposed so far - unclear how we can expose this in a browser environment (?)
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

        tracing::debug!("wgpu painter initialized.");

        Ok(Self {
            canvas,
            canvas_id: canvas_id.to_owned(),
            render_state: Some(render_state),
            surface,
            surface_size: [0, 0],
            limits,
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
        let canvas_size = [self.canvas.width(), self.canvas.height()];
        if canvas_size != self.surface_size {
            self.surface.configure(
                &render_state.device,
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: render_state.target_format,
                    width: canvas_size[0],
                    height: canvas_size[1],
                    present_mode: wgpu::PresentMode::Fifo,
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                },
            );
            self.surface_size = canvas_size.clone();
        }

        let frame = self.surface.get_current_texture().map_err(|err| {
            JsValue::from_str(&format!(
                "Failed to acquire next swap chain texture: {}",
                err
            ))
        })?;

        let mut encoder =
            render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui_webpainter_paint_and_update_textures"),
                });

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: canvas_size,
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
