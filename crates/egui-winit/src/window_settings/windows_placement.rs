//! Win32 `WINDOWPLACEMENT` for mixed-DPI multi-monitor restore.
//!
//! Apply hidden (`SW_HIDE`) twice for `WM_DPICHANGED`
//! (<https://stackoverflow.com/questions/66632170>). Cloak until first present.

#![expect(unsafe_code)]
#![expect(clippy::undocumented_unsafe_blocks)]

use egui::ViewportBuilder;
use raw_window_handle::{HasWindowHandle as _, RawWindowHandle};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::{
        Dwm::{DwmSetWindowAttribute, DWMWA_CLOAK},
        Gdi::{GetMonitorInfoW, MonitorFromRect, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    },
    UI::{
        HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
        WindowsAndMessaging::{
            GetWindowLongPtrW, GetWindowPlacement, GetWindowRect, SetWindowLongPtrW,
            SetWindowPlacement, SetWindowPos, GWL_STYLE, SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE,
            SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, SW_SHOWNORMAL, WINDOWPLACEMENT,
            WPF_RESTORETOMAXIMIZED, WS_MAXIMIZE,
        },
    },
};

use super::WindowSettings;

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub(super) struct WindowsPlacement {
    flags: u32,
    show_cmd: u32,
    min_position: [i32; 2],
    max_position: [i32; 2],
    /// `rcNormalPosition` as `[left, top, right, bottom]`.
    normal: [i32; 4],
}

pub(super) fn capture(window: &winit::window::Window) -> Option<WindowsPlacement> {
    let hwnd = hwnd(window)?;
    let mut placement = empty_placement();
    if unsafe { GetWindowPlacement(hwnd, &mut placement) } == 0 {
        return None;
    }
    // Aero Snap keeps the *pre-snap* rect in `rcNormalPosition`; prefer what the user sees.
    prefer_visible_if_snapped(hwnd, &mut placement);
    Some(from_native(placement))
}

pub(super) fn apply_to_settings(settings: &mut WindowSettings, placement: WindowsPlacement) {
    // Keep live `inner_size_points` from `from_window` — re-deriving creeps under DPI.
    settings.outer_position_pixels =
        Some(egui::pos2(placement.normal[0] as f32, placement.normal[1] as f32));
    settings.inner_position_pixels = None;
    settings.maximized = is_maximized(placement);
    settings.windows_placement = Some(placement);
}

pub(super) fn viewport_builder(
    settings: &WindowSettings,
    egui_zoom_factor: f32,
    mut viewport_builder: ViewportBuilder,
    placement: WindowsPlacement,
) -> ViewportBuilder {
    let scale = dpi_scale(placement.normal) * egui_zoom_factor;
    let left = placement.normal[0] as f32 / scale;
    let top = placement.normal[1] as f32 / scale;
    // Position only — never `with_inner_size` (create DPI × placement DPI = growth).
    viewport_builder = viewport_builder.with_position([left, top]);
    if settings.fullscreen {
        viewport_builder = viewport_builder.with_fullscreen(true);
    }
    viewport_builder
}

pub(super) fn apply_hidden(window: &winit::window::Window, saved: WindowsPlacement) {
    let Some(hwnd) = hwnd(window) else {
        return;
    };
    set_placement(hwnd, saved, SW_HIDE as u32);
    set_placement(hwnd, saved, SW_HIDE as u32);
    if is_maximized(saved) {
        size_to_work_area(hwnd, saved.normal);
    }
}

/// Cloak, then show — still invisible until [`end_reveal`] after the first present.
pub(super) fn begin_reveal(window: &winit::window::Window, saved: Option<WindowsPlacement>) {
    if let Some(hwnd) = hwnd(window) {
        set_cloak(hwnd, true);
    }
    window.set_visible(true);
    if saved.is_some_and(is_maximized) {
        window.set_maximized(true);
    }
}

pub(super) fn end_reveal(window: &winit::window::Window) {
    if let Some(hwnd) = hwnd(window) {
        set_cloak(hwnd, false);
    }
}

fn prefer_visible_if_snapped(hwnd: HWND, placement: &mut WINDOWPLACEMENT) {
    if placement.showCmd == SW_SHOWMAXIMIZED as u32 || placement.showCmd == SW_SHOWMINIMIZED as u32
    {
        return;
    }
    let mut screen = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    if unsafe { GetWindowRect(hwnd, &mut screen) } == 0 {
        return;
    }
    // Size is invariant under screen→workspace; skip GetMonitorInfo when not snapped.
    if !size_differs(&screen, &placement.rcNormalPosition) {
        return;
    }
    placement.rcNormalPosition = screen_to_workspace(screen);
}

