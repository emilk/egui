
use egui::{Context, Id, Popup, Pos2, RawInput, LayerId};

#[test]
fn test_popup_with_builder() {
    let ctx = Context::default();
    let _ = ctx.run(RawInput::default(), |ctx| {
        let popup = Popup::new(
            Id::new("my_popup"),
            ctx.clone(),
            Pos2::new(0.0, 0.0),
            LayerId::background(),
        );

        let response = popup.show_with(|ui| {
            ui.label("Hello from builder!");
        });

        assert!(response.is_some());
        let response = response.unwrap();
        assert!(response.response.rect.width() > 0.0);
        assert!(response.response.rect.height() > 0.0);
    });
}
