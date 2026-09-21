//! Tests the accesskit accessibility output of egui.

use egui::{
    CentralPanel, Context, Panel, RawInput, Ui, Window,
    accesskit::{Action, NodeId, Orientation, Role, TreeUpdate},
    containers::menu::MenuButton,
};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};

/// Baseline test that asserts there are no spurious nodes in the
/// accesskit output when the ui is empty.
///
/// This gives reasonable certainty that any nodes appearing in the other accesskit outputs
/// are put there because of the widgets rendered.
#[test]
fn empty_ui_should_return_tree_with_only_root_window() {
    let output = accesskit_output_single_egui_frame(|_ui| {
        // Nothing here beyond the default empty UI
    });

    assert_eq!(
        output.nodes.len(),
        2,
        "Expected the root node and the top level Ui; found: {output:#?}",
    );

    assert_eq!(
        output
            .nodes
            .iter()
            .filter(|(_, n)| n.role() == Role::GenericContainer)
            .count(),
        1,
        "Expected a single Ui as a GenericContainer node.",
    );

    let (id, root) = &output.nodes[0];

    assert_eq!(*id, output.tree.unwrap().root);
    assert_eq!(root.role(), Role::Window);
}

#[test]
fn button_node() {
    let button_text = "This is a test button!";

    let output = accesskit_output_single_egui_frame(|ui| {
        CentralPanel::default().show(ui, |ui| ui.button(button_text));
    });

    let (_, button) = output
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button)
        .expect("Button should exist in the accesskit output");

    assert_eq!(button.label(), Some(button_text));
    assert!(!button.is_disabled());
}

#[test]
fn disabled_button_node() {
    let button_text = "This is a test button!";

    let output = accesskit_output_single_egui_frame(|ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.add_enabled(false, egui::Button::new(button_text))
        });
    });

    let (_, button) = output
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button)
        .expect("Button should exist in the accesskit output");

    assert_eq!(button.label(), Some(button_text));
    assert!(button.is_disabled());
}

#[test]
fn toggle_button_node() {
    let button_text = "A toggle button";

    let mut selected = false;
    let output = accesskit_output_single_egui_frame(|ui| {
        CentralPanel::default().show(ui, |ui| ui.toggle_value(&mut selected, button_text));
    });

    let (_, toggle) = output
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Button)
        .expect("Toggle button should exist in the accesskit output");

    assert_eq!(toggle.label(), Some(button_text));
    assert!(!toggle.is_disabled());
}

/// Selectable text used to overwrite the role reported by the widget with
/// [`Role::Label`], so a link was indistinguishable from static text.
#[test]
fn selectable_text_keeps_the_role_of_the_widget() {
    let output = accesskit_output_single_egui_frame(|ui| {
        assert!(
            ui.style().interaction.selectable_labels,
            "This test is about the selectable-label code path"
        );
        CentralPanel::default().show(ui, |ui| {
            ui.label("A label");
            ui.add(egui::Link::new("A link"));
            let mut text = "Some text".to_owned();
            ui.add(egui::TextEdit::multiline(&mut text));
        });
    });

    let role_of = |label: &str| {
        output
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some(label))
            .map(|(_, node)| node.role())
    };

    assert_eq!(role_of("A link"), Some(Role::Link));

    // A label has no `label`, only a value, so look it up by role instead:
    assert_eq!(
        output
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::Label)
            .count(),
        1,
        "Only the label itself should be a `Label`; found: {output:#?}"
    );

    assert_eq!(
        output
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::MultilineTextInput)
            .count(),
        1,
        "The multiline `TextEdit` should keep its refined role; found: {output:#?}"
    );
}

#[test]
fn multiple_disabled_widgets() {
    let output = accesskit_output_single_egui_frame(|ui| {
        CentralPanel::default().show(ui, |ui| {
            ui.add_enabled_ui(false, |ui| {
                let _ = ui.button("Button 1");
                let _ = ui.button("Button 2");
                let _ = ui.button("Button 3");
            })
        });
    });

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
    // Windows are explicitly invisible during their first frame's sizing work.
    let output = accesskit_output_after_frames(2, |ui| {
        let mut open = true;
        Window::new("test window")
            .open(&mut open)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                let _ = ui.button("A button");
            });
    });

    let root = output.tree.as_ref().map(|tree| tree.root).unwrap();

    let window_id = assert_window_exists(&output, "test window", root);
    assert_button_exists(&output, "A button", window_id);
    assert_button_exists(&output, "Close window", window_id);
    assert_button_exists(&output, "Hide", window_id);
}

