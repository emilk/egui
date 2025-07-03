use egui::{TextWrapMode, Vec2, containers::Sides};
use egui_kittest::{Harness, SnapshotResults};

#[test]
fn sides_container_tests() {
    let mut results = SnapshotResults::new();

    test_variants("default", |sides| sides, &mut results);

    test_variants(
        "shrink_left",
        |sides| sides.shrink_left().truncate(),
        &mut results,
    );

    test_variants(
        "shrink_right",
        |sides| sides.shrink_right().truncate(),
        &mut results,
    );

    test_variants(
        "wrap_left",
        |sides| sides.shrink_left().wrap_mode(TextWrapMode::Wrap),
        &mut results,
    );

    test_variants(
        "wrap_right",
        |sides| sides.shrink_right().wrap_mode(TextWrapMode::Wrap),
        &mut results,
    );
}

fn test_variants(
    name: &str,
    mut create_sides: impl FnMut(Sides) -> Sides,
    results: &mut SnapshotResults,
) {
    for (variant_name, left_text, right_text, fit_contents) in [
        ("short", "Left", "Right", false),
        (
            "long",
            "Very long left content that should not fit.",
            "Very long right text that should also not fit.",
            false,
        ),
        ("short_fit_contents", "Left", "Right", true),
        (
            "long_fit_contents",
            "Very long left content that should not fit.",
            "Very long right text that should also not fit.",
            true,
        ),
    ] {
        let mut harness = Harness::builder()
            .with_size(Vec2::new(400.0, 50.0))
            .build_ui(|ui| {
                create_sides(Sides::new()).show(
                    ui,
                    |left| {
                        left.label(left_text);
                    },
                    |right| {
                        right.label(right_text);
                    },
                );
            });

        if fit_contents {
            harness.fit_contents();
        }

        results.add(harness.try_snapshot(&format!("sides/{name}_{variant_name}")));
    }
}
