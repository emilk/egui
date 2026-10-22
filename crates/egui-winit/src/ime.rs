//! Preserve IME settings while winit's explicit capabilities are disabled.
use winit::{
    dpi::{PhysicalPosition, PhysicalSize, Position, Size},
    window::{
        ImeCapabilities, ImeEnableRequest, ImeHint, ImePurpose, ImeRequest, ImeRequestData, Window,
        WindowId,
    },
};

#[derive(Clone)]
struct Desired {
    purpose: ImePurpose,
    position: Position,
    size: Size,
}

impl Default for Desired {
    fn default() -> Self {
        Self {
            purpose: ImePurpose::Normal,
            position: PhysicalPosition::new(0, 0).into(),
            size: PhysicalSize::new(0, 0).into(),
        }
    }
}

impl Desired {
    fn enable_request(&self) -> Option<ImeEnableRequest> {
        ImeEnableRequest::new(
            ImeCapabilities::new()
                .with_hint_and_purpose()
                .with_cursor_area(),
            ImeRequestData::default()
                .with_hint_and_purpose(ImeHint::NONE, self.purpose)
                .with_cursor_area(self.position, self.size),
        )
    }
}

fn key(window: WindowId) -> egui::Id {
    egui::Id::unique(("winit-ime-settings", window))
}

fn remember(ctx: &egui::Context, key: egui::Id, update: impl FnOnce(&mut Desired)) -> Desired {
    ctx.data_mut(|data| {
        let mut desired = data.get_temp::<Desired>(key).unwrap_or_default();
        update(&mut desired);
        data.insert_temp(key, desired.clone());
        desired
    })
}

pub fn forget(ctx: &egui::Context, window: WindowId) {
    ctx.data_mut(|data| data.remove::<Desired>(key(window)));
}

/// Keep settings scoped to the State even if a viewport disappears before Destroyed
/// can be dispatched to it. Native window IDs may be reused by the next window.
pub struct Binding {
    ctx: egui::Context,
    window: WindowId,
}

impl Binding {
    pub fn bind(slot: &mut Option<Self>, ctx: &egui::Context, window: WindowId) {
        if slot.as_ref().is_none_or(|binding| binding.window != window) {
            *slot = Some(Self {
                ctx: ctx.clone(),
                window,
            });
        }
    }
}

impl Drop for Binding {
    fn drop(&mut self) {
        forget(&self.ctx, self.window);
    }
}

pub fn set_allowed(ctx: &egui::Context, window: &dyn Window, allowed: bool) {
    let request = if allowed {
        let desired = remember(ctx, key(window.id()), |_| {});
        let Some(request) = desired.enable_request() else {
            return;
        };
        ImeRequest::Enable(request)
    } else {
        // Disable capabilities without discarding the desired settings. Both ordinary
        // focus changes and composition interruption must restore them on re-enable.
        ImeRequest::Disable
    };
    let _ = window.request_ime_update(request);
}

pub fn set_purpose(ctx: &egui::Context, window: &dyn Window, purpose: ImePurpose) {
    remember(ctx, key(window.id()), |desired| desired.purpose = purpose);
    if window
        .ime_capabilities()
        .is_some_and(|caps| caps.hint_and_purpose())
    {
        let _ = window.request_ime_update(ImeRequest::Update(
            ImeRequestData::default().with_hint_and_purpose(ImeHint::NONE, purpose),
        ));
    }
}

pub fn set_cursor_area(ctx: &egui::Context, window: &dyn Window, position: Position, size: Size) {
    remember(ctx, key(window.id()), |desired| {
        desired.position = position;
        desired.size = size;
    });
    if window
        .ime_capabilities()
        .is_some_and(|caps| caps.cursor_area())
    {
        let _ = window.request_ime_update(ImeRequest::Update(
            ImeRequestData::default().with_cursor_area(position, size),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_before_enable_and_after_disable_are_preserved_per_window() {
        let ctx = egui::Context::default();
        let first = egui::Id::unique("first");
        let second = egui::Id::unique("second");
        let position = PhysicalPosition::new(25, 80).into();
        let size = PhysicalSize::new(10, 20).into();
        // Viewport commands may arrive before IMEAllowed(true).
        remember(&ctx, first, |desired| {
            desired.purpose = ImePurpose::Password;
            desired.position = position;
            desired.size = size;
        });
        for _ in 0..2 {
            // Disabling does not remove the desired data. Enabling repeatedly must
            // restore the same password purpose and candidate-window position.
            let request = remember(&ctx, first, |_| {}).enable_request().unwrap();
            assert_eq!(
                request.request_data().hint_and_purpose,
                Some((ImeHint::NONE, ImePurpose::Password))
            );
            assert_eq!(request.request_data().cursor_area, Some((position, size)));
        }
        let other = remember(&ctx, second, |_| {}).enable_request().unwrap();
        assert_eq!(
            other.request_data().hint_and_purpose,
            Some((ImeHint::NONE, ImePurpose::Normal))
        );
    }

    #[test]
    fn dropping_window_state_clears_settings_before_native_id_reuse() {
        let ctx = egui::Context::default();
        let window = WindowId::from_raw(42);
        let mut binding = None;
        Binding::bind(&mut binding, &ctx, window);
        remember(&ctx, key(window), |desired| {
            desired.purpose = ImePurpose::Password;
        });
        drop(binding);
        let fresh = remember(&ctx, key(window), |_| {})
            .enable_request()
            .unwrap();
        assert_eq!(
            fresh.request_data().hint_and_purpose,
            Some((ImeHint::NONE, ImePurpose::Normal))
        );
    }
}
