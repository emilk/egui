use egui::{Context, RawInput, CentralPanel};
use egui_extras::{StripBuilder, Size};

#[test]
fn test_stripbuilder_new_with() {
    let ctx = Context::default();
    let _ = ctx.run(RawInput::default(), |ctx| {
        CentralPanel::default().show(ctx, |ui| {
            let resp = StripBuilder::new_with(ui, |sb| {
                sb.size(Size::remainder())
                    .horizontal(|mut strip| {
                        strip.cell(|ui| {
                            ui.label("hello");
                        });
                    })
            });
            assert!(resp.rect.width() > 0.0);
        });
    });
}