#[test]
fn central_panel_is_a_pane() {
    let output = accesskit_output_single_egui_frame(|ui| {
        CentralPanel::default().show(ui, |ui| ui.label("Hello"));
    });

    assert!(
        output
            .nodes
            .iter()
            .any(|(_, node)| node.role() == Role::Pane),
        "The panel should be a Pane, not an anonymous container; found: {output:#?}",
    );
}

/// A menu hangs under the button that opened it.
#[test]
fn menu_hangs_under_its_button() {
    let mut harness = Harness::new_ui(|ui| {
        MenuButton::new("File").ui(ui, |ui| {
            let _ = ui.button("Open");
        });
    });

    harness.get_by_label("File").click();
    harness.run();

    let button = harness.get_by_role_and_label(Role::Button, "File");
    button.get_by_role(Role::Menu).get_by_label("Open");
}

#[test]
fn combo_box_popup_is_a_list_box() {
    let mut harness = Harness::new_ui(|ui| {
        egui::ComboBox::from_label("Fruit")
            .selected_text("Apple")
            .show_ui(ui, |ui| {
                let _ = ui.selectable_label(false, "Apple");
            });
    });

    harness.get_by_role(Role::ComboBox).click();
    harness.run();

    harness.get_by_role(Role::ListBox).get_by_label("Apple");
}

/// The tooltip text describes the widget, and the tooltip itself hangs under it.
#[test]
fn tooltip_hangs_under_its_widget() {
    let mut harness = Harness::new_ui(|ui| {
        ui.ctx()
            .global_style_mut(|style| style.interaction.tooltip_delay = 0.0);
        let _ = ui.button("Hover me").on_hover_text("Some help");
    });

    harness.get_by_label("Hover me").hover();
    harness.run();

    let button = harness.get_by_role_and_label(Role::Button, "Hover me");
    assert_eq!(
        button.accesskit_node().description(),
        Some("Some help".to_owned())
    );
    button.get_by_role(Role::Tooltip).get_by_label("Some help");
}

/// An icon-only button has no text of its own, so the tooltip text becomes its name.
#[test]
fn tooltip_text_names_an_unnamed_widget() {
    let mut harness = Harness::new_ui(|ui| {
        let _ = ui.add(egui::Button::new("")).on_hover_text("Play");
    });
    harness.run();

    harness.get_by_role_and_label(Role::Button, "Play");
}

fn accesskit_output_single_egui_frame(run_ui: impl FnMut(&mut Ui)) -> TreeUpdate {
    accesskit_output_after_frames(1, run_ui)
}

fn accesskit_output_after_frames(frames: usize, mut run_ui: impl FnMut(&mut Ui)) -> TreeUpdate {
    let ctx = Context::default();
    // Disable animations, so we do not need to wait for animations to end to see the result.
    ctx.global_style_mut(|style| style.animation_time = 0.0);
    ctx.enable_accesskit();

    for _ in 1..frames {
        ctx.run_ui(RawInput::default(), &mut run_ui)
            .drop_without_applying_deltas();
    }
    let mut output = ctx.run_ui(RawInput::default(), run_ui);
    output.textures_delta.clear(); // Don't panic on drop with unapplied deltas

    output
        .platform_output
        .accesskit_update
        .expect("Missing accesskit update")
}

/// <https://github.com/emilk/egui/issues/8557>
#[test]
fn resizable_side_panel_exposes_resize_handle() {
    let output = accesskit_output_single_egui_frame(|ui| {
        Panel::left("test_panel").resizable(true).show(ui, |ui| {
            ui.label("Panel content");
        });
    });

    let (_, handle) = output
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Splitter)
        .expect("Panel resize handle should be exposed as a splitter node");

    assert_eq!(handle.label(), Some("Resize panel"));
    // The divider of a left/right panel is a vertical splitter:
    assert_eq!(handle.orientation(), Some(Orientation::Vertical));
    // The value is the panel size as a fraction of the available space,
    // and the bounds are the reachable sizes (`min_size`/`max_size`) on the same scale:
    let min = handle
        .min_numeric_value()
        .expect("Splitter should have a min");
    let max = handle
        .max_numeric_value()
        .expect("Splitter should have a max");
    let value = handle
        .numeric_value()
        .expect("Splitter should announce the current panel size");
    assert!(
        0.0 <= min && min < value && value < max && max <= 1.0,
        "Expected 0 <= min < value < max <= 1, got min={min} value={value} max={max}"
    );

    for action in [Action::Increment, Action::Decrement, Action::SetValue] {
        assert!(
            handle.supports_action(action),
            "Splitter should support {action:?}"
        );
    }
}

