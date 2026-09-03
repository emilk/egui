use egui::{
    Color32, Vec2,
    accesskit::Role,
    color_picker::{Alpha, color_picker_color32, color_picker_hsva_2d},
    ecolor::Hsva,
    style::NumericColorSpace,
};
use egui_kittest::{Harness, kittest::Queryable as _};

#[test]
fn color_picker() {
    let mut color = Color32::from_rgba_unmultiplied(130, 130, 130, 45);
    let mut harness = Harness::builder()
        .with_pixels_per_point(2.0)
        .build_ui(move |ui| {
            ui.spacing_mut().slider_width = 275.0;
            color_picker_color32(ui, &mut color, Alpha::OnlyBlend);
        });
    harness.run();
    harness.fit_contents();
    harness.snapshot("color_picker");
}

#[test]
fn editing_alpha_does_not_change_rgb() {
    for color_space in [NumericColorSpace::GammaByte, NumericColorSpace::Linear] {
        let color = Hsva::new(0.94, 0.73, 0.82, 0.12);
        let mut harness = Harness::new_ui_state(
            |ui, color| {
                ui.spacing_mut().slider_width = 275.0;
                color_picker_hsva_2d(ui, color, Alpha::OnlyBlend);
            },
            color,
        );
        harness.ctx.all_styles_mut(|style| {
            style.visuals.numeric_color_space = color_space;
        });
        harness.run();

        let rgb_before = harness.state().to_rgb();
        let alpha_before = harness.state().a;
        let drag_values: Vec<_> = harness
            .query_all_by_role(Role::SpinButton)
            .collect();
        assert_eq!(drag_values.len(), 4);
        let grab = drag_values[3].rect().center();
        let target = grab + Vec2::new(20.0, 0.0);

        harness.hover_at(grab);
        harness.run();
        harness.drag_at(grab);
        harness.run();
        harness.hover_at(target);
        harness.run();
        harness.drop_at(target);
        harness.run();

        assert_ne!(harness.state().a, alpha_before);
        assert_eq!(
            harness.state().to_rgb(),
            rgb_before,
            "editing alpha changed RGB in {color_space:?} mode"
        );
    }
}
