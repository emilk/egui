use crate::Harness;
use egui::{Pos2, Rect, Vec2};

/// Builder for [`Harness`].
pub struct HarnessBuilder {
    pub(crate) screen_rect: Rect,
    pub(crate) dpi: f32,
}

impl Default for HarnessBuilder {
    fn default() -> Self {
        Self {
            screen_rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)),
            dpi: 1.0,
        }
    }
}

impl HarnessBuilder {
    /// Set the size of the window.
    #[inline]
    pub fn with_size(mut self, size: impl Into<Vec2>) -> Self {
        let size = size.into();
        self.screen_rect.set_width(size.x);
        self.screen_rect.set_height(size.y);
        self
    }

    /// Set the DPI of the window.
    #[inline]
    pub fn with_dpi(mut self, dpi: f32) -> Self {
        self.dpi = dpi;
        self
    }

    /// Create a new Harness with the given app closure.
    ///
    /// The ui closure will immediately be called once to create the initial ui.
    ///
    /// # Example
    /// ```rust
    /// # use egui::CentralPanel;
    /// # use egui_kittest::Harness;
    /// let mut harness = Harness::builder()
    ///     .with_size(egui::Vec2::new(300.0, 200.0))
    ///     .build(|ctx| {
    ///         CentralPanel::default().show(ctx, |ui| {
    ///             ui.label("Hello, world!");
    ///         });
    ///     });
    /// ```
    pub fn build<'a>(self, app: impl FnMut(&egui::Context) + 'a) -> Harness<'a> {
        Harness::from_builder(&self, app)
    }
}
