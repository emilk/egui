use egui::ViewportBuilder;

/// Position and size of a native window.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct WindowGeometry {
    /// Position of window frame/titlebar in physical pixels.
    outer_position_pixels: Option<egui::Pos2>,

    /// Scale factor of the window, without egui zoom.
    scale_factor: f32,

    /// Inner size of window in points.
    inner_size_points: egui::Vec2,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            outer_position_pixels: None,
            scale_factor: 1.0,
            inner_size_points: egui::Vec2::ZERO,
        }
    }
}

impl WindowGeometry {
    pub fn from_window(egui_zoom_factor: f32, window: &winit::window::Window) -> Self {
        let scale_factor = window.scale_factor() as f32;
        let inner_size = window.inner_size();
        Self {
            outer_position_pixels: window
                .outer_position()
                .ok()
                .map(|p| egui::pos2(p.x as f32, p.y as f32)),
            scale_factor,
            inner_size_points: egui::vec2(inner_size.width as f32, inner_size.height as f32)
                / (egui_zoom_factor * scale_factor),
        }
    }
}

/// Can be used to store native window settings (position and size).
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct WindowSettings {
    /// Position and size when neither maximized nor fullscreen.
    normal_geometry: WindowGeometry,

    fullscreen: bool,

    maximized: bool,
}

impl WindowSettings {
    /// While maximized or fullscreen, keeps `last_normal_geometry`, so the window restores to it.
    pub fn from_window(
        egui_zoom_factor: f32,
        window: &winit::window::Window,
        last_normal_geometry: Option<WindowGeometry>,
    ) -> Self {
        let fullscreen = window.fullscreen().is_some();
        let maximized = window.is_maximized();
        let normal_geometry = last_normal_geometry
            .filter(|_| fullscreen || maximized)
            .unwrap_or_else(|| WindowGeometry::from_window(egui_zoom_factor, window));
        Self {
            normal_geometry,
            fullscreen,
            maximized,
        }
    }

    pub fn inner_size_points(&self) -> egui::Vec2 {
        self.normal_geometry.inner_size_points
    }

    /// Position and size when neither maximized nor fullscreen.
    pub fn normal_geometry(&self) -> WindowGeometry {
        self.normal_geometry
    }

    pub fn initialize_viewport_builder(
        &self,
        egui_zoom_factor: f32,
        mut viewport_builder: ViewportBuilder,
    ) -> ViewportBuilder {
        profiling::function_scope!();

        let geometry = &self.normal_geometry;

        if let Some(pos) = geometry.outer_position_pixels {
            // This only places the window roughly, so `initialize_window` moves it again.
            viewport_builder =
                viewport_builder.with_position(pos / (egui_zoom_factor * geometry.scale_factor));
        }

        viewport_builder = viewport_builder.with_inner_size(geometry.inner_size_points);
        if cfg!(target_os = "macos") && self.fullscreen {
            // Mac only enters fullscreen once the window is visible, animating from its current frame.
            // Start maximized so the first frame fills the screen,
            // and enter fullscreen in `enter_deferred_fullscreen`.
            viewport_builder.with_fullscreen(false).with_maximized(true)
        } else {
            viewport_builder
                .with_fullscreen(self.fullscreen)
                .with_maximized(self.maximized)
        }
    }

    pub fn initialize_window(&self, window: &winit::window::Window) {
        // `WindowBuilder::with_position` only places the window roughly, since winit converts
        // points with a different scale factor than we saved with. So move it exactly:
        let geometry = &self.normal_geometry;
        let Some(pos) = geometry.outer_position_pixels else {
            return;
        };
        let pos: winit::dpi::Position = if cfg!(target_os = "macos") {
            // Mac places windows in points. Pixels are ambiguous across monitors with different scale factors.
            let pos = pos / geometry.scale_factor;
            #[expect(
                clippy::disallowed_types,
                reason = "these are OS points, unaffected by egui zoom"
            )]
            winit::dpi::LogicalPosition::new(pos.x, pos.y).into()
        } else {
            winit::dpi::PhysicalPosition::new(pos.x, pos.y).into()
        };

        // Moving a maximized window un-maximizes it, so move the normal frame instead:
        let maximized = window.is_maximized();
        if maximized {
            window.set_maximized(false);
        }
        window.set_outer_position(pos);
        if maximized {
            window.set_maximized(true);
        }
    }

    /// Should [`Self::enter_deferred_fullscreen`] be called once the first frame is painted?
    pub fn enter_fullscreen_after_first_frame(&self) -> bool {
        cfg!(target_os = "macos") && self.fullscreen
    }

    /// Enter the fullscreen deferred by [`Self::initialize_viewport_builder`].
    ///
    /// Call once the first frame is painted, before showing the window.
    pub fn enter_deferred_fullscreen(window: &winit::window::Window) {
        // Un-maximize first, so the window leaves fullscreen to its normal frame:
        window.set_maximized(false);
        window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
    }

    pub fn clamp_size_to_sane_values(&mut self, largest_monitor_size_points: egui::Vec2) {
        use egui::NumExt as _;

        let size = &mut self.normal_geometry.inner_size_points;

        // Prevent ridiculously small windows:
        let min_size = egui::Vec2::splat(64.0);
        *size = size.at_least(min_size);

        // Make sure we don't try to create a window larger than the largest monitor
        // because on Linux that can lead to a crash.
        *size = size.at_most(largest_monitor_size_points);
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

        let geometry = &mut self.normal_geometry;
        if let Some(pos_px) = &mut geometry.outer_position_pixels {
            clamp_pos_to_monitors(
                egui_zoom_factor,
                event_loop,
                geometry.inner_size_points,
                pos_px,
            );
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
