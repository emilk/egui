// # TODO
// * A new ui for each node, nested
// * Better drag-and-drop around the "empty" drag source
// * Resizing of vertical layouts and grids
// * Styling
// * Handle rects without a lot of unwraps

use std::collections::{HashMap, HashSet};

use egui::{Id, Key, NumExt, Pos2, Rect, Response, Sense, TextStyle, Ui, WidgetText};

mod branch;

pub use branch::{Branch, Grid, GridLoc, Layout, Linear, LinearDir, Tabs};

// ----------------------------------------------------------------------------
// Types required for state

/// An identifier for a node in the dock tree, be it a branch or a leaf.
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NodeId(u128);

impl NodeId {
    pub const ZERO: Self = Self(0);

    pub fn random() -> Self {
        use rand::Rng as _;
        Self(rand::thread_rng().gen())
    }

    /// Corresponding [`egui::Id`], used for dragging.
    pub fn id(&self) -> Id {
        Id::new(self)
    }
}

impl std::fmt::Debug for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08X}", self.0 as u32)
    }
}

/// The top level type. Contains all peristent state, including layouts and sizes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Node<Leaf> {
    Leaf(Leaf),
    Branch(Branch),
}

impl<Leaf> Node<Leaf> {
    fn layout(&self) -> Option<Layout> {
        match self {
            Node::Leaf(_) => None,
            Node::Branch(branch) => Some(branch.get_layout()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutInsertion {
    Tabs(usize),
    Horizontal(usize),
    Vertical(usize),
    Grid(GridLoc),
}

#[derive(Clone, Copy, Debug)]
struct InsertionPoint {
    parent_id: NodeId,

    /// Where in the parent?
    insertion: LayoutInsertion,
}

impl InsertionPoint {
    fn new(parent_id: NodeId, insertion: LayoutInsertion) -> Self {
        Self {
            parent_id,
            insertion,
        }
    }
}

// ----------------------------------------------------------------------------

#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiResponse {
    None,

    /// The viewer is being dragged via some element in the Leaf
    DragStarted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SimplificationOptions {
    pub prune_empty_tabs: bool,
    pub prune_single_child_tabs: bool,
    pub prune_empty_layouts: bool,
    pub prune_single_child_layouts: bool,
    pub all_leaves_must_have_tabs: bool,
}

impl Default for SimplificationOptions {
    fn default() -> Self {
        Self {
            prune_empty_tabs: true,
            prune_single_child_tabs: true,
            prune_empty_layouts: true,
            prune_single_child_layouts: true,
            all_leaves_must_have_tabs: false,
        }
    }
}

/// Trait defining how the [`Dock`] and its leaf should be shown.
pub trait Behavior<Leaf> {
    /// Show this leaf node in the given [`egui::Ui`].
    ///
    /// If this is an unknown node, return [`NodeAction::Remove`] and the node will be removed.
    fn leaf_ui(&mut self, _ui: &mut Ui, _node_id: NodeId, _leaf: &mut Leaf) -> UiResponse;

    fn tab_text_for_leaf(&mut self, leaf: &Leaf) -> WidgetText;

    fn tab_text_for_node(&mut self, nodes: &Nodes<Leaf>, node_id: NodeId) -> WidgetText {
        match &nodes.nodes[&node_id] {
            Node::Leaf(leaf) => self.tab_text_for_leaf(leaf),
            Node::Branch(branch) => format!("{:?}", branch.get_layout()).into(),
        }
    }

    fn tab_ui(
        &mut self,
        nodes: &Nodes<Leaf>,
        ui: &mut Ui,
        id: Id,
        node_id: NodeId,
        selected: bool,
        is_being_dragged: bool,
    ) -> Response {
        let text = self.tab_text_for_node(nodes, node_id);
        let font_id = TextStyle::Button.resolve(ui.style());
        let galley = text.into_galley(ui, Some(false), f32::INFINITY, font_id);
        let (_, rect) = ui.allocate_space(galley.size());
        let response = ui.interact(rect, id, Sense::click_and_drag());
        let widget_style = ui.style().interact_selectable(&response, selected);

        // Show a gap when dragged
        if ui.is_rect_visible(rect) && !is_being_dragged {
            if selected {
                ui.painter().rect_filled(rect, 0.0, widget_style.bg_fill);
            }
            ui.painter()
                .galley_with_color(rect.min, galley.galley, widget_style.text_color());
        }

        response
    }

    /// Returns `false` if this leaf should be removed from its parent.
    fn retain_leaf(&mut self, _leaf: &Leaf) -> bool {
        true
    }

    // ---
    // Settings:

    /// The height of the bar holding tab names.
    fn tab_bar_height(&self, _style: &egui::Style) -> f32 {
        20.0
    }

    /// Width of the gap between nodes in a horizontal or vertical layout
    fn gap_width(&self, _style: &egui::Style) -> f32 {
        1.0
    }

    // No child should shrink below this size
    fn min_size(&self) -> f32 {
        32.0
    }

    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions::default()
    }

    fn resize_stroke(&self, style: &egui::Style, resize_state: ResizeState) -> egui::Stroke {
        match resize_state {
            ResizeState::Idle => egui::Stroke::NONE, // Let the gap speak for itself
            ResizeState::Hovering => style.visuals.widgets.hovered.fg_stroke,
            ResizeState::Dragging => style.visuals.widgets.active.fg_stroke,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResizeState {
    Idle,
    Hovering,
    Dragging,
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
}

impl<Leaf> Nodes<Leaf> {
    pub fn rect(&self, node_id: NodeId) -> Option<Rect> {
        self.rects.get(&node_id).copied()
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
    pub fn insert_branch(&mut self, branch: Branch) -> NodeId {
        self.insert_node(Node::Branch(branch))
    }

    #[must_use]
    pub fn insert_leaf(&mut self, leaf: Leaf) -> NodeId {
        self.insert_node(Node::Leaf(leaf))
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

    fn parent(&self, it: NodeId, needle_child: NodeId) -> Option<NodeId> {
        match &self.nodes.get(&it)? {
            Node::Leaf(_) => None,
            Node::Branch(branch) => {
                for &child in branch.children() {
                    if child == needle_child {
                        return Some(it);
                    }
                    if let Some(parent) = self.parent(child, needle_child) {
                        return Some(parent);
                    }
                }
                None
            }
        }
    }

    /// Performs no simplifcations!
    fn remove_node_id_from_parent(&mut self, it: NodeId, remove: NodeId) -> GcAction {
        if it == remove {
            return GcAction::Remove;
        }
        let Some(mut node) = self.nodes.remove(&it) else { return GcAction::Remove; };
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
                    self.nodes.insert(parent_id, node);
                } else {
                    let new_node_id = self.insert_node(node);
                    let mut tabs = Tabs::new(vec![new_node_id]);
                    tabs.children.insert(index.min(1), child_id);
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
            &mut drop_context,
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
                    if let Some(parent_rect) = self.nodes.rect(insertion_point.parent_id) {
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
                if ctx.input(|i| i.key_pressed(Key::Escape)) {
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

fn is_possible_drag(ctx: &egui::Context) -> bool {
    ctx.input(|input| {
        !input.pointer.any_pressed()
            && !input.pointer.could_any_button_be_click()
            && !input.pointer.any_click()
    })
}

fn is_being_dragged(ctx: &egui::Context, node_id: NodeId) -> bool {
    ctx.memory(|mem| mem.is_being_dragged(node_id.id())) && is_possible_drag(ctx)
}

// ----------------------------------------------------------------------------
// gc

#[derive(PartialEq, Eq)]
enum GcAction {
    Keep,
    Remove,
}

impl<Leaf> Nodes<Leaf> {
    fn gc_root(&mut self, behavior: &mut dyn Behavior<Leaf>, root_id: NodeId) {
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
}

// ----------------------------------------------------------------------------
// layout

impl<Leaf> Nodes<Leaf> {
    fn layout_node(
        &mut self,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        rect: Rect,
        node_id: NodeId,
    ) {
        let Some(mut node) = self.nodes.remove(&node_id) else { return; };
        self.rects.insert(node_id, rect);

        if let Node::Branch(branch) = &mut node {
            branch.layout(self, style, behavior, drop_context, rect, node_id);
        }

        self.nodes.insert(node_id, node);
    }
}

fn sizes_from_shares(shares: &[f32], available_size: f32, gap_width: f32) -> Vec<f32> {
    assert!(!shares.is_empty());
    let available_size = available_size - gap_width * (shares.len() - 1) as f32;
    let available_size = available_size.at_least(0.0);

    let total_share: f32 = shares.iter().sum();
    if total_share <= 0.0 {
        vec![available_size / shares.len() as f32; shares.len()]
    } else {
        shares
            .iter()
            .map(|&share| share / total_share * available_size)
            .collect()
    }
}

// ----------------------------------------------------------------------------
// ui

pub struct DropContext {
    enabled: bool,
    dragged_node_id: Option<NodeId>,
    mouse_pos: Option<Pos2>,

    best_insertion: Option<InsertionPoint>,
    best_dist_sq: f32,
    preview_rect: Option<Rect>,
}

impl DropContext {
    fn on_node<Leaf>(&mut self, parent_id: NodeId, rect: Rect, node: &Node<Leaf>) {
        if !self.enabled {
            return;
        }

        if node.layout() != Some(Layout::Horizontal) {
            self.suggest_rect(
                InsertionPoint::new(parent_id, LayoutInsertion::Horizontal(0)),
                rect.split_left_right_at_fraction(0.5).0,
            );
            self.suggest_rect(
                InsertionPoint::new(parent_id, LayoutInsertion::Horizontal(usize::MAX)),
                rect.split_left_right_at_fraction(0.5).1,
            );
        }

        if node.layout() != Some(Layout::Vertical) {
            self.suggest_rect(
                InsertionPoint::new(parent_id, LayoutInsertion::Vertical(0)),
                rect.split_top_bottom_at_fraction(0.5).0,
            );
            self.suggest_rect(
                InsertionPoint::new(parent_id, LayoutInsertion::Vertical(usize::MAX)),
                rect.split_top_bottom_at_fraction(0.5).1,
            );
        }

        // self.suggest_rect(InsertionPoint::new(parent_id, LayoutType::Tabs, 1), rect);
    }

    fn suggest_rect(&mut self, insertion: InsertionPoint, preview_rect: Rect) {
        self.suggest_point(insertion, preview_rect.center(), preview_rect);
    }

    fn suggest_point(&mut self, insertion: InsertionPoint, target_point: Pos2, preview_rect: Rect) {
        if !self.enabled {
            return;
        }
        if let Some(mouse_pos) = self.mouse_pos {
            let dist_sq = mouse_pos.distance_sq(target_point);
            if dist_sq < self.best_dist_sq {
                self.best_dist_sq = dist_sq;
                self.best_insertion = Some(insertion);
                self.preview_rect = Some(preview_rect);
            }
        }
    }
}

impl<Leaf> Nodes<Leaf> {
    fn node_ui(
        &mut self,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut Ui,
        node_id: NodeId,
    ) {
        let (Some(rect), Some(mut node)) = (self.rect(node_id), self.nodes.remove(&node_id)) else { return };

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
}

// ----------------------------------------------------------------------------
// Simplification

#[must_use]
enum SimplifyAction {
    Remove,
    Keep,
    Replace(NodeId),
}

impl<Leaf> Nodes<Leaf> {
    fn simplify(&mut self, options: &SimplificationOptions, it: NodeId) -> SimplifyAction {
        let Some(mut node) = self.nodes.remove(&it) else { return SimplifyAction::Remove; };

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
}

impl<Leaf> Nodes<Leaf> {
    fn make_all_leaves_children_of_tabs(&mut self, parent_is_tabs: bool, it: NodeId) {
        let Some(mut node) = self.nodes.remove(&it) else { return; };

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
