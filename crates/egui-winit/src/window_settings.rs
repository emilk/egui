use egui::ViewportBuilder;

/// On Mac and Windows, [`WindowSettings::initialize_window`] moves the window to its saved position
/// after it is created, and only then maximizes it or makes it fullscreen.
const PLACE_AFTER_CREATION: bool = cfg!(any(target_os = "macos", target_os = "windows"));

/// Position and size of a native window.
///
/// Windows and Linux position windows in physical pixels, which are unambiguous
/// across monitors with different scale factors. Mac positions them in logical points.
/// We store the position in pixels together with the scale factor it was saved with,
/// so it can be converted to the logical units winit and Mac expect at restore time.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowGeometry {
    /// Position of window frame/titlebar in physical pixels.
    outer_position_pixels: Option<egui::Pos2>,

    /// Scale factor of the window when it was saved, without egui zoom.
    ///
    /// Needed to convert `outer_position_pixels` to logical units, which
    /// `ViewportBuilder::with_position` and Mac's `set_outer_position` take.
    /// Stored because at restore time there is no window to ask yet, and the
    /// monitor winit creates it on may not be the one it was saved on.
    scale_factor: f32,

    /// Inner size of window in points.
    inner_size_points: egui::Vec2,
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
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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
            // On Windows, winit converts this back to pixels with the scale factor of the monitor
            // the window is created on, which can differ from `geometry.scale_factor`.
            // So on Mac and Windows, `initialize_window` sets the saved position again.
            viewport_builder =
                viewport_builder.with_position(pos / (egui_zoom_factor * geometry.scale_factor));
        }

        viewport_builder = viewport_builder.with_inner_size(geometry.inner_size_points);
        if PLACE_AFTER_CREATION {
            // Moving a maximized or fullscreen window resets it,
            // so `initialize_window` maximizes or enters fullscreen after moving.
            viewport_builder
                .with_fullscreen(false)
                .with_maximized(false)
        } else {
            viewport_builder
                .with_fullscreen(self.fullscreen)
                .with_maximized(self.maximized)
        }
    }

    pub fn initialize_window(&self, window: &winit::window::Window) {
        if !PLACE_AFTER_CREATION {
            return;
        }

        // The position from `initialize_viewport_builder` can be off:
        // on Windows, winit converts it to pixels with the scale factor of the monitor the window
        // was created on, not the one we saved it on.
        // On Mac, windows were sometimes not restored onto secondary monitors from it alone.
        // So set the saved position directly, in the unit each platform positions windows in:
        let geometry = &self.normal_geometry;
        if let Some(pos) = geometry.outer_position_pixels {
            if cfg!(target_os = "macos") {
                // Mac places windows in points. Pixels are ambiguous across monitors with different scale factors.
                let pos = pos / geometry.scale_factor;
                #[expect(
                    clippy::disallowed_types,
                    reason = "these are OS points, unaffected by egui zoom"
                )]
                window.set_outer_position(winit::dpi::LogicalPosition::new(pos.x, pos.y));
            } else {
                window.set_outer_position(winit::dpi::PhysicalPosition::new(pos.x, pos.y));
            }
        }

        if self.fullscreen && cfg!(target_os = "macos") {
            // Mac only enters fullscreen once the window is visible, animating from its current frame.
            // Maximize so the first frame fills the screen,
            // and enter fullscreen in `before_first_show`.
            window.set_maximized(true);
        } else if self.fullscreen {
            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        } else if self.maximized {
            crate::set_maximized(window, true);
        }
    }

    /// Call once the first frame is painted, right before showing the window.
    pub fn before_first_show(&self, window: &winit::window::Window) {
        if cfg!(target_os = "macos") && self.fullscreen {
            // Enter the fullscreen deferred by `initialize_window`.
            // Un-maximize first, so the window leaves fullscreen to its normal frame:
            window.set_maximized(false);
            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
        }
    }

    /// Call right after showing the window for the first time.
    pub fn after_first_show(&self, window: &winit::window::Window) {
        // X11 window managers ignore maximize and fullscreen requests for hidden windows,
        // so the ones from `initialize_viewport_builder` are lost:
        if is_x11(window) {
            if self.fullscreen {
                window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            } else if self.maximized {
                window.set_maximized(true);
            }
        }
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
        // If this happens on Mac, a maximized window gets zero size, so we drop the position.
        // If this happens on Windows, the window becomes invisible to the user 🤦‍♂️
        // So on Windows we clamp the position to the monitor it is on.
        let geometry = &mut self.normal_geometry;
        let Some(pos_px) = geometry.outer_position_pixels else {
            return;
        };

        if cfg!(target_os = "macos") {
            // Pixels mean different points on each Mac monitor, so compare in points:
            let pos = pos_px / geometry.scale_factor;
            let is_on_a_monitor = event_loop.available_monitors().any(|monitor| {
                (monitor_rect_px(&monitor) / monitor.scale_factor() as f32).contains(pos)
            });
            if !is_on_a_monitor {
                geometry.outer_position_pixels = None;
            }
        } else if cfg!(target_os = "windows") {
            let mut pos_px = pos_px;
            clamp_pos_to_monitors(
                egui_zoom_factor,
                geometry.scale_factor,
                event_loop,
                geometry.inner_size_points,
                &mut pos_px,
            );
            geometry.outer_position_pixels = Some(pos_px);
        }
    }
}

