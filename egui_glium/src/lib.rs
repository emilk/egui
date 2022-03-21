//! [`egui`] bindings for [`glium`](https://github.com/glium/glium).
//!
//! The main type you want to use is [`EguiGlium`].
//!
//! If you are writing an app, you may want to look at [`eframe`](https://docs.rs/eframe) instead.

#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

mod painter;
pub use painter::Painter;

pub use egui_winit;

// ----------------------------------------------------------------------------

/// Convenience wrapper for using [`egui`] from a [`glium`] app.
pub struct EguiGlium {
    pub egui_ctx: egui::Context,
    pub egui_winit: egui_winit::State,
    pub painter: crate::Painter,

    shapes: Vec<egui::epaint::ClippedShape>,
    textures_delta: egui::TexturesDelta,
}

impl EguiGlium {
    pub fn new(display: &glium::Display) -> Self {
        let painter = crate::Painter::new(display);
        Self {
            egui_ctx: Default::default(),
            egui_winit: egui_winit::State::new(
                painter.max_texture_side(),
                display.gl_window().window(),
            ),
            painter,
            shapes: Default::default(),
            textures_delta: Default::default(),
        }
    }

    /// Returns `true` if egui wants exclusive use of this event
    /// (e.g. a mouse click on an egui window, or entering text into a text field).
    /// For instance, if you use egui for a game, you want to first call this
    /// and only when this returns `false` pass on the events to your game.
    ///
    /// Note that egui uses `tab` to move focus between elements, so this will always return `true` for tabs.
    pub fn on_event(&mut self, event: &glium::glutin::event::WindowEvent<'_>) -> bool {
        self.egui_winit.on_event(&self.egui_ctx, event)
    }

    /// Returns `true` if egui requests a repaint.
    ///
    /// Call [`Self::paint`] later to paint.
    pub fn run(&mut self, display: &glium::Display, run_ui: impl FnMut(&egui::Context)) -> bool {
        let raw_input = self
            .egui_winit
            .take_egui_input(display.gl_window().window());
        let egui::FullOutput {
            platform_output,
            needs_repaint,
            textures_delta,
            shapes,
        } = self.egui_ctx.run(raw_input, run_ui);

        self.egui_winit.handle_platform_output(
            display.gl_window().window(),
            &self.egui_ctx,
            platform_output,
        );

        self.shapes = shapes;
        self.textures_delta.append(textures_delta);

        needs_repaint
    }

    /// Paint the results of the last call to [`Self::run`].
    pub fn paint<T: glium::Surface>(&mut self, display: &glium::Display, target: &mut T) {
        let shapes = std::mem::take(&mut self.shapes);
        let textures_delta = std::mem::take(&mut self.textures_delta);
        let clipped_primitives = self.egui_ctx.tessellate(shapes);
        self.painter.paint_and_update_textures(
            display,
            target,
            self.egui_ctx.pixels_per_point(),
            &clipped_primitives,
            &textures_delta,
        );
    }
}
