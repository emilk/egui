use egui::{Id, NumExt as _, Rect, Ui};

use super::{
    is_possible_drag, Behavior, Branch, DropContext, InsertionPoint, Node, NodeId, Nodes,
    SimplificationOptions, SimplifyAction,
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

impl<Leaf> Dock<Leaf> {
    pub fn new(root: NodeId, nodes: Nodes<Leaf>) -> Self {
        Self {
            root,
            nodes,
            smoothed_preview_rect: None,
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
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

    /// Show the dock in the given [`Ui`].
    ///
    /// The dock will use upp all the avilable space - nothing more, nothing less.
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

        self.preview_dragged_node(behavior, &drop_context, ui);
    }

    fn preview_dragged_node(
        &mut self,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &DropContext,
        ui: &mut Ui,
    ) {
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
                        // TODO(emilk): preview contents?
                        let text = behavior.tab_title_for_node(&self.nodes, dragged_node_id);
                        ui.label(text);
                    });
                });

            if let Some(preview_rect) = drop_context.preview_rect {
                let preview_rect = self.smooth_preview_rect(ui.ctx(), preview_rect);

                let parent_rect = drop_context
                    .best_insertion
                    .and_then(|insertion_point| self.nodes.try_rect(insertion_point.parent_id));

                behavior.paint_drag_preview(ui.visuals(), ui.painter(), parent_rect, preview_rect);

                if behavior.preview_dragged_leaves() {
                    // TODO(emilk): add support for previewing branches too.
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

    /// Take the preview rectangle and smooth it over time.
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

    /// Move the given node to the given insertion point.
    pub fn move_node(&mut self, moved_node_id: NodeId, insertion_point: InsertionPoint) {
        log::debug!(
            "Moving {moved_node_id:?} into {:?}",
            insertion_point.insertion
        );
        self.remove_node_id_from_parent(moved_node_id);
        self.nodes.insert(insertion_point, moved_node_id);
    }

    /// Find the currently dragged node, if any.
    pub fn dragged_id(&self, ctx: &egui::Context) -> Option<NodeId> {
        if !is_possible_drag(ctx) {
            // We're not sure we're dragging _at all_ yet.
            return None;
        }

        for &node_id in self.nodes.nodes.keys() {
            if node_id == self.root {
                continue; // not allowed to drag root
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

    /// Performs no simplifcations, nor does it remove the actual [`Node`].
    pub fn remove_node_id_from_parent(&mut self, remove_me: NodeId) {
        for parent in self.nodes.nodes.values_mut() {
            if let Node::Branch(branch) = parent {
                branch.retain(|child| child != remove_me);
            }
        }
    }
}
