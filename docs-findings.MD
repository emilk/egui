# Documentation Findings

1. No FAQ or crate-level walkthroughs for missing widgets/layouts/tooltips in `crates/egui/src/lib.rs`; only widget enum mentions exist and tooltips appear solely in `warn_if_debug_build`.
2. Item-level docs lack runnable examples for `SelectableLabel`, tooltip helpers, etc., leaving the "crate-level AND item-level" requirement from issue #186 unmet.
3. Slider docs lack links to the slider demo and vice versa, so the cross-link tasks remain open (`crates/egui/src/widgets/slider.rs`, `crates/egui_demo_lib/src/demo/sliders.rs`).
4. A doctest is still marked `ignore` (`Ui::menu_image_button`), violating the "all doc examples should run" goal.
5. No GIFs live in the docs themselves; only the changelog embeds old animations, so the "Add a few gifs" checkbox is still unchecked.
