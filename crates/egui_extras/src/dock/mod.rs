use std::collections::{HashMap, HashSet};

use egui::{pos2, vec2, CursorIcon, NumExt, Rect, WidgetText};

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
    // Vertical(Vertical)
    // Grid(Grid)
}

impl<Leaf> NodeLayout<Leaf> {
    pub fn name(&self) -> &'static str {
        match self {
            NodeLayout::Leaf(_) => "Leaf",
            NodeLayout::Tabs(_) => "Tabs",
            NodeLayout::Horizontal(_) => "Horizontal",
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
    fn leaf_ui(&mut self, _ui: &mut egui::Ui, _node_id: NodeId, _leaf: &mut Leaf);

    fn tab_text_for_leaf(&mut self, leaf: &Leaf) -> egui::WidgetText;

    fn tab_text_for_node(&mut self, nodes: &Nodes<Leaf>, node_id: NodeId) -> WidgetText {
        match &nodes.nodes[&node_id].layout {
            NodeLayout::Leaf(leaf) => self.tab_text_for_leaf(leaf),
            layout => layout.name().into(),
        }
    }

    fn tab_ui(
        &mut self,
        nodes: &Nodes<Leaf>,
        ui: &mut egui::Ui,
        node_id: NodeId,
        selected: bool,
    ) -> egui::Response {
        let text = self.tab_text_for_node(nodes, node_id);
        let font_id = egui::TextStyle::Button.resolve(ui.style());
        let galley = text.into_galley(ui, Some(false), f32::INFINITY, font_id);
        let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::click_and_drag());
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
    fn tab_bar_height(&self, _style: &egui::Style) -> f32 {
        20.0
    }

    /// Width of the gap between nodes in a horizontal or vertical layout
    fn gap_width(&self, _style: &egui::Style) -> f32 {
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

    pub fn get(&self, node_id: NodeId) -> Option<&NodeLayout<Leaf>> {
        self.nodes.get(&node_id).map(|node| &node.layout)
    }
}

// Usage
impl<Leaf> Dock<Leaf> {
    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn ui(&mut self, behavior: &mut dyn Behavior<Leaf>, ui: &mut egui::Ui) {
        self.nodes.gc_root(behavior, self.root);

        self.nodes.layout_node(
            ui.style(),
            behavior,
            ui.available_rect_before_wrap(),
            self.root,
        );

        self.nodes.node_ui(behavior, ui, self.root);
    }
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
            log::warn!("Cycle detected in egui_extras::dock");
            return GcAction::Remove;
        }

        match &mut node.layout {
            NodeLayout::Leaf(leaf) => {
                if !behavior.retain_leaf(leaf) {
                    return GcAction::Remove;
                }
            }
            NodeLayout::Tabs(layout) => {
                layout
                    .children
                    .retain(|&child| self.gc_node_id(behavior, visited, child) == GcAction::Keep);
            }
            NodeLayout::Horizontal(layout) => {
                layout
                    .children
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
        style: &egui::Style,
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
        }

        self.nodes.insert(node_id, node);
    }

    fn layout_tabs(
        &mut self,
        style: &egui::Style,
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
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
        rect: Rect,
        horizontal: &Horizontal,
    ) {
        if horizontal.children.is_empty() {
            return;
        }
        let num_gaps = horizontal.children.len() - 1;
        let gap_width = behavior.gap_width(style);
        let total_gap_width = gap_width * num_gaps as f32;
        let available_width = (rect.width() - total_gap_width).at_least(0.0);

        let widths = horizontal
            .shares
            .split(&horizontal.children, available_width);

        let mut x = rect.min.x;
        for (child, width) in horizontal.children.iter().zip(widths) {
            let child_rect = Rect::from_min_size(pos2(x, rect.min.y), vec2(width, rect.height()));
            self.layout_node(style, behavior, child_rect, *child);
            x += width + gap_width;
        }
    }
}

// ----------------------------------------------------------------------------
// ui

impl<Leaf> Nodes<Leaf> {
    fn node_ui(&mut self, behavior: &mut dyn Behavior<Leaf>, ui: &mut egui::Ui, node_id: NodeId) {
        let Some(mut node) = self.nodes.remove(&node_id) else { return };

        match &mut node.layout {
            NodeLayout::Leaf(leaf) => {
                let mut leaf_ui = ui.child_ui(node.rect, *ui.layout());
                behavior.leaf_ui(&mut leaf_ui, node_id, leaf);
            }
            NodeLayout::Tabs(tabs) => self.tabs_ui(behavior, ui, node.rect, tabs),
            NodeLayout::Horizontal(horizontal) => {
                self.horizontal_ui(behavior, ui, node.rect, horizontal);
            }
        };
        self.nodes.insert(node_id, node);
    }

    fn tabs_ui(
        &mut self,
        behavior: &mut dyn Behavior<Leaf>,
        ui: &mut egui::Ui,
        rect: Rect,
        tabs: &mut Tabs,
    ) {
        let tab_bar_height = behavior.tab_bar_height(ui.style());
        let tab_bar_rect = {
            let mut r = rect;
            r.max.y = r.min.y + tab_bar_height;
            r
        };
        let mut tab_bar_ui = ui.child_ui(tab_bar_rect, *ui.layout());

        // Show tab bar:
        tab_bar_ui.horizontal(|ui| {
            for &child_id in &tabs.children {
                let selected = child_id == tabs.active;
                let response = behavior.tab_ui(self, ui, child_id, selected);
                let response = response.on_hover_cursor(CursorIcon::Grab);
                if response.clicked() {
                    tabs.active = child_id;
                }
            }
        });

        self.node_ui(behavior, ui, tabs.active);
    }

    fn horizontal_ui(
        &mut self,
        behavior: &mut dyn Behavior<Leaf>,
        ui: &mut egui::Ui,
        _rect: Rect,
        horizontal: &mut Horizontal,
    ) {
        for child in &horizontal.children {
            self.node_ui(behavior, ui, *child);
        }
    }
}
