/// Can be used to store native window settings (position and size).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowSettings {
    /// Position of window in physical pixels. This is either
    /// the inner or outer position depending on the platform.
    /// See [`winit::window::WindowBuilder::with_position`] for details.
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

    pub fn inner_size_points(&self) -> Option<egui::Vec2> {
        self.inner_size_points
    }

    pub fn initialize_window(
        &self,
        mut window: winit::window::WindowBuilder,
    ) -> winit::window::WindowBuilder {
        // If the app last ran on two monitors and only one is now connected, then
        // the given position is invalid.
        // If this happens on Mac, the window is clamped into valid area.
        // If this happens on Windows, the clamping behavior is managed by the function
        // clamp_window_to_sane_position.
        if let Some(pos) = self.position {
            window = window.with_position(winit::dpi::PhysicalPosition {
                x: pos.x as f64,
                y: pos.y as f64,
            });
        }

        if let Some(inner_size_points) = self.inner_size_points {
            window
                .with_inner_size(winit::dpi::LogicalSize {
                    width: inner_size_points.x as f64,
                    height: inner_size_points.y as f64,
                })
                .with_fullscreen(
                    self.fullscreen
                        .then_some(winit::window::Fullscreen::Borderless(None)),
                )
        } else {
            window
        }
    }

    pub fn clamp_to_sane_values(&mut self, max_size: egui::Vec2) {
        use egui::NumExt as _;

        if let Some(size) = &mut self.inner_size_points {
            // Prevent ridiculously small windows
            let min_size = egui::Vec2::splat(64.0);
            *size = size.at_least(min_size);
            *size = size.at_most(max_size);
        }
    }

    pub fn clamp_window_to_sane_position<E>(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<E>,
    ) {
        if let (Some(position), Some(inner_size_points)) =
            (&mut self.position, &self.inner_size_points)
        {
            let monitors = event_loop.available_monitors();
            // default to primary monitor, in case the correct monitor was disconnected.
            let mut active_monitor = if let Some(active_monitor) = event_loop
                .primary_monitor()
                .or_else(|| event_loop.available_monitors().next())
            {
                active_monitor
            } else {
                return; // no monitors 🤷
            };
            for monitor in monitors {
                let monitor_x_range = (monitor.position().x - inner_size_points.x as i32)
                    ..(monitor.position().x + monitor.size().width as i32);
                let monitor_y_range = (monitor.position().y - inner_size_points.y as i32)
                    ..(monitor.position().y + monitor.size().height as i32);

                if monitor_x_range.contains(&(position.x as i32))
                    && monitor_y_range.contains(&(position.y as i32))
                {
                    active_monitor = monitor;
                }
            }

            let mut inner_size_pixels = *inner_size_points * (active_monitor.scale_factor() as f32);
            // Add size of title bar. This is 32 px by default in Win 10/11.
            if cfg!(target_os = "windows") {
                inner_size_pixels +=
                    egui::Vec2::new(0.0, 32.0 * active_monitor.scale_factor() as f32);
            }
            let monitor_position = egui::Pos2::new(
                active_monitor.position().x as f32,
                active_monitor.position().y as f32,
            );
            let monitor_size = egui::Vec2::new(
                active_monitor.size().width as f32,
                active_monitor.size().height as f32,
            );

            // Window size cannot be negative or the subsequent `clamp` will panic.
            let window_size = (monitor_size - inner_size_pixels).max(egui::Vec2::ZERO);
            // To get the maximum position, we get the rightmost corner of the display, then
            // subtract the size of the window to get the bottom right most value window.position
            // can have.
            *position = position.clamp(monitor_position, monitor_position + window_size);
        }
    }
}
