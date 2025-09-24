use egui::{SafeAreaInsets, epaint::MarginF32};
use objc::runtime::Object;
use objc::{class, msg_send, sel, sel_impl};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct UIEdgeInsets {
    top: f64,
    left: f64,
    bottom: f64,
    right: f64,
}

/// Gets the ios safe area insets
/// A safe area defines the area within a view that isn’t covered by a navigation bar, tab bar,
/// toolbar, or other views a window might provide. Safe areas are essential for avoiding a device’s
/// interactive and display features, like Dynamic Island on iPhone or the camera housing on some
/// Mac models. For developer guidance, see Positioning content relative to the safe area.
///
/// Once winit v0.31 has been released this can be removed in favour of `winit::Window::safe_area`.
#[allow(unsafe_code)]
pub fn get_ios_safe_area_insets() -> UIEdgeInsets {
    unsafe {
        let shared_application: *mut Object = msg_send![class!(UIApplication), sharedApplication];
        let windows: *mut Object = msg_send![shared_application, windows];
        let first_object: *mut Object = msg_send![windows, firstObject];
        let safe_area_insets: UIEdgeInsets = msg_send![first_object, safeAreaInsets];
        safe_area_insets
    }
}

impl From<UIEdgeInsets> for SafeAreaInsets {
    fn from(value: UIEdgeInsets) -> Self {
        SafeAreaInsets(MarginF32 {
            top: value.top as f32,
            left: value.left as f32,
            bottom: value.bottom as f32,
            right: value.right as f32,
        })
    }
}
