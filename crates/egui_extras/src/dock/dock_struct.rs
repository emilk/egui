use egui::{Id, NumExt as _, Pos2, Rect, Ui};

use super::{
    is_possible_drag, Behavior, Branch, DropContext, GcAction, Grid, InsertionPoint,
    LayoutInsertion, Linear, LinearDir, Node, NodeId, Nodes, SimplificationOptions, SimplifyAction,
    Tabs,
};

/// The top level type. Contains all persistent state, including layouts and sizes.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Dock<Leaf> {
    pub root: NodeId,
    pub nodes: Nodes<Leaf>,

    /// Smoothed avaerage of preview
    #[serde(skip)]
    pub smoothed_preview_rect: Option<Rect>,
}

impl<Leaf> Default for Dock<Leaf> {
    fn default() -> Self {
        Self {
            root: Default::default(),
            nodes: Default::default(),
            smoothed_preview_rect: None,
        }
    }
}

impl<Leaf: std::fmt::Debug> std::fmt::Debug for Dock<Leaf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Print a hiearchical view of the tree:
        fn format_node<Leaf: std::fmt::Debug>(
            f: &mut std::fmt::Formatter<'_>,
            nodes: &Nodes<Leaf>,
            indent: usize,
            node_id: NodeId,
        ) -> std::fmt::Result {
            write!(f, "{} {node_id:?} ", "  ".repeat(indent))?;
            if let Some(node) = nodes.get(node_id) {
                match node {
                    Node::Leaf(leaf) => writeln!(f, "Leaf {leaf:?}"),
                    Node::Branch(branch) => {
                        writeln!(
                            f,
                            "{}",
                            match branch {
                                Branch::Tabs(_) => "Tabs",
                                Branch::Linear(_) => "Linear",
                                Branch::Grid(_) => "Grid",
                            }
                        )?;
                        for &child in branch.children() {
                            format_node(f, nodes, indent + 1, child)?;
                        }
                        Ok(())
                    }
                }
            } else {
                write!(f, "DANGLING {node_id:?}")
            }
        }

        writeln!(f, "Dock {{")?;
        format_node(f, &self.nodes, 1, self.root)?;
        write!(f, "\n}}")
    }
}

// ----------------------------------------------------------------------------

// Construction

impl<Leaf> Dock<Leaf> {
    pub fn new(root: NodeId, nodes: Nodes<Leaf>) -> Self {
        Self {
            root,
            nodes,
            smoothed_preview_rect: None,
        }
    }

    pub fn parent_of(&self, node_id: NodeId) -> Option<NodeId> {
        self.nodes
            .nodes
            .iter()
            .find(|(_, node)| {
                if let Node::Branch(branch) = node {
                    branch.children().contains(&node_id)
                } else {
                    false
                }
            })
            .map(|(id, _)| *id)
    }
}

impl<Leaf> Nodes<Leaf> {
    pub fn try_rect(&self, node_id: NodeId) -> Option<Rect> {
        self.rects.get(&node_id).copied()
    }

