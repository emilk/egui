use std::{any::Any, sync::Arc};

use crate::*;

/// Information passed along with [`PaintCallback`] ([`Shape::Callback`]).
pub struct PaintCallbackInfo {
    /// Viewport in points.
    ///
    /// This specifies where on the screen to paint, and the borders of this
    /// Rect is the [-1, +1] of the Normalized Device Coordinates.
    ///
    /// Note than only a portion of this may be visible due to [`Self::clip_rect`].
    ///
    /// This comes from [`PaintCallback::rect`].
    pub viewport: Rect,

    /// Clip rectangle in points.
    pub clip_rect: Rect,

    /// Pixels per point.
    pub pixels_per_point: f32,

    /// Full size of the screen, in pixels.
    pub screen_size_px: [u32; 2],
}

#[test]
fn test_viewport_rounding() {
    for i in 0..=10_000 {
        // Two adjacent viewports should never overlap:
        let x = i as f32 / 97.0;
        let left = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0)).with_max_x(x);
        let right = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0)).with_min_x(x);

        for pixels_per_point in [0.618, 1.0, std::f32::consts::PI] {
            let left = ViewportInPixels::from_points(&left, pixels_per_point, [100, 100]);
            let right = ViewportInPixels::from_points(&right, pixels_per_point, [100, 100]);
            assert_eq!(left.left_px + left.width_px, right.left_px);
        }
    }
}

impl PaintCallbackInfo {
    /// The viewport rectangle. This is what you would use in e.g. `glViewport`.
    pub fn viewport_in_pixels(&self) -> ViewportInPixels {
        ViewportInPixels::from_points(&self.viewport, self.pixels_per_point, self.screen_size_px)
    }

    /// The "scissor" or "clip" rectangle. This is what you would use in e.g. `glScissor`.
    pub fn clip_rect_in_pixels(&self) -> ViewportInPixels {
        ViewportInPixels::from_points(&self.clip_rect, self.pixels_per_point, self.screen_size_px)
    }
}

/// If you want to paint some 3D shapes inside an egui region, you can use this.
///
/// This is advanced usage, and is backend specific.
#[derive(Clone)]
pub struct PaintCallback {
    /// Where to paint.
    ///
    /// This will become [`PaintCallbackInfo::viewport`].
    pub rect: Rect,

    /// Paint something custom (e.g. 3D stuff).
    ///
    /// The concrete value of `callback` depends on the rendering backend used. For instance, the
    /// `glow` backend requires that callback be an `egui_glow::CallbackFn` while the `wgpu`
    /// backend requires a `egui_wgpu::Callback`.
    ///
    /// If the type cannot be downcast to the type expected by the current backend the callback
    /// will not be drawn.
    ///
    /// The rendering backend is responsible for first setting the active viewport to
    /// [`Self::rect`].
    ///
    /// The rendering backend is also responsible for restoring any state, such as the bound shader
    /// program, vertex array, etc.
    ///
    /// Shape has to be clone, therefore this has to be an `Arc` instead of a `Box`.
    pub callback: Arc<dyn Any + Send + Sync>,
}

impl std::fmt::Debug for PaintCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomShape")
            .field("rect", &self.rect)
            .finish_non_exhaustive()
    }
}

impl std::cmp::PartialEq for PaintCallback {
    fn eq(&self, other: &Self) -> bool {
        self.rect.eq(&other.rect) && Arc::ptr_eq(&self.callback, &other.callback)
    }
}

impl From<PaintCallback> for Shape {
    #[inline(always)]
    fn from(shape: PaintCallback) -> Self {
        Self::Callback(shape)
    }
}
