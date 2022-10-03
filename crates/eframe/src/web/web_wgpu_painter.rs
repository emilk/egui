use egui_wgpu::renderer::ScreenDescriptor;
use egui_wgpu::Renderer;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use egui::{ClippedPrimitive, Rgba};
use wgpu::Backends;

use crate::WebOptions;

pub(crate) struct WebPainter {
    canvas: HtmlCanvasElement,
    canvas_id: String,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    surface_size: [u32; 2],
    renderer: Renderer,
    limits: wgpu::Limits,
}

impl WebPainter {
    pub fn new(canvas_id: &str, _options: &WebOptions) -> Result<Self, String> {
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
        let renderer = egui_wgpu::Renderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb, 1, 0);

        Ok(Self {
            canvas,
            canvas_id: canvas_id.to_owned(),
            device,
            queue,
            renderer,
            surface,
            surface_size: [0, 0],
            limits,
        })
    }

    // TODO: do we need all of these??

    pub fn canvas_id(&self) -> &str {
        &self.canvas_id
    }

    pub fn max_texture_side(&self) -> usize {
        self.limits.max_texture_dimension_2d as _
    }

    fn configure_surface(&self) {
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                width: self.canvas.width(),
                height: self.canvas.height(),
                present_mode: wgpu::PresentMode::Fifo,
            },
        );
    }

    pub fn paint_and_update_textures(
        &mut self,
        clear_color: Rgba,
        clipped_primitives: &[egui::ClippedPrimitive],
        pixels_per_point: f32,
        textures_delta: &egui::TexturesDelta,
    ) -> Result<(), JsValue> {
        // Resize surface if needed
        let canvas_size = [self.canvas.width(), self.canvas.height()];
        if canvas_size != self.surface_size {
            self.configure_surface();
            self.surface_size = canvas_size;
        }

        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("eframe encoder"),
            });

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: canvas_size,
            pixels_per_point,
        };

        for (id, image_delta) in &textures_delta.set {
            self.renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        self.renderer.update_buffers(
            &self.device,
            &self.queue,
            clipped_primitives,
            &screen_descriptor,
        );

        // Record all render passes.
        self.renderer.render(
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

        for id in &textures_delta.free {
            self.renderer.free_texture(id);
        }

        // Submit the commands.
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }

    pub fn destroy(&mut self) {
        // TODO: destroy things? doesn't fit well with wgpu
    }
}
