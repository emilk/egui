//! Snapshot tests for [`egui_extras::StripBuilder`].
//!
//! `Strip` shares [`egui_extras::StripLayout`] with `Table`,
//! so these guard the changes that are really aimed at `Table`.

use egui::{Theme, Ui, Vec2};
use egui_extras::{Size, StripBuilder};
use egui_kittest::Harness;

fn snapshot(name: &str, size: [f32; 2], add_contents: impl FnMut(&mut Ui)) {
    let mut harness = Harness::builder()
        .with_size(Vec2::from(size))
        .with_theme(Theme::Dark)
        .build_ui(add_contents);
    harness.run();
    harness.snapshot(format!("strip/{name}"));
}

/// A cell that paints its own background, so that the cell sizes are visible.
fn filled_cell(ui: &mut Ui, text: &str) {
    ui.painter().rect_filled(
        ui.max_rect(),
        egui::CornerRadius::ZERO,
        ui.visuals().widgets.inactive.bg_fill,
    );
    ui.label(text);
}

/// All the [`Size`] kinds in one horizontal strip.
#[test]
fn size_kinds() {
    snapshot("size_kinds", [400.0, 60.0], |ui| {
        StripBuilder::new(ui)
            .size(Size::exact(60.0))
            .size(Size::initial(40.0))
            .size(Size::relative(0.25))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                for name in ["exact", "initial", "relative", "remainder"] {
                    strip.cell(|ui| {
                        filled_cell(ui, name);
                    });
                }
            });
    });
}

/// A vertical strip of horizontal strips.
#[test]
fn nested_strips() {
    snapshot("nested_strips", [300.0, 140.0], |ui| {
        StripBuilder::new(ui)
            .size(Size::exact(40.0))
            .size(Size::remainder())
            .vertical(|mut strip| {
                strip.cell(|ui| {
                    filled_cell(ui, "top");
                });
                strip.strip(|builder| {
                    builder.sizes(Size::remainder(), 3).horizontal(|mut strip| {
                        for name in ["a", "b", "c"] {
                            strip.cell(|ui| {
                                filled_cell(ui, name);
                            });
                        }
                    });
                });
            });
    });
}

/// An un-filled strip pads itself out with empty cells.
#[test]
fn missing_cells_are_empty() {
    snapshot("missing_cells_are_empty", [300.0, 60.0], |ui| {
        StripBuilder::new(ui)
            .sizes(Size::remainder(), 3)
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    filled_cell(ui, "only one");
                });
            });
    });
}
