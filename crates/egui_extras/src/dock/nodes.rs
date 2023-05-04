use std::collections::{HashMap, HashSet};

use egui::{Rect, Ui};

use super::{
    Behavior, Branch, DropContext, GcAction, InsertionPoint, Layout, LayoutInsertion, Node, NodeId,
    SimplificationOptions, SimplifyAction, UiResponse,
};

/// Contains all node state, but no root.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Nodes<Leaf> {
    pub nodes: HashMap<NodeId, Node<Leaf>>,

    /// Filled in by the layout step at the start of each frame.
    #[serde(default, skip)]
    pub rects: HashMap<NodeId, Rect>,
}

impl<Leaf> Default for Nodes<Leaf> {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
            rects: Default::default(),
        }
    }
}

// ----------------------------------------------------------------------------

impl<Leaf> Nodes<Leaf> {
    pub(super) fn gc_root(&mut self, behavior: &mut dyn Behavior<Leaf>, root_id: NodeId) {
        let mut visited = HashSet::default();
        self.gc_node_id(behavior, &mut visited, root_id);

        if visited.len() < self.nodes.len() {
            log::warn!(
                "GC collecting nodes: {:?}",
                self.nodes
                    .keys()
                    .filter(|id| !visited.contains(id))
                    .collect::<Vec<_>>()
            );
        }

        self.nodes.retain(|node_id, _| visited.contains(node_id));
    }

    fn gc_node_id(
        &mut self,
        behavior: &mut dyn Behavior<Leaf>,
        visited: &mut HashSet<NodeId>,
        node_id: NodeId,
    ) -> GcAction {
        let Some(mut node) = self.nodes.remove(&node_id) else { return GcAction::Remove; };
        if !visited.insert(node_id) {
            log::warn!("Cycle or duplication detected");
            return GcAction::Remove;
        }

        match &mut node {
            Node::Leaf(leaf) => {
                if !behavior.retain_leaf(leaf) {
                    return GcAction::Remove;
                }
            }
            Node::Branch(branch) => {
                branch.retain(|child| self.gc_node_id(behavior, visited, child) == GcAction::Keep);
            }
        }
        self.nodes.insert(node_id, node);
        GcAction::Keep
    }

    pub(super) fn layout_node(
        &mut self,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
        rect: Rect,
        node_id: NodeId,
    ) {
        let Some(mut node) = self.nodes.remove(&node_id) else {
            log::warn!("Failed to find node {node_id:?} during layout");
            return;
        };
        self.rects.insert(node_id, rect);

        if let Node::Branch(branch) = &mut node {
            branch.layout(self, style, behavior, rect);
        }

        self.nodes.insert(node_id, node);
    }

    pub(super) fn node_ui(
        &mut self,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut Ui,
        node_id: NodeId,
    ) {
        // NOTE: important that we get thr rect and node in two steps,
        // otherwise we could loose the node when there is no rect.
        let Some(rect) = self.try_rect(node_id) else {
            log::warn!("Failed to find rect for node {node_id:?} during ui");
            return
        };
        let Some(mut node) = self.nodes.remove(&node_id) else {
            log::warn!("Failed to find node {node_id:?} during ui");
            return
        };

        let drop_context_was_enabled = drop_context.enabled;
        if Some(node_id) == drop_context.dragged_node_id {
            // Can't drag a node onto self or any children
            drop_context.enabled = false;
        }
        drop_context.on_node(node_id, rect, &node);

        drop_context.suggest_rect(
            InsertionPoint::new(node_id, LayoutInsertion::Tabs(usize::MAX)),
            rect.split_top_bottom_at_y(rect.top() + behavior.tab_bar_height(ui.style()))
                .1,
        );

        let mut ui = egui::Ui::new(
            ui.ctx().clone(),
            ui.layer_id(),
            ui.id().with(node_id),
            rect,
            rect,
        );
        match &mut node {
            Node::Leaf(leaf) => {
                if behavior.leaf_ui(&mut ui, node_id, leaf) == UiResponse::DragStarted {
                    ui.memory_mut(|mem| mem.set_dragged_id(node_id.id()));
                }
            }
            Node::Branch(branch) => {
                branch.ui(self, behavior, drop_context, &mut ui, rect, node_id);
            }
        };

        self.nodes.insert(node_id, node);
        drop_context.enabled = drop_context_was_enabled;
    }

    pub(super) fn simplify(
        &mut self,
        options: &SimplificationOptions,
        it: NodeId,
    ) -> SimplifyAction {
        let Some(mut node) = self.nodes.remove(&it) else {
            log::warn!("Failed to find node {it:?} during simplify");
            return SimplifyAction::Remove;
        };

        if let Node::Branch(branch) = &mut node {
            // TODO: join nested versions of the same horizontal/vertical layouts

            branch.simplify_children(|child| self.simplify(options, child));

            if branch.get_layout() == Layout::Tabs {
                if options.prune_empty_tabs && branch.is_empty() {
                    log::debug!("Simplify: removing empty tabs node");
                    return SimplifyAction::Remove;
                }
                if options.prune_single_child_tabs && branch.children().len() == 1 {
                    if options.all_leaves_must_have_tabs
                        && matches!(self.get(branch.children()[0]), Some(Node::Leaf(_)))
                    {
                        // Keep it
                    } else {
                        log::debug!("Simplify: collapsing single-child tabs node");
                        return SimplifyAction::Replace(branch.children()[0]);
                    }
                }
            } else {
                if options.prune_empty_layouts && branch.is_empty() {
                    log::debug!("Simplify: removing empty layout node");
                    return SimplifyAction::Remove;
                }
                if options.prune_single_child_layouts && branch.children().len() == 1 {
                    log::debug!("Simplify: collapsing single-child layout node");
                    return SimplifyAction::Replace(branch.children()[0]);
                }
            }
        }

        self.nodes.insert(it, node);
        SimplifyAction::Keep
    }

    pub(super) fn make_all_leaves_children_of_tabs(&mut self, parent_is_tabs: bool, it: NodeId) {
        let Some(mut node) = self.nodes.remove(&it) else {
            log::warn!("Failed to find node {it:?} during make_all_leaves_children_of_tabs");
            return;
        };

        match &mut node {
            Node::Leaf(_) => {
                if !parent_is_tabs {
                    // Add tabs to this leaf:
                    log::debug!("Auto-adding Tabs-parent to leaf {it:?}");
                    let new_id = NodeId::random();
                    self.nodes.insert(new_id, node);
                    self.nodes
                        .insert(it, Node::Branch(Branch::new_tabs(vec![new_id])));
                    return;
                }
            }
            Node::Branch(branch) => {
                let is_tabs = branch.get_layout() == Layout::Tabs;
                for &child in branch.children() {
                    self.make_all_leaves_children_of_tabs(is_tabs, child);
                }
            }
        }

        self.nodes.insert(it, node);
    }
}
