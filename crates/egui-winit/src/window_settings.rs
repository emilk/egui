/// Can be used to store native window settings (position and size).
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct WindowSettings {
    /// Position of window content in physical pixels.
    inner_position_pixels: Option<egui::Pos2>,

    /// Position of window frame/titlebar in physical pixels.
    outer_position_pixels: Option<egui::Pos2>,

    fullscreen: bool,

    /// Inner size of window in logical pixels
    inner_size_points: Option<egui::Vec2>,
}

impl WindowSettings {
    pub fn from_display(window: &winit::window::Window) -> Self {
        let inner_size_points = window.inner_size().to_logical::<f32>(window.scale_factor());

        let inner_position_pixels = window
            .inner_position()
            .ok()
            .map(|p| egui::pos2(p.x as f32, p.y as f32));

        let outer_position_pixels = window
            .outer_position()
            .ok()
            .map(|p| egui::pos2(p.x as f32, p.y as f32));

        Self {
            inner_position_pixels,
            outer_position_pixels,

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

    pub fn initialize_window_builder(
        &self,
        mut window: winit::window::WindowBuilder,
    ) -> winit::window::WindowBuilder {
        // `WindowBuilder::with_position` expects inner position in Macos, and outer position elsewhere
        // See [`winit::window::WindowBuilder::with_position`] for details.
        let pos_px = if cfg!(target_os = "macos") {
            self.inner_position_pixels
        } else {
            self.outer_position_pixels
        };
        if let Some(pos_px) = pos_px {
            window = window.with_position(winit::dpi::PhysicalPosition {
                x: pos_px.x as f64,
                y: pos_px.y as f64,
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

    pub fn initialize_window(&self, window: &winit::window::Window) {
        if cfg!(target_os = "macos") {
            // Mac sometimes has problems restoring the window to secondary monitors
            // using only `WindowBuilder::with_position`, so we need this extra step:
            if let Some(pos) = self.outer_position_pixels {
                window.set_outer_position(winit::dpi::PhysicalPosition { x: pos.x, y: pos.y });
            }
        }
    }

    pub fn clamp_size_to_sane_values(&mut self, largest_monitor_size_points: egui::Vec2) {
        use egui::NumExt as _;

        if let Some(size) = &mut self.inner_size_points {
            // Prevent ridiculously small windows:
            let min_size = egui::Vec2::splat(64.0);
            *size = size.at_least(min_size);

            // Make sure we don't try to create a window larger than the largest monitor
            // because on Linux that can lead to a crash.
            *size = size.at_most(largest_monitor_size_points);
        }
    }

    pub fn clamp_position_to_monitors<E>(
        &mut self,
        event_loop: &winit::event_loop::EventLoopWindowTarget<E>,
    ) {
        // If the app last ran on two monitors and only one is now connected, then
        // the given position is invalid.
        // If this happens on Mac, the window is clamped into valid area.
        // If this happens on Windows, the window becomes invisible to the user 🤦‍♂️
        // So on Windows we clamp the position to the monitor it is on.
        if !cfg!(target_os = "windows") {
            return;
        }

        let Some(inner_size_points) = self.inner_size_points else {
            return;
        };

        if let Some(pos_px) = &mut self.inner_position_pixels {
            clamp_pos_to_monitors(event_loop, inner_size_points, pos_px);
        }
        if let Some(pos_px) = &mut self.outer_position_pixels {
            clamp_pos_to_monitors(event_loop, inner_size_points, pos_px);
        }
    }
}

fn clamp_pos_to_monitors<E>(
    event_loop: &winit::event_loop::EventLoopWindowTarget<E>,
    window_size_pts: egui::Vec2,
    position_px: &mut egui::Pos2,
) {
    let monitors = event_loop.available_monitors();

    // default to primary monitor, in case the correct monitor was disconnected.
    let Some(mut active_monitor) = event_loop
        .primary_monitor()
        .or_else(|| event_loop.available_monitors().next())
    else {
        return; // no monitors 🤷
    };

    for monitor in monitors {
        let window_size_px = window_size_pts * (monitor.scale_factor() as f32);
        let monitor_x_range = (monitor.position().x - window_size_px.x as i32)
            ..(monitor.position().x + monitor.size().width as i32);
        let monitor_y_range = (monitor.position().y - window_size_px.y as i32)
            ..(monitor.position().y + monitor.size().height as i32);

        if monitor_x_range.contains(&(position_px.x as i32))
            && monitor_y_range.contains(&(position_px.y as i32))
        {
            active_monitor = monitor;
        }
    }

    let mut window_size_px = window_size_pts * (active_monitor.scale_factor() as f32);
    // Add size of title bar. This is 32 px by default in Win 10/11.
    if cfg!(target_os = "windows") {
        window_size_px += egui::Vec2::new(0.0, 32.0 * active_monitor.scale_factor() as f32);
    }
    let monitor_position = egui::Pos2::new(
        active_monitor.position().x as f32,
        active_monitor.position().y as f32,
    );
    let monitor_size_px = egui::Vec2::new(
        active_monitor.size().width as f32,
        active_monitor.size().height as f32,
    );

    // Window size cannot be negative or the subsequent `clamp` will panic.
    let window_size = (monitor_size_px - window_size_px).max(egui::Vec2::ZERO);
    // To get the maximum position, we get the rightmost corner of the display, then
    // subtract the size of the window to get the bottom right most value window.position
    // can have.
    *position_px = position_px.clamp(monitor_position, monitor_position + window_size);
}
