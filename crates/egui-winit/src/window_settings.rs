use egui::ViewportBuilder;

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

    maximized: bool,

    /// Inner size of window in logical pixels
    inner_size_points: Option<egui::Vec2>,
}

impl WindowSettings {
    pub fn from_window(egui_zoom_factor: f32, window: &winit::window::Window) -> Self {
        let inner_size_points = window
            .inner_size()
            .to_logical::<f32>(egui_zoom_factor as f64 * window.scale_factor());

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
            maximized: window.is_maximized(),

            inner_size_points: Some(egui::vec2(
                inner_size_points.width,
                inner_size_points.height,
            )),
        }
    }

    pub fn inner_size_points(&self) -> Option<egui::Vec2> {
        self.inner_size_points
    }

    pub fn initialize_viewport_builder(
        &self,
        egui_zoom_factor: f32,
        event_loop: &winit::event_loop::ActiveEventLoop,
        mut viewport_builder: ViewportBuilder,
    ) -> ViewportBuilder {
        profiling::function_scope!();

        // `WindowBuilder::with_position` expects inner position in Macos, and outer position elsewhere
        // See [`winit::window::WindowBuilder::with_position`] for details.
        let pos_px = if cfg!(target_os = "macos") {
            self.inner_position_pixels
        } else {
            self.outer_position_pixels
        };
        if let Some(pos) = pos_px {
            let monitor_scale_factor = if let Some(inner_size_points) = self.inner_size_points {
                find_active_monitor(egui_zoom_factor, event_loop, inner_size_points, &pos)
                    .map_or(1.0, |monitor| monitor.scale_factor() as f32)
            } else {
                1.0
            };

            let scaled_pos = pos / (egui_zoom_factor * monitor_scale_factor);
            viewport_builder = viewport_builder.with_position(scaled_pos);
        }

        if let Some(inner_size_points) = self.inner_size_points {
            viewport_builder = viewport_builder
                .with_inner_size(inner_size_points)
                .with_fullscreen(self.fullscreen)
                .with_maximized(self.maximized);
        }

        viewport_builder
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

    pub fn clamp_position_to_monitors(
        &mut self,
        egui_zoom_factor: f32,
        event_loop: &winit::event_loop::ActiveEventLoop,
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
            clamp_pos_to_monitors(egui_zoom_factor, event_loop, inner_size_points, pos_px);
        }
        if let Some(pos_px) = &mut self.outer_position_pixels {
            clamp_pos_to_monitors(egui_zoom_factor, event_loop, inner_size_points, pos_px);
        }
    }
}

fn find_active_monitor(
    egui_zoom_factor: f32,
    event_loop: &winit::event_loop::ActiveEventLoop,
    window_size_pts: egui::Vec2,
    position_px: &egui::Pos2,
) -> Option<winit::monitor::MonitorHandle> {
    profiling::function_scope!();
    let monitors = event_loop.available_monitors();

    // default to primary monitor, in case the correct monitor was disconnected.
    let Some(mut active_monitor) = event_loop
        .primary_monitor()
        .or_else(|| event_loop.available_monitors().next())
    else {
        return None; // no monitors 🤷
    };

    let mut active_monitor_overlap = 0.0;
    for monitor in monitors {
        let window_size_px = window_size_pts * (egui_zoom_factor * monitor.scale_factor() as f32);
        let window_rect = egui::Rect::from_min_size(*position_px, window_size_px);
        let overlap = window_rect.intersect(monitor_rect_px(&monitor)).area();

        if active_monitor_overlap < overlap {
            active_monitor = monitor;
            active_monitor_overlap = overlap;
        }
    }

    Some(active_monitor)
}

fn monitor_rect_px(monitor: &winit::monitor::MonitorHandle) -> egui::Rect {
    let pos = monitor.position();
    let size = monitor.size();
    egui::Rect::from_min_size(
        egui::pos2(pos.x as f32, pos.y as f32),
        egui::vec2(size.width as f32, size.height as f32),
    )
}

fn clamp_pos_to_monitors(
    egui_zoom_factor: f32,
    event_loop: &winit::event_loop::ActiveEventLoop,
    window_size_pts: egui::Vec2,
    position_px: &mut egui::Pos2,
) {
    profiling::function_scope!();

    let Some(active_monitor) =
        find_active_monitor(egui_zoom_factor, event_loop, window_size_pts, position_px)
    else {
        return; // no monitors 🤷
    };

    let mut window_size_px =
        window_size_pts * (egui_zoom_factor * active_monitor.scale_factor() as f32);
    // Add size of title bar. This is 32 px by default in Win 10/11.
    if cfg!(target_os = "windows") {
        window_size_px += egui::Vec2::new(
            0.0,
            32.0 * egui_zoom_factor * active_monitor.scale_factor() as f32,
        );
    }
    let monitor_rect = monitor_rect_px(&active_monitor);

    // Window size cannot be negative or the subsequent `clamp` will panic.
    let window_size = (monitor_rect.size() - window_size_px).max(egui::Vec2::ZERO);
    // To get the maximum position, we get the rightmost corner of the display, then
    // subtract the size of the window to get the bottom right most value window.position
    // can have.
    *position_px = position_px.clamp(monitor_rect.min, monitor_rect.min + window_size);
}
