use std::sync::Arc;

use egui::mutex::RwLock;
use egui_wgpu::renderer::ScreenDescriptor;
use egui_wgpu::RenderState;

use wasm_bindgen::JsValue;

use egui::Rgba;
use web_sys::HtmlCanvasElement;

use super::web_painter::WebPainter;
use crate::WebOptions;

pub(crate) struct WebPainterWgpu {
    canvas: HtmlCanvasElement,
    canvas_id: String,
    surface: wgpu::Surface,
    surface_size: [u32; 2],
    limits: wgpu::Limits,
    render_state: RenderState,
}

impl WebPainterWgpu {
    fn configure_surface(&mut self, new_size: &[u32; 2]) {
        self.surface.configure(
            &self.render_state.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.render_state.target_format,
                width: new_size[0],
                height: new_size[1],
                present_mode: wgpu::PresentMode::Fifo,
            },
        );
        self.surface_size = new_size.clone();
    }

    pub fn render_state(&self) -> RenderState {
        self.render_state.clone()
    }
}

impl WebPainter for WebPainterWgpu {
    fn new(canvas_id: &str, _options: &WebOptions) -> Result<Self, String> {
        let canvas = super::canvas_element_or_die(canvas_id);
        let limits = wgpu::Limits::downlevel_webgl2_defaults(); // TODO: Expose to eframe user

        // TODO: Should be able to switch between webgl & webgpu (only)
        let backends = wgpu::Backends::GL; //wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
        let instance = wgpu::Instance::new(backends);
        let surface = instance.create_surface_from_canvas(&canvas);

        let adapter = pollster::block_on(wgpu::util::initialize_adapter_from_env_or_default(
            &instance,
            backends,
            Some(&surface),
        ))
        .expect("No suitable GPU adapters found on the system!");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("eframe device"),
                features: wgpu::Features::empty(),
                limits: limits.clone(),
            },
            None, // TODO: Expose to eframe user
        ))
        .unwrap();

        // TODO: MSAA & depth
        // TODO: renderer unhappy about srgb. why? Can't use anything else
        let target_format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let renderer = egui_wgpu::Renderer::new(&device, target_format, 1, 0);
        let render_state = RenderState {
            device: Arc::new(device),
            queue: Arc::new(queue),
            target_format,
            renderer: Arc::new(RwLock::new(renderer)),
        };

        Ok(Self {
            canvas,
            canvas_id: canvas_id.to_owned(),
            render_state,
            surface,
            surface_size: [0, 0],
            limits,
        })
    }

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
        // Resize surface if needed
        let canvas_size = [self.canvas.width(), self.canvas.height()];
        if canvas_size != self.surface_size {
            self.configure_surface(&canvas_size);
        }

        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.render_state
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("eframe encoder"),
                });

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: canvas_size,
            pixels_per_point,
        };

        {
            let mut renderer = self.render_state.renderer.write();
            for (id, image_delta) in &textures_delta.set {
                renderer.update_texture(
                    &self.render_state.device,
                    &self.render_state.queue,
                    *id,
                    image_delta,
                );
            }

            renderer.update_buffers(
                &self.render_state.device,
                &self.render_state.queue,
                clipped_primitives,
                &screen_descriptor,
            );
        }

        // Record all render passes.
        self.render_state.renderer.read().render(
            &mut encoder,
            &view,
            clipped_primitives,
            &screen_descriptor,
            Some(wgpu::Color {
                r: clear_color.r() as f64,
                g: clear_color.g() as f64,
                b: clear_color.b() as f64,
                a: clear_color.a() as f64,
            }),
        );

        {
            let mut renderer = self.render_state.renderer.write();
            for id in &textures_delta.free {
                renderer.free_texture(id);
            }
        }

        // Submit the commands.
        self.render_state
            .queue
            .submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    fn destroy(&mut self) {
        // TODO: destroy things? doesn't fit well with wgpu
    }
}
