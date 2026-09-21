//! Snapshot tests for [`Aligned`], which places contents of unknown size at an [`Align2`]
//! within the available space of the parent [`egui::Ui`].
//!
//! [`Aligned`] measures its contents on the first pass (adding them invisibly and requesting a
//! discard), then places them on the next pass using the remembered size. That means a single
//! `Harness::run` should already produce the final, correctly placed layout, with no flicker.

use core::cell::Cell;
use std::rc::Rc;

use egui::{
    Align2, Aligned, Color32, Frame, Rect, Stroke, StrokeKind, TextWrapMode, UiBuilder, Vec2, pos2,
};
use egui_kittest::{Harness, SnapshotResults};

/// Outline of each cell, i.e. the available space the [`Aligned`] is aligning within.
const CELL_OUTLINE: Color32 = Color32::from_rgb(255, 0, 255);

/// Background of the aligned contents, so their extent is unmistakable.
const CONTENT_FILL: Color32 = Color32::from_rgb(20, 60, 120);

const ALIGNMENTS: [(&str, Align2); 9] = [
    ("left_top", Align2::LEFT_TOP),
    ("center_top", Align2::CENTER_TOP),
    ("right_top", Align2::RIGHT_TOP),
    ("left_center", Align2::LEFT_CENTER),
    ("center_center", Align2::CENTER_CENTER),
    ("right_center", Align2::RIGHT_CENTER),
    ("left_bottom", Align2::LEFT_BOTTOM),
    ("center_bottom", Align2::CENTER_BOTTOM),
    ("right_bottom", Align2::RIGHT_BOTTOM),
];

/// Show `add_contents` inside a framed box, so the snapshot shows exactly what got aligned.
fn framed_contents(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    Frame::new()
        .fill(CONTENT_FILL)
        .inner_margin(4)
        .show(ui, add_contents);
}

/// All nine [`Align2`] variants in a 3×3 grid of equally sized cells.
///
/// Each cell outlines its available space, and the aligned contents should sit flush against
/// the corresponding edge(s) of that outline (or be centered between them).
#[test]
fn aligned_in_all_nine_positions() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(480.0, 240.0))
        .build_ui(|ui| {
            let full_rect = ui.available_rect_before_wrap();
            let cell_size = full_rect.size() / 3.0;

            for (i, (name, align2)) in ALIGNMENTS.iter().enumerate() {
                let (col, row) = (i % 3, i / 3);
                let cell_rect = Rect::from_min_size(
                    full_rect.min + cell_size * Vec2::new(col as f32, row as f32),
                    cell_size,
                );

                ui.painter().rect_stroke(
                    cell_rect,
                    0,
                    Stroke::new(1.0, CELL_OUTLINE),
                    StrokeKind::Inside,
                );

                // Each cell is its own `Ui`, so the `Aligned`s get distinct ids without
                // needing an explicit `id_salt`:
                ui.scope_builder(
                    UiBuilder::new()
                        .id_salt(name)
                        .max_rect(cell_rect.shrink(1.0)),
                    |ui| {
                        Aligned::new(*align2).show(ui, |ui| {
                            framed_contents(ui, |ui| {
                                ui.label(*name);
                            });
                        });
                    },
                );
            }
        });

    harness.run();
    harness.snapshot("aligned/all_nine_positions");
}

/// Several [`Aligned`] in the _same_ [`egui::Ui`], distinguished by `id_salt`.
///
/// If the ids clashed, the containers would share their remembered size and end up misplaced,
/// and egui would warn about the id clash (which the harness would surface).
#[test]
fn aligned_several_in_same_ui_with_id_salt() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 120.0))
        .build_ui(|ui| {
            let available_rect = ui.available_rect_before_wrap();
            ui.painter().rect_stroke(
                available_rect,
                0,
                Stroke::new(1.0, CELL_OUTLINE),
                StrokeKind::Inside,
            );

            // The parent `Ui` is vertical. The first container is aligned to the top,
            // so the cursor advances past it, and the second one aligns within what is left below.
            Aligned::new(Align2::RIGHT_TOP)
                .id_salt("wide")
                .show(ui, |ui| {
                    framed_contents(ui, |ui| {
                        ui.label("A wide label, top right");
                    });
                });

            Aligned::new(Align2::CENTER_BOTTOM)
                .id_salt("narrow")
                .show(ui, |ui| {
                    framed_contents(ui, |ui| {
                        ui.label("Narrow, bottom");
                    });
                });
        });

    harness.run();
    harness.snapshot("aligned/several_in_same_ui");
}

