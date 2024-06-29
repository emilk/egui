//! Tests the accesskit accessibility output of egui.

use accesskit::Role;
use egui::{Context, RawInput};

/// Baseline test that asserts there are no spurious nodes in the
/// accesskit output when the ui is empty.
///
/// This gives reasonable certainty that any nodes appearing in the other accesskit outputs
/// are put there because of the widgets rendered.
#[test]
fn empty_ui_should_return_tree_with_only_root_window() {
    let ctx = Context::default();
    ctx.enable_accesskit();

    let output = ctx.run(RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |_| {});
    });

    let tree_update = output
        .platform_output
        .accesskit_update
        .expect("Missing accesskit update");

    let tree = tree_update.tree.unwrap();

    assert_eq!(
        tree_update.nodes.len(),
        1,
        "Empty ui should produce only the root window."
    );
    let (id, root) = &tree_update.nodes[0];

    assert_eq!(*id, tree.root);
    assert_eq!(root.role(), Role::Window);
}

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

    assert_eq!(
        nodes.len(),
        2,
        "Expected only the root node and the button."
    );

    nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.name() == Some(button_text))
        .expect("Button should exist in the accesskit output");
}

#[test]
fn toggle_button_text() {
    let button_text = "A toggle button";

    let ctx = Context::default();
    ctx.enable_accesskit();

    let mut selected = false;
    let output = ctx.run(RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| ui.toggle_value(&mut selected, button_text));
    });

    let nodes = output
        .platform_output
        .accesskit_update
        .expect("Missing accesskit update")
        .nodes;

    assert_eq!(
        nodes.len(),
        2,
        "Expected only the root node and the button."
    );

    nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button && node.name() == Some(button_text))
        .expect("Toggle button should exist in the accesskit output");
}
