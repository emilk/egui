//! Accessibility regressions for explicitly invisible UI.
//!
//! Hidden widgets must be omitted without leaving dangling relationships or focus.
//! Disabling, clipping, sizing, and requesting a discard alone must not hide widgets.

use std::collections::{HashMap, HashSet};

use egui::{
    UiBuilder,
    accesskit::{Role, TreeUpdate},
};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};

/// At the pass limit, hidden sizing content and `add_visible(false)` stay excluded.
/// A pending discard must neither hide visible content nor make `add_visible` permanent.
#[test]
fn discard_at_pass_limit_excludes_only_explicitly_hidden_widgets() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    ctx.options_mut(|options| options.max_passes = 1.try_into().unwrap());

    let mut passes = 0;
    let output = ctx.run_ui(Default::default(), |ui| {
        passes += 1;
        ui.label("visible before discard");

        ui.scope_builder(UiBuilder::new().sizing_pass().invisible(), |ui| {
            ui.label("hidden sizing label");
            ui.ctx().request_discard("Full sizing");
        });

        ui.add_visible(false, egui::Button::new("hidden button"));
        ui.label("visible after discard");
    });

    let tree = validated_tree(output);
    assert_eq!(passes, 1);

    // Labels expose their text as a value; buttons use an accessible label.
    let texts: Vec<_> = tree
        .nodes
        .iter()
        .filter_map(|(_, node)| node.label().or_else(|| node.value()))
        .collect();
    assert!(!texts.contains(&"hidden sizing label"));
    assert!(!texts.contains(&"hidden button"));
    assert!(texts.contains(&"visible before discard"));
    assert!(texts.contains(&"visible after discard"));
}

/// Invisibility propagates through nested layouts and all node-creation paths,
/// including built-in widgets, direct metadata updates, and output events.
#[test]
fn nested_hidden_widgets_and_direct_node_creation() {
    let tree = tree_for(|ui| {
        ui.scope_builder(UiBuilder::new().invisible(), |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("hidden nested label");
                    let _ = ui.button("hidden button");
                    ui.text_edit_singleline(&mut "hidden text".to_owned());
                    ui.add(egui::Slider::new(&mut 0.5, 0.0..=1.0).text("hidden slider"));

                    // Register a focusable widget without going through widget_info.
                    let response =
                        ui.allocate_response(egui::vec2(20.0, 20.0), egui::Sense::click());

                    let node_result = ui.ctx().accesskit_node_builder(response.id, |node| {
                        node.set_label("hidden direct node");
                    });
                    assert!(node_result.is_none());

                    response.output_event(egui::output::OutputEvent::Clicked(
                        egui::WidgetInfo::labeled(Role::Button, false, "hidden event"),
                    ));
                });
            });
        });
    });

    // No hidden containers or descendants, not just no matching labels.
    assert_eq!(tree.nodes.len(), 2);
}

/// A hidden duplicate must not make querying the visible label ambiguous.
#[test]
fn visible_and_hidden_matching_labels() {
    let harness = Harness::new_ui(|ui| {
        ui.label("metrics-row");
        ui.scope_builder(UiBuilder::new().invisible(), |ui| {
            ui.label("metrics-row");
        });
    });

    // This query panics if more than one matching node is exposed.
    harness.get_by_label("metrics-row");
}

/// Disabled, sizing-only, and fully clipped widgets are not explicitly invisible.
/// They remain accessible, with the disabled widget retaining its disabled state.
#[test]
fn disabled_visible_sizing_and_clipped_widgets_remain_accessible() {
    let harness = Harness::new_ui(|ui| {
        ui.add_enabled(false, egui::Button::new("disabled button"));

        ui.scope_builder(UiBuilder::new().sizing_pass(), |ui| {
            ui.label("visible sizing label");
        });

        ui.scope(|ui| {
            ui.set_clip_rect(egui::Rect::NOTHING);
            let _ = ui.button("clipped button");
        });
    });

    let disabled_button = harness.get_by_label("disabled button");
    assert!(disabled_button.accesskit_node().is_disabled());
    harness.get_by_label("visible sizing label");
    harness.get_by_label("clipped button");
}

/// Hiding a focused subtree removes its nodes; showing it again restores them.
#[test]
fn visible_hidden_visible_transition() {
    let mut harness = Harness::new_ui_state(
        |ui, visible| {
            let builder = if *visible {
                UiBuilder::new()
            } else {
                UiBuilder::new().invisible()
            };

            ui.scope_builder(builder, |ui| {
                ui.label("changing label");
                ui.text_edit_singleline(&mut "changing text".to_owned());
            });
        },
        true,
    );

    // Start with visible content and focus the text input.
    harness.get_by_label("changing label");
    harness.get_by_role(Role::TextInput).focus();
    harness.run();

    // Hide the subtree, including the previously focused input.
    *harness.state_mut() = false;
    harness.run();

    assert!(harness.query_by_label("changing label").is_none());
    assert!(harness.query_by_role(Role::TextInput).is_none());

    // Exclusion must not persist into a later visible frame.
    *harness.state_mut() = true;
    harness.run();

    harness.get_by_label("changing label");
    harness.get_by_role(Role::TextInput);
}