/// Like a `Panel`, an [`Aligned`] at the _end_ of the parent's main axis reserves only that strip,
/// so widgets added afterwards go in the space _before_ it.
///
/// Here: something in the bottom-right corner of a vertical `Ui`, and then a label,
/// which should end up at the top, above the aligned contents.
#[test]
fn aligned_at_bottom_leaves_space_above() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(240.0, 100.0))
        .build_ui(|ui| {
            let available_before = ui.available_rect_before_wrap();
            ui.painter().rect_stroke(
                available_before,
                0,
                Stroke::new(1.0, CELL_OUTLINE),
                StrokeKind::Inside,
            );

            let response = Aligned::new(Align2::RIGHT_BOTTOM).show(ui, |ui| {
                framed_contents(ui, |ui| {
                    let _ = ui.button("Bottom right");
                });
            });

            let available_after = ui.available_rect_before_wrap();
            let label_response = ui.label("Added after, but placed above");

            if ui.output(|o| !o.requested_discard()) {
                let content_rect = response.response.rect;
                let spacing = ui.spacing().item_spacing.y;

                assert_eq!(
                    available_after.min, available_before.min,
                    "The space above the aligned contents should still be available"
                );
                assert_eq!(
                    available_after.right(),
                    available_before.right(),
                    "The full width should still be available"
                );
                assert!(
                    (available_after.bottom() - (content_rect.top() - spacing)).abs() <= 1.0,
                    "Only the strip at the bottom should be reserved: {available_after:?} vs {content_rect:?}"
                );
                assert!(
                    label_response.rect.bottom() <= content_rect.top(),
                    "The label should be placed above the aligned contents: {:?} vs {content_rect:?}",
                    label_response.rect
                );
            }
        });

    harness.run();
    harness.snapshot("aligned/at_bottom_leaves_space_above");
}

/// Same as [`aligned_at_bottom_leaves_space_above`], but in a horizontal `Ui`:
/// something on the right, and the next widget goes on the left.
#[test]
fn aligned_at_right_of_horizontal_ui_leaves_space_to_the_left() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 60.0))
        .build_ui(|ui| {
            ui.horizontal(|ui| {
                let available_before = ui.available_rect_before_wrap();
                ui.painter().rect_stroke(
                    available_before,
                    0,
                    Stroke::new(1.0, CELL_OUTLINE),
                    StrokeKind::Inside,
                );

                let response = Aligned::new(Align2::RIGHT_CENTER).show(ui, |ui| {
                    framed_contents(ui, |ui| {
                        let _ = ui.button("Right");
                    });
                });

                let available_after = ui.available_rect_before_wrap();
                let label_response = ui.label("Added after, placed left");

                if ui.output(|o| !o.requested_discard()) {
                    let content_rect = response.response.rect;
                    let spacing = ui.spacing().item_spacing.x;

                    assert_eq!(
                        available_after.left(),
                        available_before.left(),
                        "The space to the left of the aligned contents should still be available"
                    );
                    assert!(
                        (available_after.right() - (content_rect.left() - spacing)).abs() <= 1.0,
                        "Only the strip on the right should be reserved: {available_after:?} vs {content_rect:?}"
                    );
                    assert!(
                        label_response.rect.right() <= content_rect.left(),
                        "The label should be placed to the left of the aligned contents: {:?} vs {content_rect:?}",
                        label_response.rect
                    );
                }
            });
        });

    harness.run();
    harness.snapshot("aligned/at_right_leaves_space_to_the_left");
}

/// An [`Aligned`] at the _start_ of the main axis behaves like any other widget:
/// the cursor advances past it.
#[test]
fn aligned_at_top_advances_cursor_past_it() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(240.0, 100.0))
        .build_ui(|ui| {
            let response = Aligned::new(Align2::RIGHT_TOP).show(ui, |ui| {
                framed_contents(ui, |ui| {
                    let _ = ui.button("Top right");
                });
            });
            let label_response = ui.label("Added after, placed below");

            if ui.output(|o| !o.requested_discard()) {
                let content_rect = response.response.rect;
                assert!(
                    content_rect.bottom() <= label_response.rect.top(),
                    "The label should be placed below the aligned contents: {:?} vs {content_rect:?}",
                    label_response.rect
                );
            }
        });

    harness.run();
}

