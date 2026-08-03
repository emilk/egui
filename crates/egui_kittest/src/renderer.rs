use egui::TexturesDelta;
use std::mem;

pub trait TestRenderer {
    /// We use this to pass the glow / wgpu render state to [`eframe::Frame`].
    #[cfg(feature = "eframe")]
    fn setup_eframe(&self, _cc: &mut eframe::CreationContext<'_>, _frame: &mut eframe::Frame) {}

    /// Handle a [`TexturesDelta`] by updating the renderer's textures.
    fn handle_delta(&mut self, delta: &mut TexturesDelta);

    /// Render the [`crate::Harness`] and return the resulting image.
    ///
    /// # Errors
    /// Returns an error if the rendering fails.
    #[cfg(any(feature = "wgpu", feature = "snapshot"))]
    fn render(
        &mut self,
        ctx: &egui::Context,
        output: &egui::FullOutput,
    ) -> Result<image::RgbaImage, String>;
}

/// A lazy renderer that initializes the renderer on the first render call.
///
/// By default, this will create a wgpu renderer if the wgpu feature is enabled.
pub enum LazyRenderer {
    Uninitialized {
        textures_delta: TexturesDelta,
        builder: Option<Box<dyn FnOnce() -> Box<dyn TestRenderer>>>,
    },
    Initialized {
        renderer: Box<dyn TestRenderer>,
    },
}

impl Default for LazyRenderer {
    fn default() -> Self {
        #[cfg(feature = "wgpu")]
        return Self::new(crate::wgpu::WgpuTestRenderer::new);
        #[cfg(not(feature = "wgpu"))]
        return Self::Uninitialized {
            textures_delta: Default::default(),
            builder: None,
        };
    }
}

impl LazyRenderer {
    pub fn new<T: TestRenderer + 'static>(create_renderer: impl FnOnce() -> T + 'static) -> Self {
        Self::Uninitialized {
            textures_delta: Default::default(),
            builder: Some(Box::new(move || Box::new(create_renderer()))),
        }
    }
}

impl TestRenderer for LazyRenderer {
    fn handle_delta(&mut self, delta: &mut TexturesDelta) {
        match self {
            Self::Uninitialized {
                textures_delta: texture_ops,
                ..
            } => texture_ops.append(mem::take(delta)),
            Self::Initialized { renderer } => renderer.handle_delta(delta),
        }
    }

    #[cfg(any(feature = "wgpu", feature = "snapshot"))]
    fn render(
        &mut self,
        ctx: &egui::Context,
        output: &egui::FullOutput,
    ) -> Result<image::RgbaImage, String> {
        match self {
            Self::Uninitialized {
                textures_delta,
                builder: build,
            } => {
                let mut renderer = build.take().ok_or({
                    "No default renderer available. \
                    Enable the wgpu feature or set one via HarnessBuilder::renderer"
                })?();
                if !textures_delta.is_empty() {
                    renderer.handle_delta(textures_delta);
                }
                let image = renderer.render(ctx, output)?;
                *self = Self::Initialized { renderer };
                Ok(image)
            }
            Self::Initialized { renderer } => renderer.render(ctx, output),
        }
    }
}

impl Drop for LazyRenderer {
    fn drop(&mut self) {
        match self {
            Self::Uninitialized { textures_delta, .. } => {
                textures_delta.clear(); // Don't panic when dropping unapplied deltas
            }
            Self::Initialized { .. } => {}
        }
    }
}
