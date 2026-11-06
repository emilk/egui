//! [`egui::ViewportBuilder::layout_direction`] is a chrome hint for the
//! platform integration: it travels to integrations that create native
//! windows and defaults to `None` ("let the integration decide").

use egui::{ViewportBuilder, emath::ReadingDirection};

#[test]
fn layout_direction_defaults_to_none() {
    assert_eq!(ViewportBuilder::default().layout_direction, None);
}

#[test]
fn with_layout_direction_sets_the_hint() {
    let builder = ViewportBuilder::default().with_layout_direction(ReadingDirection::Rtl);
    assert_eq!(builder.layout_direction, Some(ReadingDirection::Rtl));

    let builder = ViewportBuilder::default().with_layout_direction(ReadingDirection::Ltr);
    assert_eq!(builder.layout_direction, Some(ReadingDirection::Ltr));
}

#[test]
fn builders_compare_equal_on_the_hint() {
    assert_eq!(
        ViewportBuilder::default().with_layout_direction(ReadingDirection::Rtl),
        ViewportBuilder::default().with_layout_direction(ReadingDirection::Rtl)
    );
    assert_ne!(
        ViewportBuilder::default().with_layout_direction(ReadingDirection::Rtl),
        ViewportBuilder::default()
    );
}
