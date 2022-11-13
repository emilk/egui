use crate::shader_version::ShaderVersion;
pub use egui_winit;
use egui_winit::winit;
pub use egui_winit::EventResponse;

/// Use [`egui`] from a [`glow`] app based on [`winit`].
pub struct EguiGlow {
    pub egui_ctx: egui::Context,
    pub egui_winit: egui_winit::State,
    pub painter: crate::Painter,

    shapes: Vec<egui::epaint::ClippedShape>,
    textures_delta: egui::TexturesDelta,
}

impl EguiGlow {
    /// For automatic shader version detection set `shader_version` to `None`.
    pub fn new<E>(
        event_loop: &winit::event_loop::EventLoopWindowTarget<E>,
        gl: std::sync::Arc<glow::Context>,
        shader_version: Option<ShaderVersion>,
    ) -> Self {
        let painter = crate::Painter::new(gl, "", shader_version)
            .map_err(|error| {
                tracing::error!("error occurred in initializing painter:\n{}", error);
            })
            .unwrap();

        Self {
            egui_ctx: Default::default(),
            egui_winit: egui_winit::State::new(event_loop),
            painter,
            shapes: Default::default(),
            textures_delta: Default::default(),
        }
    }

    pub fn on_event(&mut self, event: &winit::event::WindowEvent<'_>) -> EventResponse {
        self.egui_winit.on_event(&self.egui_ctx, event)
    }

    /// Returns the `Duration` of the timeout after which egui should be repainted even if there's no new events.
    ///
    /// Call [`Self::paint`] later to paint.
    pub fn run(
        &mut self,
        window: &winit::window::Window,
        run_ui: impl FnMut(&egui::Context),
    ) -> std::time::Duration {
        let raw_input = self.egui_winit.take_egui_input(window);
        let egui::FullOutput {
            platform_output,
            repaint_after,
            textures_delta,
            shapes,
        } = self.egui_ctx.run(raw_input, run_ui);

        self.egui_winit
            .handle_platform_output(window, &self.egui_ctx, platform_output);

        self.shapes = shapes;
        self.textures_delta.append(textures_delta);
        repaint_after
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self, window: &winit::window::Window) {
        let shapes = std::mem::take(&mut self.shapes);
        let mut textures_delta = std::mem::take(&mut self.textures_delta);

        for (id, image_delta) in textures_delta.set {
            self.painter.set_texture(id, &image_delta);
        }

        let clipped_primitives = self.egui_ctx.tessellate(shapes);
        let dimensions: [u32; 2] = window.inner_size().into();
        self.painter.paint_primitives(
            dimensions,
            self.egui_ctx.pixels_per_point(),
            &clipped_primitives,
        );

        for id in textures_delta.free.drain(..) {
            self.painter.free_texture(id);
        }
    }

    /// Call to release the allocated graphics resources.
    pub fn destroy(&mut self) {
        self.painter.destroy();
    }
}
