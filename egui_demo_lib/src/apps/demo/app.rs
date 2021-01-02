/// Demonstrates how to make an app using Egui.
///
/// Implements `epi::App` so it can be used with
/// [`egui_glium`](https://crates.io/crates/egui_glium) and [`egui_web`](https://crates.io/crates/egui_web).
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct DemoApp {
    demo_windows: super::DemoWindows,
}

impl epi::App for DemoApp {
    fn name(&self) -> &str {
        "✨ Egui Demo"
    }

    fn load(&mut self, storage: &dyn epi::Storage) {
        *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
    }

    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::CtxRef, _frame: &mut epi::Frame<'_>) {
        self.demo_windows.ui(ctx);
    }
}
