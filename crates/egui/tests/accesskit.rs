//! Tests the accesskit accessibility output of egui.

use accesskit::Role;
use egui::{Context, RawInput};

#[test]
fn button_text() {
    let button_text = "This is a test button!";

    let ctx = Context::default();
    ctx.enable_accesskit();

    let output = ctx.run(RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| ui.button(button_text));
    });

    let nodes = output
        .platform_output
        .accesskit_update
        .expect("Missing accesskit update")
        .nodes;
    nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.name() == Some(button_text))
        .expect("Button should exist in the accesskit output");
}