#[cfg(feature = "x11")]
fn is_x11(window: &winit::window::Window) -> bool {
    use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
    matches!(
        window.window_handle().map(|h| h.as_raw()),
        Ok(RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_))
    )
}

#[cfg(not(feature = "x11"))]
fn is_x11(_window: &winit::window::Window) -> bool {
    false
}

/// A monitor, as needed to decide where a saved window position ends up.
#[derive(Clone, Copy, Debug)]
struct MonitorPx {
    /// In physical pixels, in the virtual desktop.
    rect: egui::Rect,

    scale_factor: f32,
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
    saved_scale_factor: f32,
    event_loop: &winit::event_loop::ActiveEventLoop,
    window_size_pts: egui::Vec2,
    position_px: &mut egui::Pos2,
) {
    profiling::function_scope!();

    let monitors: Vec<MonitorPx> = event_loop
        .available_monitors()
        .map(|monitor| MonitorPx {
            rect: monitor_rect_px(&monitor),
            scale_factor: monitor.scale_factor() as f32,
        })
        .collect();

    // Default to the primary monitor, in case the correct monitor was disconnected.
    let default_monitor = event_loop
        .primary_monitor()
        .and_then(|primary| event_loop.available_monitors().position(|m| m == primary))
        .unwrap_or(0);

    // Add size of title bar. This is 32 px by default in Win 10/11.
    let title_bar_height = if cfg!(target_os = "windows") {
        32.0
    } else {
        0.0
    };

    clamp_pos_to_monitor_list(
        egui_zoom_factor,
        saved_scale_factor,
        window_size_pts,
        title_bar_height,
        &monitors,
        default_monitor,
        position_px,
    );
}

/// Index of the monitor that `window_rect_px` overlaps the most,
/// or `default_monitor` if it overlaps none of them.
fn find_active_monitor(
    window_rect_px: egui::Rect,
    monitors: &[MonitorPx],
    default_monitor: usize,
) -> usize {
    let mut active_monitor = default_monitor;
    let mut active_monitor_overlap = 0.0;
    for (i, monitor) in monitors.iter().enumerate() {
        let overlap = window_rect_px.intersect(monitor.rect).area();
        if active_monitor_overlap < overlap {
            active_monitor = i;
            active_monitor_overlap = overlap;
        }
    }
    active_monitor
}

