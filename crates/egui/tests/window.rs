use egui::{Context, RawInput, Window};

#[test]
fn test_window_with_builder() {
    let ctx = Context::default();
    let _ = ctx.run(RawInput::default(), |ctx| {
        let window = Window::new("test_window");
        let response_opt = window.show_with(ctx, |ui| {
            ui.label("Hello from builder!");
        });
        assert!(response_opt.is_some());
        let response = response_opt.unwrap();
        assert!(response.response.rect.width() > 0.0);
    });
}
