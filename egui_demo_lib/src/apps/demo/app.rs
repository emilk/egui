#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DemoApp {
    demo_windows: super::DemoWindows,
}

impl epi::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut epi::Frame) {
        self.demo_windows.ui(ctx);
    }
}
