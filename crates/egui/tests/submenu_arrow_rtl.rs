//! The built-in submenu arrow never reaches the screen reader as a '⏵' text glyph.
//!
//! [`egui::containers::menu::SubMenuButton`] attaches its arrow as a painted atom
//! ([`egui::AtomKind::Paint`]), and [`egui::Atoms::text`] only collects
//! [`egui::AtomKind::Text`], so the accesskit label of the button is just its text
//! atoms. This pins that contract, in both reading directions: a future change that
//! turns the arrow into a literal '⏵' text atom (or otherwise leaks it into the
//! label) must fail here loudly.

use egui::containers::menu::SubMenuButton;
use egui::{Context, Pos2, RawInput, Rect, Vec2};

fn input(size: Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        ..Default::default()
    }
}

/// The accesskit labels published by one pass showing a [`SubMenuButton`].
fn button_labels(rtl: bool) -> Vec<String> {
    let ctx = Context::default();
    if rtl {
        ctx.set_right_to_left(true);
    }
    ctx.enable_accesskit();
    // Run one ui pass and discard the texture deltas (these tests never render).
    let mut output = ctx.run_ui(input(Vec2::new(400.0, 200.0)), |ui| {
        SubMenuButton::new("Sub").ui(ui, |ui| {
            let _ = ui.button("Body item");
        });
    });
    output.textures_delta.clear();
    let update = output
        .platform_output
        .accesskit_update
        .expect("the pass should publish an accesskit update");
    update
        .nodes
        .iter()
        .filter_map(|(_, node)| node.label().map(|label| label.to_owned()))
        .collect()
}

#[test]
fn submenu_button_accesskit_label_does_not_contain_the_arrow_glyph() {
    for rtl in [false, true] {
        let labels = button_labels(rtl);
        assert!(
            labels.iter().any(|label| label == "Sub"),
            "the submenu button should be labeled by its text (rtl: {rtl}), got {labels:?}"
        );
        assert!(
            labels.iter().all(|label| !label.contains('⏵')),
            "the painted submenu arrow must not be passed to the screen reader as a \
             '⏵' text glyph (rtl: {rtl}), got {labels:?}"
        );
    }
}
