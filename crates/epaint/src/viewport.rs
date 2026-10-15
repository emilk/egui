use crate::Rect;

/// Size of the viewport in whole, physical pixels.
pub struct ViewportInPixels {
    /// Physical pixel offset for left side of the viewport.
    pub left_px: i32,

    /// Physical pixel offset for top side of the viewport.
    pub top_px: i32,

    /// Physical pixel offset for bottom side of the viewport.
    ///
    /// This is what `glViewport`, `glScissor` etc expects for the y axis.
    pub from_bottom_px: i32,

    /// Viewport width in physical pixels.
    pub width_px: i32,

    /// Viewport height in physical pixels.
    pub height_px: i32,
}

impl ViewportInPixels {
    /// Convert from ui points.
    pub fn from_points(rect: &Rect, pixels_per_point: f32, screen_size_px: [u32; 2]) -> Self {
        // Fractional pixel values for viewports are generally valid, but may cause sampling issues
        // and rounding errors might cause us to get out of bounds.

        // Round:
        let left_px = (pixels_per_point * rect.min.x).round() as i32; // inclusive
        let top_px = (pixels_per_point * rect.min.y).round() as i32; // inclusive
        let right_px = (pixels_per_point * rect.max.x).round() as i32; // exclusive
        let bottom_px = (pixels_per_point * rect.max.y).round() as i32; // exclusive

        // Clamp to screen:
        let screen_width = screen_size_px[0] as i32;
        let screen_height = screen_size_px[1] as i32;
        let left_px = left_px.clamp(0, screen_width);
        let right_px = right_px.clamp(left_px, screen_width);
        let top_px = top_px.clamp(0, screen_height);
        let bottom_px = bottom_px.clamp(top_px, screen_height);

        let width_px = right_px - left_px;
        let height_px = bottom_px - top_px;

        Self {
            left_px,
            top_px,
            from_bottom_px: screen_height - height_px - top_px,
            width_px,
            height_px,
        }
    }
}
