//! Tests for [`egui::CollapsingHeader`].

use egui::{CollapsingHeader, TextWrapMode, Vec2};
use egui_kittest::{Harness, kittest::Queryable as _};

const LONG_TITLE: &str = "Plan (1/3) Find gaps in the camera timestamps across all fifty episodes";

/// A truncated header stays within its parent, and still reports its full title.
#[test]
fn collapsing_header_truncates_long_title() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 60.0))
        .build_ui(|ui| {
            CollapsingHeader::new(LONG_TITLE)
                .wrap_mode(TextWrapMode::Truncate)
                .show(ui, |ui| ui.label("Body"));
        });
    harness.run();

    let header = harness.get_by_label(LONG_TITLE);
    assert!(
        header.rect().right() <= 200.0,
        "header overflows: {:?}",
        header.rect()
    );
    harness.snapshot("collapsing_header_truncated");
}
