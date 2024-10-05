use crate::Harness;
use egui::{Pos2, Rect, Vec2};

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
    pub fn with_size(mut self, size: Vec2) -> Self {
        self.screen_rect.set_width(size.x);
        self.screen_rect.set_height(size.y);
        self
    }

    pub fn with_dpi(mut self, dpi: f32) -> Self {
        self.dpi = dpi;
        self
    }

    pub fn build<'a>(self, app: impl FnMut(&egui::Context) + 'a) -> Harness<'a> {
        Harness::from_builder(&self, app)
    }
}
