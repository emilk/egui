use egui::ViewportBuilder;

#[cfg(target_os = "windows")]
mod windows_placement;

#[cfg(target_os = "windows")]
use windows_placement::WindowsPlacement;

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

    /// Windows-only: raw `WINDOWPLACEMENT` for mixed-DPI / multi-monitor restore.
    #[cfg(target_os = "windows")]
    #[cfg_attr(feature = "serde", serde(default))]
    windows_placement: Option<WindowsPlacement>,
}

impl WindowSettings {
    pub fn from_window(egui_zoom_factor: f32, window: &winit::window::Window) -> Self {
        let inner_size_points = window
            .inner_size()
            .to_logical::<f32>(egui_zoom_factor as f64 * window.scale_factor());

        let mut settings = Self {
            inner_position_pixels: window
                .inner_position()
                .ok()
                .map(|p| egui::pos2(p.x as f32, p.y as f32)),
            outer_position_pixels: window
                .outer_position()
                .ok()
                .map(|p| egui::pos2(p.x as f32, p.y as f32)),
            fullscreen: window.fullscreen().is_some(),
            maximized: window.is_maximized(),
            inner_size_points: Some(egui::vec2(
                inner_size_points.width,
                inner_size_points.height,
            )),
            #[cfg(target_os = "windows")]
            windows_placement: None,
        };

        #[cfg(target_os = "windows")]
        if let Some(placement) = windows_placement::capture(window) {
            windows_placement::apply_to_settings(&mut settings, placement);
        }

        settings
    }

    pub fn inner_size_points(&self) -> Option<egui::Vec2> {
        self.inner_size_points
    }

    pub fn initialize_viewport_builder(
        &self,
        egui_zoom_factor: f32,
        event_loop: &winit::event_loop::ActiveEventLoop,
        viewport_builder: ViewportBuilder,
    ) -> ViewportBuilder {
        profiling::function_scope!();

        #[cfg(target_os = "windows")]
        if let Some(placement) = self.windows_placement {
            return windows_placement::viewport_builder(
                self,
                egui_zoom_factor,
                viewport_builder,
                placement,
            );
        }

        apply_cross_platform_builder(self, egui_zoom_factor, event_loop, viewport_builder)
    }

    pub fn initialize_window(&self, window: &winit::window::Window) {
        #[cfg(target_os = "windows")]
        if let Some(placement) = self.windows_placement {
            windows_placement::apply_hidden(window, placement);
            return;
        }

        if cfg!(target_os = "macos") {
            // Mac sometimes has problems restoring the window to secondary monitors
            // using only `WindowBuilder::with_position`, so we need this extra step:
            if let Some(pos) = self.outer_position_pixels {
                window.set_outer_position(winit::dpi::PhysicalPosition { x: pos.x, y: pos.y });
            }
        }
    }

    /// First-frame show: on Windows, cloaked until [`Self::finish_first_show`].
    pub fn reveal_window(&self, window: &winit::window::Window) {
        #[cfg(target_os = "windows")]
        windows_placement::begin_reveal(window, self.windows_placement);
        #[cfg(not(target_os = "windows"))]
        window.set_visible(true);
    }

    /// Uncloak after the first present (no-op off Windows).
    pub fn finish_first_show(window: &winit::window::Window) {
        #[cfg(target_os = "windows")]
        windows_placement::end_reveal(window);
        #[cfg(not(target_os = "windows"))]
        let _ = window;
    }

    pub fn clamp_size_to_sane_values(&mut self, largest_monitor_size_points: egui::Vec2) {
        use egui::NumExt as _;

        #[cfg(target_os = "windows")]
        if self.windows_placement.is_some() {
            return;
        }

        if let Some(size) = &mut self.inner_size_points {
            let min_size = egui::Vec2::splat(64.0);
            *size = size.at_least(min_size);
            // Linux can crash if the window is larger than the largest monitor.
            *size = size.at_most(largest_monitor_size_points);
        }
    }

    pub fn clamp_position_to_monitors(
        &mut self,
        egui_zoom_factor: f32,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        // Windows: off-monitor positions make the window invisible; clamp.
        // Mac clamps itself. Skip when Win32 placement will clamp.
        if !cfg!(target_os = "windows") {
            return;
        }
        #[cfg(target_os = "windows")]
        if self.windows_placement.is_some() {
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

fn apply_cross_platform_builder(
    settings: &WindowSettings,
    egui_zoom_factor: f32,
    event_loop: &winit::event_loop::ActiveEventLoop,
    mut viewport_builder: ViewportBuilder,
) -> ViewportBuilder {
    // `with_position` expects inner position on macOS, outer elsewhere.
    let pos_px = if cfg!(target_os = "macos") {
        settings.inner_position_pixels
    } else {
        settings.outer_position_pixels
    };
    if let Some(pos) = pos_px {
        let monitor_scale = settings.inner_size_points.map_or(1.0, |size| {
            find_active_monitor(egui_zoom_factor, event_loop, size, &pos)
                .map_or(1.0, |m| m.scale_factor() as f32)
        });
        viewport_builder = viewport_builder.with_position(pos / (egui_zoom_factor * monitor_scale));
    }
    if let Some(inner_size_points) = settings.inner_size_points {
        viewport_builder = viewport_builder
            .with_inner_size(inner_size_points)
            .with_fullscreen(settings.fullscreen)
            .with_maximized(settings.maximized);
    }
    viewport_builder
}

fn find_active_monitor(
    egui_zoom_factor: f32,
    event_loop: &winit::event_loop::ActiveEventLoop,
    window_size_pts: egui::Vec2,
    position_px: &egui::Pos2,
) -> Option<winit::monitor::MonitorHandle> {
    profiling::function_scope!();
    let monitors = event_loop.available_monitors();
    let mut active_monitor = event_loop
        .primary_monitor()
        .or_else(|| event_loop.available_monitors().next())?;

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
        return;
    };

    let mut window_size_px = window_size_pts * (egui_zoom_factor * active_monitor.scale_factor() as f32);
    // Title bar is ~32 px by default in Win 10/11.
    if cfg!(target_os = "windows") {
        window_size_px += egui::Vec2::new(
            0.0,
            32.0 * egui_zoom_factor * active_monitor.scale_factor() as f32,
        );
    }
    let monitor_rect = monitor_rect_px(&active_monitor);
    let window_size = (monitor_rect.size() - window_size_px).max(egui::Vec2::ZERO);
    *position_px = position_px.clamp(monitor_rect.min, monitor_rect.min + window_size);
}
