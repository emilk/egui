//! Tests the accesskit accessibility output of egui.

use std::collections::{BTreeMap, HashMap, HashSet};

use egui::{
    Label,
    accesskit::{Role, TreeUpdate},
};

/// Check unique IDs, valid focus, and that every node has exactly one path from the root.
fn assert_valid_tree_structure(tree: &TreeUpdate) {
    let nodes: HashMap<_, _> = tree.nodes.iter().map(|(id, node)| (*id, node)).collect();
    assert_eq!(nodes.len(), tree.nodes.len(), "duplicate IDs");
    assert!(nodes.contains_key(&tree.focus), "dangling focus");

    let mut visited = HashSet::new();
    let mut pending = vec![tree.tree.as_ref().expect("tree metadata").root];

    while let Some(id) = pending.pop() {
        assert!(visited.insert(id), "cycle or multiple parents");
        let node = nodes.get(&id).expect("dangling child");
        pending.extend(node.children());
    }

    assert_eq!(visited.len(), nodes.len(), "unreachable nodes");
}

/// Extract the tree without leaving unapplied texture deltas on the frame output.
fn validated_tree(mut output: egui::FullOutput) -> TreeUpdate {
    output.textures_delta.clear();

    let tree = output
        .platform_output
        .accesskit_update
        .expect("AccessKit update");
    assert_valid_tree_structure(&tree);

    tree
}

fn count_labels(tree: &TreeUpdate) -> BTreeMap<String, usize> {
    let mut label_counts: BTreeMap<String, usize> = Default::default();

    for label in tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() != Role::TextRun) // Text runs repeat the text of their parent
        .filter_map(|(_, node)| node.label().or_else(|| node.value()))
    {
        *label_counts.entry(label.to_owned()).or_default() += 1;
    }
    label_counts
}

#[track_caller]
fn check_node_counts(output: egui::FullOutput, expected_node_counts: &[(&str, usize)]) {
    let expected: BTreeMap<String, usize> = expected_node_counts
        .iter()
        .map(|&(label, count)| (label.to_owned(), count))
        .collect();
    assert_eq!(
        count_labels(&validated_tree(output)),
        expected,
        "Unexpected accessibility node labels"
    );
}

/// When calling `request_discard`, no nodes from the discarded pass should show up in the widget tree.
#[test]
fn discarded_pass_discards_nodes() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    ctx.options_mut(|options| options.max_passes = 2.try_into().unwrap());

    let mut num_passes = 0;

    let output = ctx.run_ui(Default::default(), |ui| {
        num_passes += 1;

        ui.label("always visible");

        if ui.current_pass_index() == 0 {
            ui.label("only visible in discarded pass");
            ui.request_discard("test");
        } else {
            ui.label("only visible in final pass");
        }
    });

    assert_eq!(num_passes, 2);

    check_node_counts(
        output,
        &[("always visible", 1), ("only visible in final pass", 1)],
    );
}

/// Don't expose invisible widgets to accessibility.
#[test]
fn hide_invisible_nodes() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();

    let output = ctx.run_ui(Default::default(), |ui| {
        ui.add_visible(true, Label::new("always visible"));
        ui.add_visible(false, Label::new("never visible"));
    });

    check_node_counts(output, &[("always visible", 1)]);
}

/// Forget nodes from previous frames
#[test]
fn forget_nodes_from_previous_frames() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();

    let output = ctx.run_ui(Default::default(), |ui| {
        ui.label("always visible");
        ui.label("only visible in first frame");
    });
    check_node_counts(
        output,
        &[("always visible", 1), ("only visible in first frame", 1)],
    );

    let output = ctx.run_ui(Default::default(), |ui| {
        ui.label("always visible");
    });
    check_node_counts(output, &[("always visible", 1)]);
}

#[test]
fn forget_nodes_from_sizing_frame() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    ctx.options_mut(|options| options.max_passes = 1.try_into().unwrap());

    let output = ctx.run_ui(Default::default(), |ui| {
        egui::Area::new(ui.make_persistent_id("area"))
            .accessible_name("My area")
            .show(ui, |ui| {
                ui.label("always visible");
                ui.label("only visible in first frame");
            });
    });

    // The first time an Area shows up, it is doing a sizing pass, and should not show up in accessibility tree
    check_node_counts(output, &[]);

    let output = ctx.run_ui(Default::default(), |ui| {
        egui::Area::new(ui.make_persistent_id("area"))
            .accessible_name("My area")
            .show(ui, |ui| {
                ui.label("always visible");
                ui.label("only visible in second frame");
            });
    });

    check_node_counts(
        output,
        &[
            ("My area", 1),
            ("always visible", 1),
            ("only visible in second frame", 1),
        ],
    );
}

/// Forget nodes from sizing pass of Area
#[test]
fn forget_nodes_from_sizing_passes() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    ctx.options_mut(|options| options.max_passes = 2.try_into().unwrap());

    let mut num_passes = 0;

    let output = ctx.run_ui(Default::default(), |ui| {
        num_passes += 1;

        egui::Area::new(ui.make_persistent_id("area"))
            .accessible_name("My area")
            .show(ui, |ui| {
                ui.label("always visible");

                if ui.current_pass_index() == 0 {
                    ui.label("only visible in discarded pass");
                    ui.request_discard("test");
                } else {
                    ui.label("only visible in second pass");
                }
            });
    });

    assert_eq!(num_passes, 2);

    check_node_counts(
        output,
        &[
            ("My area", 1),
            ("always visible", 1),
            ("only visible in second pass", 1),
        ],
    );
}

/// Requesting focus for an invisible widget must not point the accessibility focus at a node that
/// isn't in the tree, but the focus should still be applied once the widget becomes visible.
#[test]
fn request_focus_during_sizing_pass() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    ctx.options_mut(|options| options.max_passes = 1.try_into().unwrap());

    let mut text = String::new();
    let mut run_ui = |ui: &mut egui::Ui, request_focus: bool| {
        egui::Area::new(ui.make_persistent_id("area")).show(ui, |ui| {
            let response = ui.text_edit_singleline(&mut text);
            if request_focus {
                response.request_focus();
            }
        });
    };

    // The first time an Area shows up it does an invisible sizing pass:
    let tree = validated_tree(ctx.run_ui(Default::default(), |ui| run_ui(ui, true)));
    assert_eq!(
        Some(tree.focus),
        tree.tree.as_ref().map(|tree| tree.root),
        "An invisible widget should not have accessibility focus"
    );

    let tree = validated_tree(ctx.run_ui(Default::default(), |ui| run_ui(ui, false)));
    let focused = tree
        .nodes
        .iter()
        .find(|(id, _)| *id == tree.focus)
        .map(|(_, node)| node.role());
    assert_eq!(
        focused,
        Some(Role::TextInput),
        "The focus request should apply once the widget is visible"
    );
}
