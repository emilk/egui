use std::collections::{HashMap, HashSet};

use egui::{Pos2, Rect, Ui};

use super::{
    Behavior, Branch, DropContext, GcAction, Grid, InsertionPoint, Layout, LayoutInsertion, Linear,
    LinearDir, Node, NodeId, SimplificationOptions, SimplifyAction, Tabs, UiResponse,
};

/// Contains all node state, but no root.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Nodes<Leaf> {
    pub nodes: HashMap<NodeId, Node<Leaf>>,

    /// Filled in by the layout step at the start of each frame.
    #[serde(default, skip)]
    pub(super) rects: HashMap<NodeId, Rect>,
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
    pub(super) fn try_rect(&self, node_id: NodeId) -> Option<Rect> {
        self.rects.get(&node_id).copied()
    }

    pub(super) fn rect(&self, node_id: NodeId) -> Rect {
        let rect = self.try_rect(node_id);
        debug_assert!(rect.is_some(), "Failed to find rect for {node_id:?}");
        rect.unwrap_or(egui::Rect::from_min_max(Pos2::ZERO, Pos2::ZERO))
    }

    pub fn get(&self, node_id: NodeId) -> Option<&Node<Leaf>> {
        self.nodes.get(&node_id)
    }

    pub fn get_mut(&mut self, node_id: NodeId) -> Option<&mut Node<Leaf>> {
        self.nodes.get_mut(&node_id)
    }

    #[must_use]
    pub fn insert_node(&mut self, node: Node<Leaf>) -> NodeId {
        let id = NodeId::random();
        self.nodes.insert(id, node);
        id
    }

    #[must_use]
    pub fn insert_leaf(&mut self, leaf: Leaf) -> NodeId {
        self.insert_node(Node::Leaf(leaf))
    }

    #[must_use]
    pub fn insert_branch(&mut self, branch: Branch) -> NodeId {
        self.insert_node(Node::Branch(branch))
    }

    #[must_use]
    pub fn insert_tab_node(&mut self, children: Vec<NodeId>) -> NodeId {
        self.insert_node(Node::Branch(Branch::new_tabs(children)))
    }

    #[must_use]
    pub fn insert_horizontal_node(&mut self, children: Vec<NodeId>) -> NodeId {
        self.insert_node(Node::Branch(Branch::new_linear(
            LinearDir::Horizontal,
            children,
        )))
    }

    #[must_use]
    pub fn insert_vertical_node(&mut self, children: Vec<NodeId>) -> NodeId {
        self.insert_node(Node::Branch(Branch::new_linear(
            LinearDir::Vertical,
            children,
        )))
    }

    #[must_use]
    pub fn insert_grid_node(&mut self, children: Vec<NodeId>) -> NodeId {
        self.insert_node(Node::Branch(Branch::new_grid(children)))
    }

    pub fn insert(&mut self, insertion_point: InsertionPoint, child_id: NodeId) {
        let InsertionPoint {
            parent_id,
            insertion,
        } = insertion_point;

        let Some(mut node) = self.nodes.remove(&parent_id) else {
            log::warn!("Failed to insert: could not find parent {parent_id:?}");
            return;
        };

        match insertion {
            LayoutInsertion::Tabs(index) => {
                if let Node::Branch(Branch::Tabs(tabs)) = &mut node {
                    let index = index.min(tabs.children.len());
                    tabs.children.insert(index, child_id);
                    tabs.active = child_id;
                    self.nodes.insert(parent_id, node);
                } else {
                    let new_node_id = self.insert_node(node);
                    let mut tabs = Tabs::new(vec![new_node_id]);
                    tabs.children.insert(index.min(1), child_id);
                    tabs.active = child_id;
                    self.nodes
                        .insert(parent_id, Node::Branch(Branch::Tabs(tabs)));
                }
            }
            LayoutInsertion::Horizontal(index) => {
                if let Node::Branch(Branch::Linear(Linear {
                    dir: LinearDir::Horizontal,
                    children,
                    ..
                })) = &mut node
                {
                    let index = index.min(children.len());
                    children.insert(index, child_id);
                    self.nodes.insert(parent_id, node);
                } else {
                    let new_node_id = self.insert_node(node);
                    let mut linear = Linear::new(LinearDir::Horizontal, vec![new_node_id]);
                    linear.children.insert(index.min(1), child_id);
                    self.nodes
                        .insert(parent_id, Node::Branch(Branch::Linear(linear)));
                }
            }
            LayoutInsertion::Vertical(index) => {
                if let Node::Branch(Branch::Linear(Linear {
                    dir: LinearDir::Vertical,
                    children,
                    ..
                })) = &mut node
                {
                    let index = index.min(children.len());
                    children.insert(index, child_id);
                    self.nodes.insert(parent_id, node);
                } else {
                    let new_node_id = self.insert_node(node);
                    let mut linear = Linear::new(LinearDir::Vertical, vec![new_node_id]);
                    linear.children.insert(index.min(1), child_id);
                    self.nodes
                        .insert(parent_id, Node::Branch(Branch::Linear(linear)));
                }
            }
            LayoutInsertion::Grid(insert_location) => {
                if let Node::Branch(Branch::Grid(grid)) = &mut node {
                    grid.locations.retain(|_, pos| *pos != insert_location);
                    grid.locations.insert(child_id, insert_location);
                    grid.children.push(child_id);
                    self.nodes.insert(parent_id, node);
                } else {
                    let new_node_id = self.insert_node(node);
                    let mut grid = Grid::new(vec![new_node_id, child_id]);
                    grid.locations.insert(child_id, insert_location);
                    self.nodes
                        .insert(parent_id, Node::Branch(Branch::Grid(grid)));
                }
            }
        }
    }

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
            branch.layout_recursive(self, style, behavior, rect);
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
        // NOTE: important that we get the rect and node in two steps,
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
        drop_context.on_node(behavior, ui.style(), node_id, rect, &node);

        // Each node gets its own `Ui`, nested inside each other, with proper clip rectangles.
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
            // TODO(emilk): join nested versions of the same horizontal/vertical layouts

            branch.simplify_children(|child| self.simplify(options, child));

            if branch.layout() == Layout::Tabs {
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
                let is_tabs = branch.layout() == Layout::Tabs;
                for &child in branch.children() {
                    self.make_all_leaves_children_of_tabs(is_tabs, child);
                }
            }
        }

        self.nodes.insert(it, node);
    }
}