/// Calling `set_invisible` only hides subsequent widgets, not earlier content
/// or metadata subsequently attached to an earlier response.
#[test]
fn mid_ui_invisibility_preserves_earlier_content() {
    let harness = Harness::new_ui(|ui| {
        ui.label("earlier label");
        let earlier = ui.button("earlier button");

        ui.set_invisible();
        ui.label("later label");
        ui.vertical(|ui| {
            let _ = ui.button("later nested button");
        });

        // Metadata for an earlier response still belongs to visible content.
        earlier
            .ctx
            .accesskit_node_builder(earlier.id, |node| node.set_label("earlier button"));
    });

    harness.get_by_label("earlier label");
    harness.get_by_label("earlier button");
    assert!(harness.query_by_label("later label").is_none());
    assert!(harness.query_by_label("later nested button").is_none());
}

/// Hidden root UIs are excluded even when a custom node is created before
/// its widget registers that it belongs to the hidden UI.
#[test]
fn invisible_top_level_ui_and_late_registration() {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();

    let output = ctx.run_pass(Default::default(), |ctx| {
        let mut ui = egui::Ui::new(
            ctx.clone(),
            egui::Id::unique("hidden root"),
            UiBuilder::new().invisible().sense(egui::Sense::click()),
        );

        let id = ui.make_persistent_id("late registration");
        ctx.accesskit_node_builder(id, |node| node.set_label("created before registration"));
        ui.interact(egui::Rect::NOTHING, id, egui::Sense::click());
        ui.label("hidden root label");
    });
    let tree = validated_tree(output);

    // run_pass also constructs the ordinary root Ui for plugins.
    assert_eq!(tree.nodes.len(), 2);
}

/// The `Response::labelled_by` API must not leave a reference to an excluded label.
#[test]
fn hidden_label_reference_is_removed() {
    let tree = tree_for(|ui| {
        let button = ui.button("visible button");
        ui.set_invisible();
        let label = ui.label("hidden label");
        button.labelled_by(label.id);
    });

    let (_, button) = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button)
        .expect("visible button");

    assert!(button.labelled_by().is_empty());
}

/// Custom-widget relationships drop hidden targets but preserve visible ones.
/// Exercise every list-valued and single-target AccessKit relationship.
#[test]
fn relationships_to_hidden_nodes_are_removed() {
    for hidden_single_target in [false, true] {
        let mut visible_id = None;

        let tree = tree_for(|ui| {
            let controller = ui.button("controller");
            let visible = ui.label("visible target").id.accesskit_id();
            let hidden = ui
                .add_visible(false, egui::Label::new("hidden target"))
                .id
                .accesskit_id();
            visible_id = Some(visible);

            ui.ctx().accesskit_node_builder(controller.id, |node| {
                // Lists contain both kinds of target, so pruning must be selective.
                let targets = [hidden, visible];
                node.set_controls(targets);
                node.set_details(targets);
                node.set_described_by(targets);
                node.set_flow_to(targets);
                node.set_labelled_by(targets);
                node.set_owns(targets);
                node.set_radio_group(targets);

                // Test both removal and preservation for single-target properties.
                let target = if hidden_single_target {
                    hidden
                } else {
                    visible
                };
                node.set_active_descendant(target);
                node.set_error_message(target);
                node.set_in_page_link_target(target);
                node.set_member_of(target);
                node.set_next_on_line(target);
                node.set_previous_on_line(target);
                node.set_popup_for(target);
            });
        });

        let visible_id = visible_id.expect("visible target was created");
        let (_, controller) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("controller"))
            .expect("visible controller");

        for targets in [
            controller.controls(),
            controller.details(),
            controller.described_by(),
            controller.flow_to(),
            controller.labelled_by(),
            controller.owns(),
            controller.radio_group(),
        ] {
            assert_eq!(targets, &[visible_id]);
        }

        let expected_target = if hidden_single_target {
            None
        } else {
            Some(visible_id)
        };

        for target in [
            controller.active_descendant(),
            controller.error_message(),
            controller.in_page_link_target(),
            controller.member_of(),
            controller.next_on_line(),
            controller.previous_on_line(),
            controller.popup_for(),
        ] {
            assert_eq!(target, expected_target);
        }
    }
}

/// Clear a text selection if either endpoint is excluded; preserve it otherwise.
/// These synthetic nodes test reference integrity, not text navigation semantics.
#[test]
fn text_selection_requires_both_endpoints_to_remain_visible() {
    for hidden_anchor in [false, true] {
        for hidden_focus in [false, true] {
            let mut owner_id = None;

            let tree = tree_for(|ui| {
                let owner = ui.label("selection owner");
                owner_id = Some(owner.id.accesskit_id());
                let visible = ui.label("visible text").id.accesskit_id();
                let hidden = ui
                    .add_visible(false, egui::Label::new("hidden text"))
                    .id
                    .accesskit_id();

                ui.ctx().accesskit_node_builder(owner.id, |node| {
                    node.set_text_selection(egui::accesskit::TextSelection {
                        anchor: egui::accesskit::TextPosition {
                            node: if hidden_anchor { hidden } else { visible },
                            character_index: 0,
                        },
                        focus: egui::accesskit::TextPosition {
                            node: if hidden_focus { hidden } else { visible },
                            character_index: 1,
                        },
                    });
                });
            });

            let (_, owner) = tree
                .nodes
                .iter()
                .find(|(id, _)| Some(*id) == owner_id)
                .expect("visible selection owner");

            assert_eq!(
                owner.text_selection().is_some(),
                !hidden_anchor && !hidden_focus
            );
        }
    }
}

/// Build one frame and validate the raw update before returning it for assertions.
fn tree_for(build: impl FnMut(&mut egui::Ui)) -> TreeUpdate {
    let ctx = egui::Context::default();
    ctx.enable_accesskit();
    validated_tree(ctx.run_ui(Default::default(), build))
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
