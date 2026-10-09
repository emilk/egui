use egui::Vec2;
use egui_kittest::Harness;

#[test]
fn context_setup_applies_to_the_first_pass() {
    let spacing = Vec2::splat(42.0);
    let mut first_pass_spacing = None;

    let harness = Harness::builder()
        .with_context_setup(move |ctx| {
            ctx.all_styles_mut(|style| style.spacing.item_spacing = spacing);
        })
        .build_ui(|ui| {
            first_pass_spacing.get_or_insert_with(|| ui.spacing().item_spacing);
        });
    drop(harness);

    assert_eq!(first_pass_spacing, Some(spacing));
}

#[test]
fn harness_test_settings_override_context_setup() {
    let harness = Harness::builder()
        .with_context_setup(|ctx| {
            ctx.all_styles_mut(|style| {
                style.animation_time = 1.0;
                style.visuals.text_cursor.blink = true;
            });
        })
        .build_ui(|_ui| {});

    let style = harness.ctx.global_style();
    assert_eq!(style.animation_time, 0.0);
    assert!(!style.visuals.text_cursor.blink);
}
