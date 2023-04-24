use std::collections::{HashMap, HashSet};

use egui::{
    pos2, vec2, Color32, CursorIcon, Id, Key, NumExt, Pos2, Rect, Response, Sense, Style,
    TextStyle, Ui, WidgetText,
};

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
}

impl<Leaf> Default for Dock<Leaf> {
    fn default() -> Self {
        Self {
            root: Default::default(),
            nodes: Default::default(),
        }
    }
}

/// Contains all node state, but no root.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Nodes<Leaf> {
    pub nodes: HashMap<NodeId, NodeState<Leaf>>,
}

impl<Leaf> Default for Nodes<Leaf> {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NodeState<Leaf> {
    pub layout: NodeLayout<Leaf>,

    /// Filled in by the layout step at the start of each frame.
    #[serde(skip)]
    #[serde(default = "nan_rect")]
    pub rect: Rect,
}

fn nan_rect() -> Rect {
    Rect::NAN
}

impl<Leaf> From<NodeLayout<Leaf>> for NodeState<Leaf> {
    fn from(layout: NodeLayout<Leaf>) -> Self {
        Self {
            layout,
            rect: Rect::NAN,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum NodeLayout<Leaf> {
    Leaf(Leaf),
    Tabs(Tabs),
    Horizontal(Horizontal),
    Vertical(Vertical),
}

impl<Leaf> NodeLayout<Leaf> {
    pub fn name(&self) -> &'static str {
        match self {
            NodeLayout::Leaf(_) => "Leaf",
            NodeLayout::Tabs(_) => "Tabs",
            NodeLayout::Horizontal(_) => "Horizontal",
            NodeLayout::Vertical(_) => "Vertical",
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Tabs {
    pub children: Vec<NodeId>,
    pub active: NodeId,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Horizontal {
    pub children: Vec<NodeId>,
    pub shares: Shares,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Vertical {
    pub children: Vec<NodeId>,
    pub shares: Shares,
}

/// How large of a share of space each child has, on a 1D axis.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Shares {
    /// How large of a share each child has.
    ///
    /// For instance, the shares `[1, 2, 3]` means that the first child gets 1/6 of the space,
    /// the second gets 2/6 and the third gets 3/6.
    pub shares: HashMap<NodeId, f32>,
}

impl Shares {
    fn split(&self, children: &[NodeId], available_width: f32) -> Vec<f32> {
        let mut num_shares = 0.0;
        for child in children {
            num_shares += self.shares.get(child).copied().unwrap_or(1.0);
        }
        if num_shares == 0.0 {
            num_shares = 1.0;
        }
        children
            .iter()
            .map(|child| {
                available_width * self.shares.get(child).copied().unwrap_or(1.0) / num_shares
            })
            .collect()
    }
}

impl<Leaf> From<Tabs> for NodeLayout<Leaf> {
    fn from(tabs: Tabs) -> Self {
        NodeLayout::Tabs(tabs)
    }
}

impl<Leaf> From<Horizontal> for NodeLayout<Leaf> {
    fn from(horizontal: Horizontal) -> Self {
        NodeLayout::Horizontal(horizontal)
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
        match &nodes.nodes[&node_id].layout {
            NodeLayout::Leaf(leaf) => self.tab_text_for_leaf(leaf),
            layout => layout.name().into(),
        }
    }

    fn tab_ui(
        &mut self,
        nodes: &Nodes<Leaf>,
        ui: &mut Ui,
        id: Id,
        node_id: NodeId,
        selected: bool,
    ) -> Response {
        let text = self.tab_text_for_node(nodes, node_id);
        let font_id = TextStyle::Button.resolve(ui.style());
        let galley = text.into_galley(ui, Some(false), f32::INFINITY, font_id);
        let (_, rect) = ui.allocate_space(galley.size());
        let response = ui.interact(rect, id, Sense::click_and_drag());
        let widget_style = ui.style().interact_selectable(&response, selected);
        ui.painter()
            .galley_with_color(rect.min, galley.galley, widget_style.text_color());
        response
    }

    /// Returns `false` if this leaf should be removed from its parent.
    fn retain_leaf(&mut self, _leaf: &Leaf) -> bool {
        true
    }

    // ---
    // Settings:

    /// The height of the bar holding tab names.
    fn tab_bar_height(&self, _style: &Style) -> f32 {
        20.0
    }

    /// Width of the gap between nodes in a horizontal or vertical layout
    fn gap_width(&self, _style: &Style) -> f32 {
        1.0
    }

    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions::default()
    }
}

// ----------------------------------------------------------------------------
// Construction

impl<Leaf> Dock<Leaf> {
    pub fn new(root: NodeId, nodes: Nodes<Leaf>) -> Self {
        Self { root, nodes }
    }
}

impl<Leaf> Nodes<Leaf> {
    pub fn get(&self, node_id: NodeId) -> Option<&NodeLayout<Leaf>> {
        self.nodes.get(&node_id).map(|node| &node.layout)
    }

    pub fn get_mut(&mut self, node_id: NodeId) -> Option<&mut NodeLayout<Leaf>> {
        self.nodes.get_mut(&node_id).map(|node| &mut node.layout)
    }

    #[must_use]
    pub fn insert_node(&mut self, node: NodeState<Leaf>) -> NodeId {
        let id = NodeId::random();
        self.nodes.insert(id, node);
        id
    }

    #[must_use]
    pub fn insert_leaf(&mut self, leaf: Leaf) -> NodeId {
        self.insert_node(NodeLayout::Leaf(leaf).into())
    }

    #[must_use]
    pub fn insert_tab_node(&mut self, children: Vec<NodeId>) -> NodeId {
        let tabs = Tabs {
            active: children.first().copied().unwrap_or_default(),
            children,
        };
        self.insert_node(NodeLayout::Tabs(tabs).into())
    }

    #[must_use]
    pub fn insert_horizontal_node(&mut self, children: Vec<NodeId>) -> NodeId {
        let horizontal = Horizontal {
            children,
            shares: Default::default(),
        };
        self.insert_node(NodeLayout::Horizontal(horizontal).into())
    }

    fn parent(&self, it: NodeId, needle_child: NodeId) -> Option<NodeId> {
        match &self.nodes.get(&it)?.layout {
            NodeLayout::Leaf(_) => None,
            NodeLayout::Tabs(Tabs { children, .. })
            | NodeLayout::Horizontal(Horizontal { children, .. })
            | NodeLayout::Vertical(Vertical { children, .. }) => {
                for &child in children {
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
        match &mut node.layout {
            NodeLayout::Leaf(_) => {}
            NodeLayout::Tabs(Tabs { children, .. }) => {
                children.retain(|&child| {
                    self.remove_node_id_from_parent(child, remove) == GcAction::Keep
                });
            }
            NodeLayout::Horizontal(Horizontal { children, .. })
            | NodeLayout::Vertical(Vertical { children, .. }) => {
                children.retain(|&child| {
                    self.remove_node_id_from_parent(child, remove) == GcAction::Keep
                });
            }
        }
        self.nodes.insert(it, node);
        GcAction::Keep
    }

    fn insert(&mut self, insertion_point: InsertionPoint, child_id: NodeId) {
        let InsertionPoint {
            parent_id,
            layout_type,
            index,
        } = insertion_point;
        let Some(mut node) = self.nodes.remove(&parent_id) else {
            log::warn!("Failed to insert: could not find parent {parent_id:?}");
            return;
        };
        match layout_type {
            LayoutType::Tabs => {
                if let NodeLayout::Tabs(layout) = &mut node.layout {
                    let index = index.min(layout.children.len());
                    layout.children.insert(index, child_id);
                    self.nodes.insert(parent_id, node);
                } else {
                    let new_node_id = self.insert_node(node);
                    let mut layout = Tabs {
                        children: vec![new_node_id],
                        active: new_node_id,
                    };
                    layout.children.insert(index.min(1), child_id);
                    self.nodes
                        .insert(parent_id, NodeLayout::Tabs(layout).into());
                }
            }
            LayoutType::Horizontal => {
                if let NodeLayout::Horizontal(layout) = &mut node.layout {
                    let index = index.min(layout.children.len());
                    layout.children.insert(index, child_id);
                    self.nodes.insert(parent_id, node);
                } else {
                    let new_node_id = self.insert_node(node);
                    let mut layout = Horizontal {
                        children: vec![new_node_id],
                        shares: Default::default(),
                    };
                    layout.children.insert(index.min(1), child_id);
                    self.nodes
                        .insert(parent_id, NodeLayout::Horizontal(layout).into());
                }
            }
            LayoutType::Vertical => {
                if let NodeLayout::Vertical(layout) = &mut node.layout {
                    let index = index.min(layout.children.len());
                    layout.children.insert(index, child_id);
                    self.nodes.insert(parent_id, node);
                } else {
                    let new_node_id = self.insert_node(node);
                    let mut layout = Vertical {
                        children: vec![new_node_id],
                        shares: Default::default(),
                    };
                    layout.children.insert(index.min(1), child_id);
                    self.nodes
                        .insert(parent_id, NodeLayout::Vertical(layout).into());
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

    pub fn ui(&mut self, behavior: &mut dyn Behavior<Leaf>, ui: &mut Ui) {
        let options = behavior.simplification_options();
        self.simplify(&options);
        if options.all_leaves_must_have_tabs {
            self.nodes
                .make_all_leaves_children_of_tabs(false, self.root);
        }
        self.nodes.gc_root(behavior, self.root);

        self.nodes.layout_node(
            ui.style(),
            behavior,
            ui.available_rect_before_wrap(),
            self.root,
        );

        // Check if anything is being dragged:
        let mut drop_context = DropContext {
            active: true,
            dragged_node_id: self.dragged_id(ui.ctx()),
            mouse_pos: ui.input(|i| i.pointer.hover_pos()),
            best_dist_sq: f32::INFINITY,
            best_insertion: None,
            preview_rect: None,
        };

        self.nodes
            .node_ui(behavior, &mut drop_context, ui, self.root);

        if let (Some(mouse_pos), Some(dragged_node_id)) =
            (drop_context.mouse_pos, drop_context.dragged_node_id)
        {
            ui.output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);

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
                ui.painter().rect(
                    preview_rect,
                    1.0,
                    Color32::LIGHT_BLUE.gamma_multiply(0.5),
                    (1.0, Color32::LIGHT_BLUE),
                );

                let preview_child = false;
                if preview_child {
                    // Preview actual child?
                    if preview_rect.width() > 32.0 && preview_rect.height() > 32.0 {
                        if let Some(NodeLayout::Leaf(leaf)) = self.nodes.get_mut(dragged_node_id) {
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
            }
        }
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
            insertion_point.parent_id
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

        match &mut node.layout {
            NodeLayout::Leaf(leaf) => {
                if !behavior.retain_leaf(leaf) {
                    return GcAction::Remove;
                }
            }
            NodeLayout::Tabs(Tabs { children, .. })
            | NodeLayout::Horizontal(Horizontal { children, .. })
            | NodeLayout::Vertical(Vertical { children, .. }) => {
                children
                    .retain(|&child| self.gc_node_id(behavior, visited, child) == GcAction::Keep);
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
        style: &Style,
        behavior: &mut dyn Behavior<Leaf>,
        rect: Rect,
        node_id: NodeId,
    ) {
        let Some(mut node) = self.nodes.remove(&node_id) else { return; };
        node.rect = rect;

        match &node.layout {
            NodeLayout::Leaf(_) => {}
            NodeLayout::Tabs(tabs) => {
                self.layout_tabs(style, behavior, rect, tabs);
            }
            NodeLayout::Horizontal(horizontal) => {
                self.layout_horizontal(style, behavior, rect, horizontal);
            }
            NodeLayout::Vertical(vertical) => {
                self.layout_vertical(style, behavior, rect, vertical);
            }
        }

        self.nodes.insert(node_id, node);
    }

    fn layout_tabs(
        &mut self,
        style: &Style,
        behavior: &mut dyn Behavior<Leaf>,
        rect: Rect,
        tabs: &Tabs,
    ) {
        let mut active_rect = rect;
        active_rect.min.y += behavior.tab_bar_height(style);

        if false {
            self.layout_node(style, behavior, active_rect, tabs.active);
        } else {
            // Layout all nodes in case the user switches active tab
            for &child_id in &tabs.children {
                self.layout_node(style, behavior, active_rect, child_id);
            }
        }
    }

    fn layout_horizontal(
        &mut self,
        style: &Style,
        behavior: &mut dyn Behavior<Leaf>,
        rect: Rect,
        layout: &Horizontal,
    ) {
        if layout.children.is_empty() {
            return;
        }
        let num_gaps = layout.children.len() - 1;
        let gap_width = behavior.gap_width(style);
        let total_gap_width = gap_width * num_gaps as f32;
        let available_width = (rect.width() - total_gap_width).at_least(0.0);

        let widths = layout.shares.split(&layout.children, available_width);

        let mut x = rect.min.x;
        for (child, width) in layout.children.iter().zip(widths) {
            let child_rect = Rect::from_min_size(pos2(x, rect.min.y), vec2(width, rect.height()));
            self.layout_node(style, behavior, child_rect, *child);
            x += width + gap_width;
        }
    }

    fn layout_vertical(
        &mut self,
        style: &Style,
        behavior: &mut dyn Behavior<Leaf>,
        rect: Rect,
        layout: &Vertical,
    ) {
        if layout.children.is_empty() {
            return;
        }
        let num_gaps = layout.children.len() - 1;
        let gap_height = behavior.gap_width(style);
        let total_gap_height = gap_height * num_gaps as f32;
        let available_height = (rect.height() - total_gap_height).at_least(0.0);

        let heights = layout.shares.split(&layout.children, available_height);

        let mut y = rect.min.y;
        for (child, height) in layout.children.iter().zip(heights) {
            let child_rect = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), height));
            self.layout_node(style, behavior, child_rect, *child);
            y += height + gap_height;
        }
    }
}

// ----------------------------------------------------------------------------
// ui

enum LayoutType {
    Tabs,
    Horizontal,
    Vertical,
}

struct InsertionPoint {
    parent_id: NodeId,

    layout_type: LayoutType,

    /// Where in the parent?
    index: usize,
}

impl InsertionPoint {
    fn new(parent_id: NodeId, layout_type: LayoutType, index: usize) -> Self {
        Self {
            parent_id,
            layout_type,
            index,
        }
    }
}

struct DropContext {
    active: bool,
    dragged_node_id: Option<NodeId>,
    mouse_pos: Option<Pos2>,

    best_insertion: Option<InsertionPoint>,
    best_dist_sq: f32,
    preview_rect: Option<Rect>,
}

impl DropContext {
    fn on_node<Leaf>(&mut self, parent_id: NodeId, node: &NodeState<Leaf>) {
        if !self.active {
            return;
        }
        let rect = node.rect;

        if !matches!(node.layout, NodeLayout::Horizontal(_)) {
            self.suggest_rect(
                InsertionPoint::new(parent_id, LayoutType::Horizontal, 0),
                rect.split_left_right_at_fraction(0.5).0,
            );
            self.suggest_rect(
                InsertionPoint::new(parent_id, LayoutType::Horizontal, usize::MAX),
                rect.split_left_right_at_fraction(0.5).1,
            );
        }

        if !matches!(node.layout, NodeLayout::Vertical(_)) {
            self.suggest_rect(
                InsertionPoint::new(parent_id, LayoutType::Vertical, 0),
                rect.split_top_bottom_at_fraction(0.5).0,
            );
            self.suggest_rect(
                InsertionPoint::new(parent_id, LayoutType::Vertical, usize::MAX),
                rect.split_top_bottom_at_fraction(0.5).1,
            );
        }

        // self.suggest_rect(InsertionPoint::new(parent_id, LayoutType::Tabs, 1), rect);
    }

    fn suggest_rect(&mut self, insertion: InsertionPoint, preview_rect: Rect) {
        self.suggest_point(insertion, preview_rect.center(), preview_rect);
    }

    fn suggest_point(&mut self, insertion: InsertionPoint, target_point: Pos2, preview_rect: Rect) {
        if !self.active {
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
        let Some(mut node) = self.nodes.remove(&node_id) else { return };

        let drop_context_was_active = drop_context.active;
        if Some(node_id) == drop_context.dragged_node_id {
            // Can't drag a node onto self or any children
            drop_context.active = false;
        }
        drop_context.on_node(node_id, &node);

        drop_context.suggest_rect(
            InsertionPoint::new(node_id, LayoutType::Tabs, usize::MAX),
            node.rect
                .split_top_bottom_at_y(node.rect.top() + behavior.tab_bar_height(ui.style()))
                .1,
        );

        match &mut node.layout {
            NodeLayout::Leaf(leaf) => {
                let mut leaf_ui = ui.child_ui(node.rect, *ui.layout());
                if behavior.leaf_ui(&mut leaf_ui, node_id, leaf) == UiResponse::DragStarted {
                    ui.memory_mut(|mem| mem.set_dragged_id(node_id.id()));
                }
            }
            NodeLayout::Tabs(tabs) => {
                self.tabs_ui(behavior, drop_context, ui, node.rect, node_id, tabs);
            }
            NodeLayout::Horizontal(horizontal) => {
                self.horizontal_ui(behavior, drop_context, ui, node_id, horizontal);
            }
            NodeLayout::Vertical(vertical) => {
                self.vertical_ui(behavior, drop_context, ui, node_id, vertical);
            }
        };

        self.nodes.insert(node_id, node);
        drop_context.active = drop_context_was_active;
    }

    fn tabs_ui(
        &mut self,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut Ui,
        rect: Rect,
        node_id: NodeId,
        tabs: &mut Tabs,
    ) {
        if !tabs.children.iter().any(|&child| child == tabs.active) {
            // Make sure something is active:
            tabs.active = tabs.children.first().copied().unwrap_or_default();
        }

        let tab_bar_height = behavior.tab_bar_height(ui.style());
        let tab_bar_rect = rect.split_top_bottom_at_y(rect.top() + tab_bar_height).0;
        let mut tab_bar_ui = ui.child_ui(tab_bar_rect, *ui.layout());

        // Show tab bar:
        tab_bar_ui.horizontal(|ui| {
            let mut prev_tab_rect: Option<Rect> = None;
            let mut insertion_index = 0; // skips over drag-source, if any, beacuse it will be removed then re-inserted

            for (i, &child_id) in tabs.children.iter().enumerate() {
                if is_being_dragged(ui.ctx(), child_id) {
                    continue; // leave a gap!
                }

                let selected = child_id == tabs.active;
                let id = child_id.id();

                let response = behavior.tab_ui(self, ui, id, child_id, selected);
                let response = response.on_hover_cursor(CursorIcon::Grab);
                if response.clicked() {
                    tabs.active = child_id;
                }

                if let Some(mouse_pos) = drop_context.mouse_pos {
                    if drop_context.dragged_node_id.is_some() && response.rect.contains(mouse_pos) {
                        // Expand this tab - maybe the user wants to drop something into it!
                        tabs.active = child_id;
                    }
                }

                let rect = response.rect;

                {
                    // suggest dropping before this tab:
                    let before_point = if let Some(prev_tab_rect) = prev_tab_rect {
                        // between
                        prev_tab_rect.right_center().lerp(rect.left_center(), 0.5)
                    } else {
                        // before first
                        rect.left_center()
                    };

                    drop_context.suggest_rect(
                        InsertionPoint::new(node_id, LayoutType::Tabs, insertion_index),
                        Rect::from_center_size(before_point, vec2(4.0, rect.height())),
                    );
                }

                if i + 1 == tabs.children.len() {
                    // suggest dropping after last tab:
                    drop_context.suggest_rect(
                        InsertionPoint::new(node_id, LayoutType::Tabs, insertion_index + 1),
                        Rect::from_center_size(rect.right_center(), vec2(4.0, rect.height())),
                    );
                }

                prev_tab_rect = Some(rect);
                insertion_index += 1;
            }
        });

        // When dragged, don't show it (it is "being held")
        let is_active_being_dragged =
            ui.memory(|mem| mem.is_being_dragged(tabs.active.id())) && is_possible_drag(ui.ctx());
        if !is_active_being_dragged {
            self.node_ui(behavior, drop_context, ui, tabs.active);
        }
    }

    fn horizontal_ui(
        &mut self,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut Ui,
        parent_id: NodeId,
        horizontal: &mut Horizontal,
    ) {
        let mut prev_rect: Option<Rect> = None;
        let mut insertion_index = 0; // skips over drag-source, if any, beacuse it will be removed then re-inserted

        for (i, &child) in horizontal.children.iter().enumerate() {
            let rect = self.nodes[&child].rect;

            if is_being_dragged(ui.ctx(), child) {
                // suggest self as drop-target:
                drop_context.suggest_rect(
                    InsertionPoint::new(parent_id, LayoutType::Horizontal, i),
                    rect,
                );
            } else {
                self.node_ui(behavior, drop_context, ui, child);

                if let Some(prev_rect) = prev_rect {
                    // Suggest dropping between the rects:
                    drop_context.suggest_rect(
                        InsertionPoint::new(parent_id, LayoutType::Horizontal, insertion_index),
                        Rect::from_min_max(prev_rect.center_top(), rect.center_bottom()),
                    );
                } else {
                    // Suggest dropping before the first child:
                    drop_context.suggest_rect(
                        InsertionPoint::new(parent_id, LayoutType::Horizontal, 0),
                        rect.split_left_right_at_fraction(0.66).0,
                    );
                }

                if i + 1 == horizontal.children.len() {
                    // Suggest dropping after the last child:
                    drop_context.suggest_rect(
                        InsertionPoint::new(parent_id, LayoutType::Horizontal, insertion_index + 1),
                        rect.split_left_right_at_fraction(0.33).1,
                    );
                }
                insertion_index += 1;
            }

            prev_rect = Some(rect);
        }
    }

    fn vertical_ui(
        &mut self,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut Ui,
        parent_id: NodeId,
        vertical: &mut Vertical,
    ) {
        for child in &vertical.children {
            self.node_ui(behavior, drop_context, ui, *child);
        }
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

        match &mut node.layout {
            NodeLayout::Leaf(_) => {}
            NodeLayout::Tabs(Tabs { children, .. }) => {
                children.retain_mut(|child| match self.simplify(options, *child) {
                    SimplifyAction::Remove => false,
                    SimplifyAction::Keep => true,
                    SimplifyAction::Replace(new) => {
                        *child = new;
                        true
                    }
                });

                if options.prune_empty_tabs && children.is_empty() {
                    log::debug!("Simplify: removing empty tabs node");
                    return SimplifyAction::Remove;
                }
                if options.prune_single_child_tabs && children.len() == 1 {
                    if options.all_leaves_must_have_tabs
                        && matches!(self.get(children[0]), Some(NodeLayout::Leaf(_)))
                    {
                        // Keep it
                    } else {
                        log::debug!("Simplify: collapsing single-child tabs node");
                        return SimplifyAction::Replace(children[0]);
                    }
                }
            }

            NodeLayout::Horizontal(Horizontal { children, .. })
            | NodeLayout::Vertical(Vertical { children, .. }) => {
                // TODO: join nested versions of the same thing
                children.retain_mut(|child| match self.simplify(options, *child) {
                    SimplifyAction::Remove => false,
                    SimplifyAction::Keep => true,
                    SimplifyAction::Replace(new) => {
                        *child = new;
                        true
                    }
                });

                if options.prune_empty_layouts && children.is_empty() {
                    log::debug!("Simplify: removing empty layout node");
                    return SimplifyAction::Remove;
                }
                if options.prune_single_child_layouts && children.len() == 1 {
                    log::debug!("Simplify: collapsing single-child layout node");
                    return SimplifyAction::Replace(children[0]);
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

        match &mut node.layout {
            NodeLayout::Leaf(_) => {
                if !parent_is_tabs {
                    // Add tabs to this leaf:
                    let new_id = NodeId::random();
                    self.nodes.insert(new_id, node);
                    let tabs = NodeState::from(NodeLayout::Tabs(Tabs {
                        children: vec![new_id],
                        active: new_id,
                    }));
                    self.nodes.insert(it, tabs);
                    return;
                }
            }
            NodeLayout::Tabs(Tabs { children, .. }) => {
                for child in children {
                    self.make_all_leaves_children_of_tabs(true, *child);
                }
            }

            NodeLayout::Horizontal(Horizontal { children, .. })
            | NodeLayout::Vertical(Vertical { children, .. }) => {
                for child in children {
                    self.make_all_leaves_children_of_tabs(false, *child);
                }
            }
        }

        self.nodes.insert(it, node);
    }
}
