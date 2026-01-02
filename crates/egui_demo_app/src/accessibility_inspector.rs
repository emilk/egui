use std::mem;

use accesskit::{Action, ActionRequest, NodeId};
use accesskit_consumer::{FilterResult, Node, Tree, TreeChangeHandler};

use eframe::epaint::text::TextWrapMode;
use egui::{
    Button, Color32, Event, Frame, FullOutput, Id, Key, KeyboardShortcut, Label, Modifiers, Panel,
    RawInput, RichText, ScrollArea, Ui, collapsing_header::CollapsingState,
};

/// This [`egui::Plugin`] adds an inspector panel.
///
/// It can be opened with the `(Cmd/Ctrl)+Alt+I`. It shows the current AccessKit tree and details
/// for the selected node.
/// Useful when debugging accessibility issues or trying to understand the structure of the Ui.
///
/// Add via
/// ```
/// # use egui_demo_app::accessibility_inspector::AccessibilityInspectorPlugin;
/// # let ctx = egui::Context::default();
/// ctx.add_plugin(AccessibilityInspectorPlugin::default());
/// ```
#[derive(Default, Debug)]
pub struct AccessibilityInspectorPlugin {
    pub open: bool,
    tree: Option<accesskit_consumer::Tree>,
    selected_node: Option<Id>,
    queued_action: Option<ActionRequest>,
}

struct ChangeHandler;

impl TreeChangeHandler for ChangeHandler {
    fn node_added(&mut self, _node: &Node<'_>) {}

    fn node_updated(&mut self, _old_node: &Node<'_>, _new_node: &Node<'_>) {}

    fn focus_moved(&mut self, _old_node: Option<&Node<'_>>, _new_node: Option<&Node<'_>>) {}

    fn node_removed(&mut self, _node: &Node<'_>) {}
}

impl egui::Plugin for AccessibilityInspectorPlugin {
    fn debug_name(&self) -> &'static str {
        "Accessibility Inspector"
    }

    fn input_hook(&mut self, input: &mut RawInput) {
        if let Some(queued_action) = self.queued_action.take() {
            input
                .events
                .push(Event::AccessKitActionRequest(queued_action));
        }
    }

    fn output_hook(&mut self, output: &mut FullOutput) {
        if let Some(update) = output.platform_output.accesskit_update.clone() {
            self.tree = match mem::take(&mut self.tree) {
                None => {
                    // Create a new tree if it doesn't exist
                    Some(Tree::new(update, true))
                }
                Some(mut tree) => {
                    // Update the tree with the latest accesskit data
                    tree.update_and_process_changes(update, &mut ChangeHandler);

                    Some(tree)
                }
            }
        }
    }

    fn on_begin_pass(&mut self, ui: &mut Ui) {
        if ui.input_mut(|i| {
            i.consume_shortcut(&KeyboardShortcut::new(
                Modifiers::COMMAND | Modifiers::ALT,
                Key::I,
            ))
        }) {
            self.open = !self.open;
        }

        if !self.open {
            return;
        }

        ui.enable_accesskit();

        Panel::right(Self::id()).show_inside(ui, |ui| {
            ui.heading("🔎 AccessKit Inspector");
            if let Some(selected_node) = self.selected_node {
                Panel::bottom(Self::id().with("details_panel"))
                    .frame(Frame::new())
                    .show_separator_line(false)
                    .show_inside(ui, |ui| {
                        self.selection_ui(ui, selected_node);
                    });
            }

            ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
            ScrollArea::vertical().show(ui, |ui| {
                if let Some(tree) = &self.tree {
                    Self::node_ui(ui, &tree.state().root(), &mut self.selected_node);
                }
            });
        });
    }
}

impl AccessibilityInspectorPlugin {
    fn id() -> Id {
        Id::new("Accessibility Inspector")
    }

    fn selection_ui(&mut self, ui: &mut Ui, selected_node: Id) {
        ui.separator();

        if let Some(tree) = &self.tree
            && let Some(node) = tree.state().node_by_id(NodeId::from(selected_node.value()))
        {
            let node_response = ui.ctx().read_response(selected_node);

            if let Some(widget_response) = node_response {
                ui.debug_painter().debug_rect(
                    widget_response.rect,
                    ui.style_mut().visuals.selection.bg_fill,
                    "",
                );
            }

            egui::Grid::new("node_details_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Node ID");
                    ui.strong(format!("{selected_node:?}"));
                    ui.end_row();

                    ui.label("Role");
                    ui.strong(format!("{:?}", node.role()));
                    ui.end_row();

                    ui.label("Label");
                    ui.add(
                        Label::new(RichText::new(node.label().unwrap_or_default()).strong())
                            .truncate(),
                    );
                    ui.end_row();

                    ui.label("Value");
                    ui.add(
                        Label::new(RichText::new(node.value().unwrap_or_default()).strong())
                            .truncate(),
                    );
                    ui.end_row();

                    ui.label("Children");
                    ui.label(RichText::new(node.children().len().to_string()).strong());

                    ui.end_row();
                });

            ui.label("Actions");
            ui.horizontal_wrapped(|ui| {
                // Iterate through all possible actions via the `Action::n` helper.
                let mut current_action = 0;
                let all_actions = std::iter::from_fn(|| {
                    let action = Action::n(current_action);
                    current_action += 1;
                    action
                });

                for action in all_actions {
                    if node.supports_action(action, &|_node| FilterResult::Include)
                        && ui.button(format!("{action:?}")).clicked()
                    {
                        let action_request = ActionRequest {
                            target: node.id(),
                            action,
                            data: None,
                        };
                        self.queued_action = Some(action_request);
                    }
                }
            });
        } else {
            ui.label("Node not found");
        }
    }

    fn node_ui(ui: &mut Ui, node: &Node<'_>, selected_node: &mut Option<Id>) {
        if node.id() == Self::id().value().into()
            || node
                .value()
                .as_deref()
                .is_some_and(|l| l.contains("AccessKit Inspector"))
        {
            return;
        }
        let label = node
            .label()
            .or_else(|| node.value())
            .unwrap_or_else(|| node.id().0.to_string());
        let label = format!("({:?}) {}", node.role(), label);

        // Safety: This is safe since the `accesskit::NodeId` was created from an `egui::Id`.
        #[expect(unsafe_code)]
        let egui_node_id = unsafe { Id::from_high_entropy_bits(node.id().0) };

        ui.push_id(node.id(), |ui| {
            let child_count = node.children().len();
            let has_children = child_count > 0;
            let default_open = child_count == 1 && node.role() != accesskit::Role::Label;

            let mut collapsing = CollapsingState::load_with_default_open(
                ui.ctx(),
                egui_node_id.with("ak_collapse"),
                default_open,
            );

            let header_response = ui.horizontal(|ui| {
                let text = if collapsing.is_open() { "⏷" } else { "⏵" };

                if ui
                    .add_visible(has_children, Button::new(text).frame_when_inactive(false))
                    .clicked()
                {
                    collapsing.set_open(!collapsing.is_open());
                }
                let label_response =
                    ui.selectable_value(selected_node, Some(egui_node_id), label.clone());
                if label_response.hovered() {
                    let widget_response = ui.ctx().read_response(egui_node_id);

                    if let Some(widget_response) = widget_response {
                        ui.debug_painter()
                            .debug_rect(widget_response.rect, Color32::RED, "");
                    }
                }
            });

            if has_children {
                collapsing.show_body_indented(&header_response.response, ui, |ui| {
                    node.children().for_each(|c| {
                        Self::node_ui(ui, &c, selected_node);
                    });
                });
            }

            collapsing.store(ui.ctx());
        });
    }
}
