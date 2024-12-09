use crate::Harness;
use eframe::Frame;
use egui::Context;

impl<'a, State> eframe::App for &mut Harness<'a, State> {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        self.app.run(ctx, &mut self.state, false);
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }
}
