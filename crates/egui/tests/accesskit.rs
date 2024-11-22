//! Tests the accesskit accessibility output of egui.
#![cfg(feature = "accesskit")]

use accesskit::{NodeId, Role, TreeUpdate};
use egui::{CentralPanel, Context, RawInput, Window};

/// Baseline test that asserts there are no spurious nodes in the
/// accesskit output when the ui is empty.
///
/// This gives reasonable certainty that any nodes appearing in the other accesskit outputs
/// are put there because of the widgets rendered.
#[test]
fn empty_ui_should_return_tree_with_only_root_window() {
    let output = accesskit_output_single_egui_frame(|ctx| {
        CentralPanel::default().show(ctx, |_| {});
    });

    assert_eq!(
        output.nodes.len(),
        1,
        "Empty ui should produce only the root window."
    );
    let (id, root) = &output.nodes[0];

    assert_eq!(*id, output.tree.unwrap().root);
    assert_eq!(root.role(), Role::Window);
}

#[test]
fn button_node() {
    let button_text = "This is a test button!";

    let output = accesskit_output_single_egui_frame(|ctx| {
        CentralPanel::default().show(ctx, |ui| ui.button(button_text));
    });

    assert_eq!(
        output.nodes.len(),
        2,
        "Expected only the root node and the button."
    );

    let (_, button) = output
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button)
        .expect("Button should exist in the accesskit output");

    assert_eq!(button.name(), Some(button_text));
    assert!(!button.is_disabled());
}

#[test]
fn disabled_button_node() {
    let button_text = "This is a test button!";

    let output = accesskit_output_single_egui_frame(|ctx| {
        CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled(false, egui::Button::new(button_text))
        });
    });

    assert_eq!(
        output.nodes.len(),
        2,
        "Expected only the root node and the button."
    );

    let (_, button) = output
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button)
        .expect("Button should exist in the accesskit output");

    assert_eq!(button.name(), Some(button_text));
    assert!(button.is_disabled());
}

#[test]
fn toggle_button_node() {
    let button_text = "A toggle button";

    let mut selected = false;
    let output = accesskit_output_single_egui_frame(|ctx| {
        CentralPanel::default().show(ctx, |ui| ui.toggle_value(&mut selected, button_text));
    });

    assert_eq!(
        output.nodes.len(),
        2,
        "Expected only the root node and the button."
    );

    let (_, toggle) = output
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button)
        .expect("Toggle button should exist in the accesskit output");

    assert_eq!(toggle.name(), Some(button_text));
    assert!(!toggle.is_disabled());
}

#[test]
fn multiple_disabled_widgets() {
    let output = accesskit_output_single_egui_frame(|ctx| {
        CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(false, |ui| {
                let _ = ui.button("Button 1");
                let _ = ui.button("Button 2");
                let _ = ui.button("Button 3");
            })
        });
    });

    assert_eq!(
        output.nodes.len(),
        4,
        "Expected the root node and all the child widgets."
    );

    assert_eq!(
        output
            .nodes
            .iter()
            .filter(|(_, node)| node.is_disabled())
            .count(),
        3,
        "All widgets should be disabled."
    );
}

#[test]
fn window_children() {
    let output = accesskit_output_single_egui_frame(|ctx| {
        let mut open = true;
        Window::new("test window")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                let _ = ui.button("A button");
            });
    });

    let root = output.tree.as_ref().map(|tree| tree.root).unwrap();

    let window_id = assert_window_exists(&output, "test window", root);
    assert_button_exists(&output, "A button", window_id);
    assert_button_exists(&output, "Close window", window_id);
    assert_button_exists(&output, "Hide", window_id);
}

fn accesskit_output_single_egui_frame(run_ui: impl FnMut(&Context)) -> TreeUpdate {
    let ctx = Context::default();
    // Disable animations, so we do not need to wait for animations to end to see the result.
    ctx.style_mut(|style| style.animation_time = 0.0);
    ctx.enable_accesskit();

    let output = ctx.run(RawInput::default(), run_ui);

    output
        .platform_output
        .accesskit_update
        .expect("Missing accesskit update")
}

#[track_caller]
fn assert_button_exists(tree: &TreeUpdate, name: &str, parent: NodeId) {
    let (node_id, _) = tree
        .nodes
        .iter()
        .find(|(_, node)| {
            !node.is_hidden() && node.role() == Role::Button && node.name() == Some(name)
        })
        .expect("No visible button with that name exists.");

    assert_parent_child(tree, parent, *node_id);
}

#[track_caller]
fn assert_window_exists(tree: &TreeUpdate, title: &str, parent: NodeId) -> NodeId {
    let (node_id, _) = tree
        .nodes
        .iter()
        .find(|(_, node)| {
            !node.is_hidden() && node.role() == Role::Window && node.name() == Some(title)
        })
        .expect("No visible window with that title exists.");

    assert_parent_child(tree, parent, *node_id);

    *node_id
}

#[track_caller]
fn assert_parent_child(tree: &TreeUpdate, parent: NodeId, child: NodeId) {
    let (_, parent) = tree
        .nodes
        .iter()
        .find(|(id, _)| id == &parent)
        .expect("Parent does not exist.");

    assert!(
        parent.children().contains(&child),
        "Node is not a child of the given parent."
    );
}