fn size_differs(a: &RECT, b: &RECT) -> bool {
    const TOL: i32 = 8;
    let aw = a.right - a.left;
    let ah = a.bottom - a.top;
    let bw = b.right - b.left;
    let bh = b.bottom - b.top;
    (aw - bw).abs() > TOL || (ah - bh).abs() > TOL
}

fn screen_to_workspace(screen: RECT) -> RECT {
    let monitor = unsafe { MonitorFromRect(&screen, MONITOR_DEFAULTTONEAREST) };
    let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if unsafe { GetMonitorInfoW(monitor, &mut info) } == 0 {
        return screen;
    }
    let dx = info.rcWork.left - info.rcMonitor.left;
    let dy = info.rcWork.top - info.rcMonitor.top;
    RECT {
        left: screen.left - dx,
        top: screen.top - dy,
        right: screen.right - dx,
        bottom: screen.bottom - dy,
    }
}

fn is_maximized(saved: WindowsPlacement) -> bool {
    resolved_show_cmd(saved) == SW_SHOWMAXIMIZED as u32
}

fn dpi_scale(normal: [i32; 4]) -> f32 {
    let rect = normal_rect(normal);
    let monitor = unsafe { MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST) };
    let mut dpi = [96u32; 2];
    if unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi[0], &mut dpi[1]) } < 0 {
        return 1.0;
    }
    dpi[0] as f32 / 96.0
}

fn set_cloak(hwnd: HWND, cloak: bool) {
    let value = i32::from(cloak);
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK as u32,
            std::ptr::from_ref(&value).cast(),
            std::mem::size_of_val(&value) as u32,
        );
    }
}

fn resolved_show_cmd(saved: WindowsPlacement) -> u32 {
    if saved.show_cmd != SW_SHOWMINIMIZED as u32 {
        return saved.show_cmd;
    }
    if saved.flags & WPF_RESTORETOMAXIMIZED != 0 {
        SW_SHOWMAXIMIZED as u32
    } else {
        SW_SHOWNORMAL as u32
    }
}

fn empty_placement() -> WINDOWPLACEMENT {
    let mut placement: WINDOWPLACEMENT = unsafe { std::mem::zeroed() };
    placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
    placement
}

fn from_native(placement: WINDOWPLACEMENT) -> WindowsPlacement {
    WindowsPlacement {
        flags: placement.flags,
        show_cmd: placement.showCmd,
        min_position: [placement.ptMinPosition.x, placement.ptMinPosition.y],
        max_position: [placement.ptMaxPosition.x, placement.ptMaxPosition.y],
        normal: [
            placement.rcNormalPosition.left,
            placement.rcNormalPosition.top,
            placement.rcNormalPosition.right,
            placement.rcNormalPosition.bottom,
        ],
    }
}

fn hwnd(window: &winit::window::Window) -> Option<HWND> {
    let RawWindowHandle::Win32(handle) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    Some(handle.hwnd.get() as HWND)
}

fn set_placement(hwnd: HWND, saved: WindowsPlacement, show_cmd: u32) {
    let placement = WINDOWPLACEMENT {
        length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
        flags: saved.flags,
        showCmd: show_cmd,
        ptMinPosition: POINT {
            x: saved.min_position[0],
            y: saved.min_position[1],
        },
        ptMaxPosition: POINT {
            x: saved.max_position[0],
            y: saved.max_position[1],
        },
        rcNormalPosition: normal_rect(saved.normal),
    };
    unsafe {
        let _ = SetWindowPlacement(hwnd, &placement);
    }
}

fn size_to_work_area(hwnd: HWND, normal: [i32; 4]) {
    let probe = normal_rect(normal);
    unsafe {
        let monitor = MonitorFromRect(&probe, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return;
        }
        let r = info.rcWork;
        let _ = SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            r.left,
            r.top,
            (r.right - r.left).max(1),
            (r.bottom - r.top).max(1),
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
        SetWindowLongPtrW(hwnd, GWL_STYLE, style | WS_MAXIMIZE as isize);
    }
}

fn normal_rect(normal: [i32; 4]) -> RECT {
    RECT {
        left: normal[0],
        top: normal[1],
        right: normal[2],
        bottom: normal[3],
    }
}