/// Move `position_px` so the window is fully inside the monitor it is on.
///
/// `position_px` was saved together with `saved_scale_factor`, so that is the scale
/// the window has in the pixels of `position_px` - not the scale factor of whichever
/// monitor we are testing. Measuring the window with the scale factor of each candidate
/// would make it look three times as large on a 300% monitor, so it would "overlap"
/// that monitor the most, however small the part of it is that really is there.
fn clamp_pos_to_monitor_list(
    egui_zoom_factor: f32,
    saved_scale_factor: f32,
    window_size_pts: egui::Vec2,
    title_bar_height: f32,
    monitors: &[MonitorPx],
    default_monitor: usize,
    position_px: &mut egui::Pos2,
) {
    let Some(default_monitor) =
        (!monitors.is_empty()).then(|| default_monitor.min(monitors.len() - 1))
    else {
        return; // no monitors 🤷
    };

    let saved_window_size_px = window_size_pts * (egui_zoom_factor * saved_scale_factor);
    let saved_window_rect = egui::Rect::from_min_size(*position_px, saved_window_size_px);
    let active_monitor =
        monitors[find_active_monitor(saved_window_rect, monitors, default_monitor)];

    // The window is placed on that monitor, so from here on it has the monitor's scale factor:
    let mut window_size_px = window_size_pts * (egui_zoom_factor * active_monitor.scale_factor);
    window_size_px.y += title_bar_height * egui_zoom_factor * active_monitor.scale_factor;

    // Window size cannot be negative or the subsequent `clamp` will panic.
    let window_size = (active_monitor.rect.size() - window_size_px).max(egui::Vec2::ZERO);
    // To get the maximum position, we get the rightmost corner of the display, then
    // subtract the size of the window to get the bottom right most value window.position
    // can have.
    *position_px = position_px.clamp(
        active_monitor.rect.min,
        active_monitor.rect.min + window_size,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(x: f32, y: f32, w: f32, h: f32, scale_factor: f32) -> MonitorPx {
        MonitorPx {
            rect: egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h)),
            scale_factor,
        }
    }

    /// Ultrawide 100% monitor in the middle (primary), a 4K 300% TV to its right,
    /// and a 100% monitor to its left.
    fn mixed_dpi_setup() -> [MonitorPx; 3] {
        [
            monitor(0.0, 0.0, 3440.0, 1440.0, 1.0),
            monitor(3440.0, 0.0, 3840.0, 2160.0, 3.0),
            monitor(-1920.0, 0.0, 1920.0, 1080.0, 1.0),
        ]
    }

    const TITLE_BAR: f32 = 32.0;

    fn clamped(
        saved_scale_factor: f32,
        size_pts: egui::Vec2,
        monitors: &[MonitorPx],
        pos: egui::Pos2,
    ) -> egui::Pos2 {
        let mut pos = pos;
        clamp_pos_to_monitor_list(
            1.0,
            saved_scale_factor,
            size_pts,
            TITLE_BAR,
            monitors,
            0,
            &mut pos,
        );
        pos
    }

    /// A window on a 100% monitor must not be moved to a neighboring 300% monitor,
    /// just because it would be large enough to overlap it if it had that monitor's scale factor.
    #[test]
    fn window_on_100_percent_monitor_stays_when_neighbor_has_300_percent() {
        let monitors = mixed_dpi_setup();
        let size = egui::vec2(1120.0, 760.0);

        // Windows at these positions used to be moved onto the 300% monitor:
        for pos in [
            egui::pos2(1996.0, 600.0),
            egui::pos2(1463.0, 332.0),
            egui::pos2(330.0, 184.0),
            egui::pos2(0.0, 0.0),
        ] {
            assert_eq!(clamped(1.0, size, &monitors, pos), pos);
        }
    }

    #[test]
    fn window_on_100_percent_monitor_is_clamped_on_that_monitor() {
        let monitors = mixed_dpi_setup();
        let size = egui::vec2(1120.0, 760.0);

        // Too low: the window plus its title bar would leave the monitor at the bottom.
        assert_eq!(
            clamped(1.0, size, &monitors, egui::pos2(1996.0, 715.0)),
            egui::pos2(1996.0, 1440.0 - 760.0 - TITLE_BAR),
        );
    }

    #[test]
    fn window_on_300_percent_monitor_stays_there() {
        let monitors = mixed_dpi_setup();
        let size = egui::vec2(600.0, 400.0); // 1800 x 1200 px on the TV
        let pos = egui::pos2(3600.0, 100.0);
        assert_eq!(clamped(3.0, size, &monitors, pos), pos);
    }

    #[test]
    fn window_mostly_on_300_percent_monitor_is_kept_on_it() {
        let monitors = mixed_dpi_setup();
        let size = egui::vec2(600.0, 400.0); // 1800 x 1200 px on the TV
        // Sticking out to the right of the TV: pulled back onto it, not onto another monitor.
        assert_eq!(
            clamped(3.0, size, &monitors, egui::pos2(6000.0, 100.0)),
            egui::pos2(7280.0 - 1800.0, 100.0),
        );
    }

    #[test]
    fn window_on_disconnected_monitor_ends_up_on_default_monitor() {
        let monitors = mixed_dpi_setup();
        let size = egui::vec2(1120.0, 760.0);
        assert_eq!(
            clamped(1.0, size, &monitors, egui::pos2(20_000.0, 50.0)),
            egui::pos2(3440.0 - 1120.0, 50.0),
        );
    }

    #[test]
    fn no_monitors_does_not_panic() {
        let pos = egui::pos2(100.0, 100.0);
        assert_eq!(clamped(1.0, egui::vec2(100.0, 100.0), &[], pos), pos);
    }

    #[test]
    fn find_active_monitor_picks_largest_overlap() {
        let monitors = mixed_dpi_setup();
        let window =
            |x: f32| egui::Rect::from_min_size(egui::pos2(x, 100.0), egui::vec2(1000.0, 500.0));
        assert_eq!(find_active_monitor(window(100.0), &monitors, 0), 0);
        assert_eq!(find_active_monitor(window(2800.0), &monitors, 2), 0); // 640 px on main, 360 px on the TV
        assert_eq!(find_active_monitor(window(3000.0), &monitors, 0), 1); // 440 px on main, 560 px on the TV
        assert_eq!(find_active_monitor(window(-1500.0), &monitors, 0), 2);
        assert_eq!(find_active_monitor(window(50_000.0), &monitors, 2), 2); // none: default
    }
}
