use std::collections::{HashMap, HashSet};

use egui::{
    pos2, vec2, Color32, CursorIcon, Id, Key, NumExt, Pos2, Rect, Response, Sense, Style,
    TextStyle, Ui, WidgetText,
};

// ----------------------------------------------------------------------------
// Types required for state

/// An identifier for a node in the dock tree, be it a branch or a leaf.
#[derive(
    Clone, Copy, Debug, Default, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeAction {
    Keep,
    Remove,
}

/// Trait defining how the [`Dock`] and its leaf should be shown.
pub trait Behavior<Leaf> {
    /// Show this leaf node in the given [`egui::Ui`].
    ///
    /// If this is an unknown node, return [`NodeAction::Remove`] and the node will be removed.
    fn leaf_ui(&mut self, _ui: &mut Ui, _node_id: NodeId, _leaf: &mut Leaf);

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

    fn remove_node_id_from_parent(&mut self, it: NodeId, remove: NodeId) {
        let Some(mut node) = self.nodes.remove(&it) else { return; };
        match &mut node.layout {
            NodeLayout::Leaf(_) => {}
            NodeLayout::Tabs(Tabs { children, .. })
            | NodeLayout::Horizontal(Horizontal { children, .. })
            | NodeLayout::Vertical(Vertical { children, .. }) => {
                children.retain(|&child| {
                    self.remove_node_id_from_parent(child, remove);
                    child != remove
                });
            }
        }
        self.nodes.insert(it, node);
    }

    fn insert(&mut self, insertion_point: InsertionPoint, child: NodeId) {
        let InsertionPoint {
            parent_id,
            layout_type,
            index,
        } = insertion_point;
        let Some(mut node) = self.nodes.remove(&parent_id) else {
            #[cfg(feature = "log")]
            log::warn!("failed to insert");
            return;
        };
        match layout_type {
            LayoutType::Tabs => {
                if let NodeLayout::Tabs(layout) = &mut node.layout {
                    let index = index.min(layout.children.len());
                    layout.children.insert(index, child);
                    self.nodes.insert(parent_id, node);
                } else {
                    let new_node_id = self.insert_node(node);
                    let mut layout = Tabs {
                        children: vec![new_node_id],
                        active: new_node_id,
                    };
                    layout.children.insert(index.min(1), child);
                    self.nodes
                        .insert(parent_id, NodeLayout::Tabs(layout).into());
                }
            }
            LayoutType::Horizontal => {
                if let NodeLayout::Horizontal(layout) = &mut node.layout {
                    let index = index.min(layout.children.len());
                    layout.children.insert(index, child);
                    self.nodes.insert(parent_id, node);
                } else {
                    let new_node_id = self.insert_node(node);
                    let mut layout = Horizontal {
                        children: vec![new_node_id],
                        shares: Default::default(),
                    };
                    layout.children.insert(index.min(1), child);
                    self.nodes
                        .insert(parent_id, NodeLayout::Horizontal(layout).into());
                }
            }
            LayoutType::Vertical => {
                if let NodeLayout::Vertical(layout) = &mut node.layout {
                    let index = index.min(layout.children.len());
                    layout.children.insert(index, child);
                    self.nodes.insert(parent_id, node);
                } else {
                    let new_node_id = self.insert_node(node);
                    let mut layout = Vertical {
                        children: vec![new_node_id],
                        shares: Default::default(),
                    };
                    layout.children.insert(index.min(1), child);
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
            // Preview what is being dragged:
            egui::Area::new(Id::new((dragged_node_id, "preview")))
                .pivot(egui::Align2::CENTER_CENTER)
                .current_pos(mouse_pos)
                .interactable(false)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
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
            }

            if ui.input(|i| i.pointer.any_released()) {
                ui.memory_mut(|mem| mem.stop_dragging());
                if let Some(insertion_point) = drop_context.best_insertion {
                    self.remove_node_id_from_parent(dragged_node_id);
                    self.nodes.insert(insertion_point, dragged_node_id);
                }
            }
        }
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

    fn remove_node_id_from_parent(&mut self, dragged_node_id: NodeId) {
        self.nodes
            .remove_node_id_from_parent(self.root, dragged_node_id);
    }
}

fn is_possible_drag(ctx: &egui::Context) -> bool {
    ctx.input(|i| !i.pointer.could_any_button_be_click() && !i.pointer.any_pressed())
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

        #[cfg(feature = "log")]
        if visited.len() < self.nodes.len() {
            log::warn!("GC collection {} nodes", self.nodes.len() - visited.len());
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
            #[cfg(feature = "log")]
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

struct DropContext {
    active: bool,
    dragged_node_id: Option<NodeId>,
    mouse_pos: Option<Pos2>,

    best_insertion: Option<InsertionPoint>,
    best_dist_sq: f32,
    preview_rect: Option<Rect>,
}

impl DropContext {
    fn on_node(&mut self, parent_id: NodeId, rect: Rect) {
        self.suggest_point(
            InsertionPoint {
                parent_id,
                layout_type: LayoutType::Horizontal,
                index: 0,
            },
            rect.left_center(),
            rect.split_left_right_at_fraction(0.5).0,
        );
        self.suggest_point(
            InsertionPoint {
                parent_id,
                layout_type: LayoutType::Horizontal,
                index: usize::MAX,
            },
            rect.right_center(),
            rect.split_left_right_at_fraction(0.5).1,
        );
        self.suggest_point(
            InsertionPoint {
                parent_id,
                layout_type: LayoutType::Vertical,
                index: 0,
            },
            rect.center_top(),
            rect.split_top_bottom_at_fraction(0.5).0,
        );
        self.suggest_point(
            InsertionPoint {
                parent_id,
                layout_type: LayoutType::Vertical,
                index: usize::MAX,
            },
            rect.center_bottom(),
            rect.split_top_bottom_at_fraction(0.5).1,
        );
        self.suggest_point(
            InsertionPoint {
                parent_id,
                layout_type: LayoutType::Tabs,
                index: 1,
            },
            rect.center(),
            rect,
        );
    }

    fn suggest_point(&mut self, insertion: InsertionPoint, target_point: Pos2, preview_rect: Rect) {
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
            drop_context.active = true;
        }
        drop_context.on_node(node_id, node.rect);

        match &mut node.layout {
            NodeLayout::Leaf(leaf) => {
                let mut leaf_ui = ui.child_ui(node.rect, *ui.layout());
                behavior.leaf_ui(&mut leaf_ui, node_id, leaf);
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
        parent_id: NodeId,
        tabs: &mut Tabs,
    ) {
        let tab_bar_height = behavior.tab_bar_height(ui.style());
        let tab_bar_rect = rect.split_top_bottom_at_y(rect.top() + tab_bar_height).0;
        let mut tab_bar_ui = ui.child_ui(tab_bar_rect, *ui.layout());

        // Show tab bar:
        tab_bar_ui.horizontal(|ui| {
            for &child_id in &tabs.children {
                let selected = child_id == tabs.active;
                let id = child_id.id();

                // let is_node_being_dragged = ui.memory(|mem| mem.is_being_dragged(id))
                //     && is_possible_drag(ui.ctx());
                // if is_node_being_dragged {
                //     continue; // leave a gap!
                // }

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
            }
        });

        self.node_ui(behavior, drop_context, ui, tabs.active);
    }

    fn horizontal_ui(
        &mut self,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut Ui,
        parent_id: NodeId,
        horizontal: &mut Horizontal,
    ) {
        for child in &horizontal.children {
            self.node_ui(behavior, drop_context, ui, *child);
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