/// Contents that don't fit are clamped to the left/top edge of the available space,
/// rather than overflowing above or to the left of it.
#[test]
fn aligned_oversized_contents_are_clamped_to_left_top() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(160.0, 60.0))
        .build_ui(|ui| {
            let available_rect = ui.available_rect_before_wrap();

            let response = Aligned::new(Align2::RIGHT_BOTTOM).show(ui, |ui| {
                framed_contents(ui, |ui| {
                    // Don't wrap the text to fit, so the contents overflow the available space:
                    ui.style_mut().wrap_mode = Some(TextWrapMode::Extend);
                    ui.label("This label is far too wide to fit in the available space");
                    ui.label("Line two");
                    ui.label("Line three");
                });
            });

            // Painted after the contents, so the outline is visible on top of them:
            ui.painter().rect_stroke(
                available_rect,
                0,
                Stroke::new(1.0, CELL_OUTLINE),
                StrokeKind::Inside,
            );

            let content_rect = response.response.rect;
            assert!(
                content_rect.width() > available_rect.width(),
                "Test setup: contents should be wider than the available space"
            );
            assert!(
                content_rect.height() > available_rect.height(),
                "Test setup: contents should be taller than the available space"
            );
        });

    harness.run();
    harness.snapshot("aligned/oversized_contents_clamped");
}

/// When the contents change size between frames, the container re-aligns them.
///
/// The right edge of the contents must stay flush with the right edge of the available space,
/// both before and after the contents grow.
#[test]
fn aligned_follows_contents_when_they_change_size() {
    let mut results = SnapshotResults::new();

    let long_text = Rc::new(Cell::new(false));

    let mut harness = Harness::builder()
        .with_size(Vec2::new(240.0, 80.0))
        .build_ui({
            let long_text = Rc::clone(&long_text);
            move |ui| {
                let available_rect = ui.available_rect_before_wrap();
                ui.painter().rect_stroke(
                    available_rect,
                    0,
                    Stroke::new(1.0, CELL_OUTLINE),
                    StrokeKind::Inside,
                );

                let response = Aligned::new(Align2::RIGHT_BOTTOM).show(ui, |ui| {
                    framed_contents(ui, |ui| {
                        if long_text.get() {
                            ui.label("Now the label is much longer");
                            ui.label("and has a second line");
                        } else {
                            ui.label("Short");
                        }
                    });
                });

                // Note: on the very first pass the contents are added invisibly at the top-left
                // and a discard is requested, so we only check the placement on subsequent passes:
                if ui.output(|o| !o.requested_discard()) {
                    let content_rect = response.response.rect;
                    assert!(
                        (content_rect.right() - available_rect.right()).abs() <= 1.0,
                        "Contents should be flush with the right edge: {content_rect:?} vs {available_rect:?}"
                    );
                    assert!(
                        (content_rect.bottom() - available_rect.bottom()).abs() <= 1.0,
                        "Contents should be flush with the bottom edge: {content_rect:?} vs {available_rect:?}"
                    );
                }
            }
        });

    harness.run();
    results.add(harness.try_snapshot("aligned/resize_0_short"));

    long_text.set(true);
    harness.run();
    results.add(harness.try_snapshot("aligned/resize_1_long"));

    long_text.set(false);
    harness.run();
    results.add(harness.try_snapshot("aligned/resize_2_short_again"));
}

/// The very first pass should not paint the contents at all (they are measured invisibly),
/// and instead request a discard so that the user never sees a misplaced frame.
#[test]
fn aligned_first_pass_is_invisible_and_requests_discard() {
    let ctx = egui::Context::default();
    ctx.set_fonts(egui::FontDefinitions::empty());
    // Only allow a single pass, so we can inspect the output of the very first one:
    ctx.options_mut(|o| o.max_passes = 1.try_into().unwrap());

    let mut saw_discard_request = false;
    let output = ctx.run_ui(Default::default(), |ui| {
        Aligned::new(Align2::RIGHT_BOTTOM).show(ui, |ui| {
            ui.painter().rect_filled(
                Rect::from_min_size(pos2(0.0, 0.0), Vec2::splat(10.0)),
                0,
                CONTENT_FILL,
            );
            ui.allocate_space(Vec2::splat(10.0));
        });
        saw_discard_request = ui.output(|o| o.requested_discard());
    });

    assert!(
        saw_discard_request,
        "Aligned should request a discard on the first pass, when it doesn't yet know its size"
    );

    let painted_shapes: Vec<_> = output
        .shapes
        .iter()
        .filter(|s| !matches!(s.shape, egui::Shape::Noop))
        .collect();
    assert!(
        painted_shapes.is_empty(),
        "Nothing should be painted on the first pass, since the contents are invisible: {:?}",
        output.shapes
    );

    output.drop_without_applying_deltas();
}
