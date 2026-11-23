use egui::Shape;
use egui_kittest::Harness;

fn shows_perf_warning<State>(harness: &Harness<'_, State>) -> bool {
    harness.output().shapes.iter().any(|clipped| {
        matches!(&clipped.shape, Shape::Text(text) if text.galley.text().contains("PERF WARNING"))
    })
}

fn discard_harness<'a>() -> Harness<'a, bool> {
    Harness::new_ui_state(
        |ui, discard: &mut bool| {
            if *discard {
                ui.ctx().request_discard("test");
            }
            ui.label("Hello");
        },
        true,
    )
}

#[test]
fn harness_hides_the_multipass_warning() {
    let mut harness = discard_harness();
    harness.run_steps(3);
    assert!(!shows_perf_warning(&harness));

    *harness.state_mut() = false;
    harness.step();
    assert!(!shows_perf_warning(&harness));
}

#[cfg(debug_assertions)]
#[test]
fn multipass_warning_can_be_enabled() {
    let mut harness = discard_harness();
    harness
        .ctx
        .all_styles_mut(|style| style.debug.warn_if_multipass = true);
    harness.run_steps(3);
    assert!(shows_perf_warning(&harness));
}
