/// Can be used to store native window settings (position and size).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowSettings {
    /// Position of window in physical pixels. This is either
    /// the inner or outer position depending on the platform.
    /// See [`winit::window::WindowAttributes`] for details.
    position: Option<egui::Pos2>,

    fullscreen: bool,

    /// Inner size of window in logical pixels
    inner_size_points: Option<egui::Vec2>,
}

impl WindowSettings {
    pub fn from_display(window: &winit::window::Window) -> Self {
        let inner_size_points = window.inner_size().to_logical::<f32>(window.scale_factor());
        let position = if cfg!(macos) {
            // MacOS uses inner position when positioning windows.
            window
                .inner_position()
                .ok()
                .map(|p| egui::pos2(p.x as f32, p.y as f32))
        } else {
            // Other platforms use the outer position.
            window
                .outer_position()
                .ok()
                .map(|p| egui::pos2(p.x as f32, p.y as f32))
        };

        Self {
            position,

            fullscreen: window.fullscreen().is_some(),

            inner_size_points: Some(egui::vec2(
                inner_size_points.width,
                inner_size_points.height,
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
            // If this happens on Windows, the window is hidden and very difficult to find.
            // So we don't restore window positions on Windows.
            if let Some(pos) = self.position {
                window = window.with_position(winit::dpi::PhysicalPosition {
                    x: pos.x as f64,
                    y: pos.y as f64,
                });
            }
        }

        if let Some(inner_size_points) = self.inner_size_points {
            window
                .with_inner_size(winit::dpi::LogicalSize {
                    width: inner_size_points.x as f64,
                    height: inner_size_points.y as f64,
                })
                .with_fullscreen(
                    self.fullscreen
                        .then(|| winit::window::Fullscreen::Borderless(None)),
                )
        } else {
            window
        }
    }
}
