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
        .all_styles_mut(|style| style.debug.consecutive_multipass_warning_cutoff = Some(3));
    harness.run_steps(3);
    assert!(shows_perf_warning(&harness));
}

#[test]
fn run_does_not_stop_on_a_multipass_frame() {
    let mut harness = Harness::new_ui_state(
        |ui, discards_left: &mut u32| {
            if 0 < *discards_left {
                *discards_left -= 1;
                ui.ctx().request_discard("test");
            }
            ui.label("Hello");
        },
        0,
    );

    // Each multi-pass frame is two passes:
    *harness.state_mut() = 6;

    let steps = harness.run();
    assert_eq!(steps, 4);
    assert_eq!(harness.output().platform_output.num_completed_passes, 1);
    assert_eq!(*harness.state(), 0);
}

#[test]
fn endless_multipass_exceeds_max_steps() {
    let mut harness = Harness::builder().with_max_steps(4).build_ui(|ui| {
        ui.ctx().request_discard("test");
        ui.label("Hello");
    });

    let err = harness
        .try_run()
        .expect_err("A ui that always discards never settles");
    assert!(err.multipass);
    assert!(err.to_string().contains("request_discard"), "{err}");
}