/// `Increment`/`Decrement` are only offered when the panel can actually move that way.
///
/// <https://github.com/emilk/egui/issues/8557>
#[test]
fn fixed_size_panel_resize_handle_offers_no_increment_or_decrement() {
    let output = accesskit_output_single_egui_frame(|ui| {
        Panel::left("test_panel")
            .resizable(true)
            .exact_size(200.0)
            .show(ui, |ui| {
                ui.label("Panel content");
            });
    });

    let (_, handle) = output
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Splitter)
        .expect("Panel resize handle should be exposed as a splitter node");

    assert!(handle.supports_action(Action::SetValue));
    assert!(
        !handle.supports_action(Action::Increment),
        "A panel at its max size cannot grow"
    );
    assert!(
        !handle.supports_action(Action::Decrement),
        "A panel at its min size cannot shrink"
    );
}

/// <https://github.com/emilk/egui/issues/8557>
#[test]
fn resizable_top_bottom_panel_exposes_horizontal_resize_handle() {
    let output = accesskit_output_single_egui_frame(|ui| {
        Panel::top("test_panel").resizable(true).show(ui, |ui| {
            ui.label("Panel content");
        });
    });

    let (_, handle) = output
        .nodes
        .iter()
        .find(|(_, node)| node.role() == Role::Splitter)
        .expect("Panel resize handle should be exposed as a splitter node");

    assert_eq!(handle.label(), Some("Resize panel"));
    // The divider of a top/bottom panel is a horizontal splitter:
    assert_eq!(handle.orientation(), Some(Orientation::Horizontal));
}

/// The handle rect is the split position expanded by the grab radius,
/// so track the center of the strip.
#[track_caller]
fn left_panel_split_pos(harness: &Harness<'_>) -> f32 {
    let rect = harness
        .get_by_role_and_label(Role::Splitter, "Resize panel")
        .rect();
    f32::midpoint(rect.min.x, rect.max.x)
}

/// <https://github.com/emilk/egui/issues/8557>
#[test]
fn resizable_panel_can_be_resized_via_accesskit_actions() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui(|ui| {
            Panel::left("test_panel").resizable(true).show(ui, |ui| {
                // Panels only grow if the contents use the available space:
                ui.take_available_space();
                ui.label("Panel content");
            });
        });
    harness.run();

    let initial_split = left_panel_split_pos(&harness);

    harness
        .get_by_role_and_label(Role::Splitter, "Resize panel")
        .increment_accesskit();
    harness.run();

    let grown_split = left_panel_split_pos(&harness);
    assert!(
        initial_split < grown_split,
        "Increment should move the split outward, got {initial_split} -> {grown_split}"
    );

    harness
        .get_by_role_and_label(Role::Splitter, "Resize panel")
        .set_value_accesskit(0.5);
    harness.run();

    let half_split = left_panel_split_pos(&harness);
    let expected_split = 800.0 * 0.5;
    assert!(
        (half_split - expected_split).abs() < 1.0,
        "SetValue should move the split to half the available space, got {half_split}"
    );

    harness
        .get_by_role_and_label(Role::Splitter, "Resize panel")
        .decrement_accesskit();
    harness.run();

    let shrunk_split = left_panel_split_pos(&harness);
    assert!(
        shrunk_split < half_split,
        "Decrement should move the split inward, got {half_split} -> {shrunk_split}"
    );
}

/// The grab handle of a fully collapsed panel lets assistive technologies
/// pull the panel open, mirroring the drag-to-expand gesture.
///
/// <https://github.com/emilk/egui/issues/8557>
#[test]
fn collapsed_panel_can_be_expanded_via_accesskit_action() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui_state(
            |ui, expanded| {
                Panel::left("test_panel")
                    .resizable(true)
                    .show_collapsible(ui, expanded, |ui| {
                        ui.take_available_space();
                        ui.label("Panel content");
                    });
            },
            true,
        );
    harness.run();

    *harness.state_mut() = false;
    harness.run();

    // Fully collapsed: only the grab handle remains.
    harness
        .get_by_role_and_label(Role::Splitter, "Resize panel")
        .increment_accesskit();
    harness.run();

    assert!(
        *harness.state(),
        "Increment on the collapsed panel's resize handle should expand the panel"
    );
}