    pub fn rect(&self, node_id: NodeId) -> Rect {
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

    /// Performs no simplifcations!
    fn remove_node_id_from_parent(&mut self, it: NodeId, remove: NodeId) -> GcAction {
        if it == remove {
            return GcAction::Remove;
        }
        let Some(mut node) = self.nodes.remove(&it) else {
            log::warn!("Unexpected missing node during removal");
            return GcAction::Remove;
        };
        if let Node::Branch(branch) = &mut node {
            branch.retain(|child| self.remove_node_id_from_parent(child, remove) == GcAction::Keep);
        }
        self.nodes.insert(it, node);
        GcAction::Keep
    }

    fn insert(&mut self, insertion_point: InsertionPoint, child_id: NodeId) {
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
}

// Usage
impl<Leaf> Dock<Leaf> {
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Show all the leaves in the dock.
    pub fn ui(&mut self, behavior: &mut dyn Behavior<Leaf>, ui: &mut Ui) {
        let options = behavior.simplification_options();
        self.simplify(&options);
        if options.all_leaves_must_have_tabs {
            self.nodes
                .make_all_leaves_children_of_tabs(false, self.root);
        }

        self.nodes.gc_root(behavior, self.root);

        self.nodes.rects.clear();

        // Check if anything is being dragged:
        let mut drop_context = DropContext {
            enabled: true,
            dragged_node_id: self.dragged_id(ui.ctx()),
            mouse_pos: ui.input(|i| i.pointer.hover_pos()),
            best_dist_sq: f32::INFINITY,
            best_insertion: None,
            preview_rect: None,
        };

        self.nodes.layout_node(
            ui.style(),
            behavior,
            ui.available_rect_before_wrap(),
            self.root,
        );

        self.nodes
            .node_ui(behavior, &mut drop_context, ui, self.root);

        if let (Some(mouse_pos), Some(dragged_node_id)) =
            (drop_context.mouse_pos, drop_context.dragged_node_id)
        {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);

            // Preview what is being dragged:
            egui::Area::new(Id::new((dragged_node_id, "preview")))
                .pivot(egui::Align2::CENTER_CENTER)
                .current_pos(mouse_pos)
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    let mut frame = egui::Frame::popup(ui.style());
                    frame.fill = frame.fill.gamma_multiply(0.5); // Make see-through
                    frame.show(ui, |ui| {
                        // TODO: preview contents
                        let text = behavior.tab_text_for_node(&self.nodes, dragged_node_id);
                        ui.label(text);
                    });
                });

            if let Some(preview_rect) = drop_context.preview_rect {
                let preview_rect = self.smooth_preview_rect(ui.ctx(), preview_rect);

                let preview_stroke = ui.visuals().selection.stroke;
                let preview_color = preview_stroke.color;

                if let Some(insertion_point) = &drop_context.best_insertion {
                    if let Some(parent_rect) = self.nodes.try_rect(insertion_point.parent_id) {
                        // Show which parent we will be dropped into
                        ui.painter().rect_stroke(parent_rect, 1.0, preview_stroke);
                    }
                }

                ui.painter().rect(
                    preview_rect,
                    1.0,
                    preview_color.gamma_multiply(0.5),
                    preview_stroke,
                );

                let preview_child = false;
                if preview_child {
                    // Preview actual child?
                    if preview_rect.width() > 32.0 && preview_rect.height() > 32.0 {
                        if let Some(Node::Leaf(leaf)) = self.nodes.get_mut(dragged_node_id) {
                            let _ = behavior.leaf_ui(
                                &mut ui.child_ui(preview_rect, *ui.layout()),
                                dragged_node_id,
                                leaf,
                            );
                        }
                    }
                }
            }

            if ui.input(|i| i.pointer.any_released()) {
                ui.memory_mut(|mem| mem.stop_dragging());
                if let Some(insertion_point) = drop_context.best_insertion {
                    self.move_node(dragged_node_id, insertion_point);
                }
                self.smoothed_preview_rect = None;
            }
        } else {
            self.smoothed_preview_rect = None;
        }
    }

    fn smooth_preview_rect(&mut self, ctx: &egui::Context, new_rect: Rect) -> Rect {
        let dt = ctx.input(|input| input.stable_dt).at_most(0.1);
        let t = egui::emath::exponential_smooth_factor(0.9, 0.05, dt);

        let smoothed = self.smoothed_preview_rect.get_or_insert(new_rect);
        *smoothed = smoothed.lerp_towards(&new_rect, t);

        let diff = smoothed.min.distance(new_rect.min) + smoothed.max.distance(new_rect.max);
        if diff < 0.5 {
            *smoothed = new_rect;
        } else {
            ctx.request_repaint();
        }
        *smoothed
    }

    fn simplify(&mut self, options: &SimplificationOptions) {
        match self.nodes.simplify(options, self.root) {
            SimplifyAction::Remove => {
                log::warn!("Tried to simplify root node!"); // TODO: handle this
            }
            SimplifyAction::Keep => {}
            SimplifyAction::Replace(new_root) => {
                self.root = new_root;
            }
        }
    }

    fn move_node(&mut self, moved_node_id: NodeId, insertion_point: InsertionPoint) {
        log::debug!(
            "Moving {moved_node_id:?} into {:?}",
            insertion_point.insertion
        );
        self.remove_node_id_from_parent(moved_node_id);
        self.nodes.insert(insertion_point, moved_node_id);
    }

    /// Find the currently dragged node, if any.
    fn dragged_id(&self, ctx: &egui::Context) -> Option<NodeId> {
        if !is_possible_drag(ctx) {
            // We're not sure we're dragging _at all_ yet.
            return None;
        }

        for &node_id in self.nodes.nodes.keys() {
            if node_id == self.root {
                continue; // now allowed to drag root
            }

            let id = node_id.id();
            let is_node_being_dragged = ctx.memory(|mem| mem.is_being_dragged(id));
            if is_node_being_dragged {
                // Abort drags on escape:
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    ctx.memory_mut(|mem| mem.stop_dragging());
                    return None;
                }

                return Some(node_id);
            }
        }
        None
    }

    /// Performs no simplifcations!
    fn remove_node_id_from_parent(&mut self, dragged_node_id: NodeId) {
        self.nodes
            .remove_node_id_from_parent(self.root, dragged_node_id);
    }
}
