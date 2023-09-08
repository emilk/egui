use sdl2::{clipboard::ClipboardUtil, event::Event};

use crate::shader_version::ShaderVersion;

use self::platform::Platform;

mod conversions;
mod platform;

pub enum DpiScaling {
    /// Default is handled by sdl2
    Default,
    /// Custom DPI scaling
    Custom(f32),
}

/// Use [`egui`] from a [`glow`] app based on [`sdl2`].
pub struct EguiGlow {
    pub egui_ctx: egui::Context,
    pub egui_sdl2: Platform,
    pub painter: crate::Painter,

    shapes: Vec<egui::epaint::ClippedShape>,
    textures_delta: egui::TexturesDelta,
}

impl EguiGlow {
    /// For automatic shader version detection set `shader_version` to `None`.
    pub fn new(
        window: &sdl2::video::Window,
        gl: std::sync::Arc<glow::Context>,
        shader_version: Option<ShaderVersion>,
    ) -> Self {
        let painter = crate::Painter::new(gl, "", shader_version)
            .map_err(|err| {
                log::error!("error occurred in initializing painter:\n{err}");
            })
            .unwrap();

        Self {
            egui_ctx: Default::default(),
            egui_sdl2: Platform::new(window, DpiScaling::Default),
            painter,
            shapes: Default::default(),
            textures_delta: Default::default(),
        }
    }

    pub fn on_event(&mut self, event: &Event, window: &sdl2::video::Window) {
        self.egui_sdl2.handle_event(event, window);
    }

    /// Returns the `Duration` of the timeout after which egui should be repainted even if there's no new events.
    ///
    /// Call [`Self::paint`] later to paint.
    pub fn run(
        &mut self,
        window: &sdl2::video::Window,
        clipboard: &mut ClipboardUtil,
        run_ui: impl FnMut(&egui::Context),
    ) -> std::time::Duration {
        let raw_input = self.egui_sdl2.take_egui_input(window);

        let egui::FullOutput {
            platform_output,
            repaint_after,
            textures_delta,
            shapes,
        } = self.egui_ctx.run(raw_input, run_ui);

        self.egui_sdl2
            .handle_platform_output(clipboard, &self.egui_ctx, platform_output);

        self.shapes = shapes;
        self.textures_delta.append(textures_delta);
        repaint_after
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint(&mut self, window: &sdl2::video::Window) {
        let shapes = std::mem::take(&mut self.shapes);
        let mut textures_delta = std::mem::take(&mut self.textures_delta);

        for (id, image_delta) in textures_delta.set {
            self.painter.set_texture(id, &image_delta);
        }

        let clipped_primitives = self.egui_ctx.tessellate(shapes);
        let dimensions = window.drawable_size();
        self.painter.paint_primitives(
            [dimensions.0, dimensions.1],
            self.egui_ctx.pixels_per_point(),
            &clipped_primitives,
        );

        for id in textures_delta.free.drain(..) {
            self.painter.free_texture(id);
        }
    }
}

impl Drop for EguiGlow {
    fn drop(&mut self) {
        log::trace!("Destroying the painter");
        self.painter.destroy();
    }
}