/// `Decrement` past the minimum size collapses the panel,
/// mirroring the drag-to-collapse gesture.
///
/// <https://github.com/emilk/egui/issues/8557>
#[test]
fn collapsible_panel_can_be_collapsed_via_accesskit_action() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui_state(
            |ui, expanded| {
                Panel::left("test_panel")
                    .resizable(true)
                    .min_size(100.0)
                    .default_size(100.0)
                    .show_collapsible(ui, expanded, |ui| {
                        ui.take_available_space();
                        ui.label("Panel content");
                    });
            },
            true,
        );
    harness.run();

    let handle = harness.get_by_role_and_label(Role::Splitter, "Resize panel");
    assert!(
        handle
            .accesskit_node()
            .data()
            .supports_action(Action::Decrement),
        "An expanded collapsible panel at its min size can still be collapsed"
    );
    handle.decrement_accesskit();
    harness.run();

    assert!(
        !*harness.state(),
        "Decrement past the min size should collapse the panel"
    );
}

/// In `show_switched`, the collapsed panel shares the resize handle with the
/// expanded one. `Increment` on it must open the expanded panel even when the
/// collapsed panel has a fixed size, just like dragging past its `max_size` does.
///
/// <https://github.com/emilk/egui/issues/8557>
#[test]
fn switched_panel_can_be_expanded_via_accesskit_action() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .build_ui_state(
            |ui, expanded| {
                Panel::show_switched(
                    ui,
                    expanded,
                    Panel::left("collapsed").resizable(true).exact_size(24.0),
                    Panel::left("expanded").resizable(true),
                    |ui, _| {
                        ui.take_available_space();
                        ui.label("Panel content");
                    },
                );
            },
            false,
        );
    harness.run();

    let handle = harness.get_by_role_and_label(Role::Splitter, "Resize panel");
    assert!(
        handle
            .accesskit_node()
            .data()
            .supports_action(Action::Increment),
        "The collapsed panel's handle should offer to expand it"
    );
    handle.increment_accesskit();
    harness.run();

    assert!(
        *harness.state(),
        "Increment on the collapsed panel's resize handle should expand the panel"
    );
}

#[track_caller]
fn assert_button_exists(tree: &TreeUpdate, label: &str, parent: NodeId) {
    let (node_id, _) = tree
        .nodes
        .iter()
        .find(|(_, node)| {
            !node.is_hidden() && node.role() == Role::Button && node.label() == Some(label)
        })
        .expect("No visible button with that label exists.");

    assert_parent_child(tree, parent, *node_id);
}

#[track_caller]
fn assert_window_exists(tree: &TreeUpdate, title: &str, parent: NodeId) -> NodeId {
    let (node_id, _) = tree
        .nodes
        .iter()
        .find(|(_, node)| {
            !node.is_hidden() && node.role() == Role::Window && node.label() == Some(title)
        })
        .expect("No visible window with that title exists.");

    assert_parent_child(tree, parent, *node_id);

    *node_id
}

#[track_caller]
fn assert_parent_child(tree: &TreeUpdate, parent_id: NodeId, child: NodeId) {
    assert!(
        has_child_recursively(tree, parent_id, child),
        "Node is not a child of the given parent."
    );
}

fn has_child_recursively(tree: &TreeUpdate, parent: NodeId, child: NodeId) -> bool {
    let (_, parent) = tree
        .nodes
        .iter()
        .find(|(id, _)| id == &parent)
        .expect("Parent does not exist.");

    for &c in parent.children() {
        if c == child || has_child_recursively(tree, c, child) {
            return true;
        }
    }

    false
}

/// A `Ui`, an area and a popup can be named, and then found by that name.
#[test]
fn named_ui_area_and_popup() {
    let mut harness = Harness::new_ui(|ui| {
        ui.scope_builder(
            egui::UiBuilder::new().accessibility_label("Toolbox"),
            |ui| {
                let _ = ui.button("Hammer");
            },
        );

        egui::Area::new(egui::Id::unique("area"))
            .accessible_name("Floating notes")
            .show(ui.ctx(), |ui| {
                ui.label("A note");
            });

        let response = ui.button("Open");
        egui::Popup::from_response(&response)
            .open(true)
            .accessible_name("Options")
            .show(|ui| {
                let _ = ui.button("Option A");
            });
    });
    harness.run();

    harness.get_by_label("Toolbox").get_by_label("Hammer");
    harness
        .get_by_label("Floating notes")
        .get_by_label("A note");
    harness
        .get_by_role_and_label(Role::Dialog, "Options")
        .get_by_label("Option A");
}
