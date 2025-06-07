use egui::{Align2, AreaPosition, Vec2, Vec2b};

#[derive(Default)]
pub struct WindowPositionTest {
    open_with_offset: bool,
    standard_window: bool,
    opens_in_center: bool,
}

impl crate::Demo for WindowPositionTest {
    fn name(&self) -> &'static str {
        "Window Position Test"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        use egui::Window;

        let main_window = Window::new("Window/Area position demo and test")
            .resizable(false)
            .open(open)
            .auto_sized()
            .anchor(Align2::LEFT_TOP, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Areas have a big varity of ways to be positioned.");
                ui.label("And since Windows are based on Areas, this applies to them as well.");
                ui.label("Here are a bunch of examples on what can be done.");
                ui.checkbox(&mut self.standard_window, "A standard window");
                ui.checkbox(
                    &mut self.opens_in_center,
                    "A window that always opens in the center of the screen",
                );
                ui.checkbox(&mut self.open_with_offset, "Anchored, with offset");
            });

        self.standard_window &= *open;
        Window::new("Standard window")
            .open(&mut self.standard_window)
            .show(ctx, |ui| {
                ui.label("This is just a standard window");
            });

        self.opens_in_center &= *open;
        Window::new("Opens centered")
            .open(&mut self.opens_in_center)
            .position(AreaPosition::moveable_centered(true))
            .show(ctx, |ui| {
                ui.label("This is almost a standard window");
                ui.label("It differs in that it will always open in the center, centered");
            });

        let offset = Vec2::Y
            * main_window
                .map(|resp| resp.response.rect.height())
                .unwrap_or_default()
            + ctx.style().spacing.item_spacing;
        self.open_with_offset &= *open;
        Window::new("Anchored Top Left, but with an offset")
            .open(&mut self.open_with_offset)
            .min_size([125.0, 125.0])
            .resizable(Vec2b::TRUE)
            .anchor(Align2::LEFT_TOP, offset)
            .show(ctx, |ui| {
                ui.label("This window is still anchored top left.");
                ui.label("It is also offset, and you can resize it.");
            });
    }
}
