/// Demonstrates how to make an app using egui.
///
/// Implements `epi::App` so it can be used with
/// [`egui_glium`](https://github.com/emilk/egui/tree/master/egui_glium), [`egui_glow`](https://github.com/emilk/egui/tree/master/egui_glow) and [`egui_web`](https://github.com/emilk/egui/tree/master/egui_web).
#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DemoApp {
    demo_windows: super::DemoWindows,
}

impl epi::App for DemoApp {
    fn name(&self) -> &str {
        "✨ Demos"
    }

    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default();
        }
    }

    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        self.demo_windows.ui(ctx);
    }
}
