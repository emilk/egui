/// Can be used to store window settings (position and size).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowSettings {
    /// outer position of window in physical pixels
    pos: Option<egui::Pos2>,
    /// Inner size of window in logical pixels
    inner_size_points: Option<egui::Vec2>,
}

impl WindowSettings {
    pub fn from_display(window: &winit::window::Window) -> Self {
        let inner_size_points = window.inner_size().to_logical::<f32>(window.scale_factor());

        Self {
            pos: window
                .outer_position()
                .ok()
                .map(|p| egui::pos2(p.x as f32, p.y as f32)),

            inner_size_points: Some(egui::vec2(
                inner_size_points.width as f32,
                inner_size_points.height as f32,
            )),
        }
    }

    pub fn initialize_window(
        &self,
        mut window: winit::window::WindowBuilder,
    ) -> winit::window::WindowBuilder {
        if !cfg!(target_os = "windows") {
            // If the app last ran on two monitors and only one is now connected, then
            // the given position is invalid.
            // If this happens on Mac, the window is clamped into valid area.
            // If this happens on Windows, the window is hidden and impossible to bring to get at.
            // So we don't restore window positions on Windows.
            if let Some(pos) = self.pos {
                window = window.with_position(winit::dpi::PhysicalPosition {
                    x: pos.x as f64,
                    y: pos.y as f64,
                });
            }
        }

        if let Some(inner_size_points) = self.inner_size_points {
            window.with_inner_size(winit::dpi::LogicalSize {
                width: inner_size_points.x as f64,
                height: inner_size_points.y as f64,
            })
        } else {
            window
        }
    }
}
