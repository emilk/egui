//! # Dock
//! Tabs that can be dragged around and split up in horizontal, vertical, and grid-layouts.
//!
//! ## Overview
//! The user add leaves to a [`Dock`], arranged using [`Branch`]es.
//! This forms a layout tree.
//! Everything is generic over the type of leaves, leaving up to the user what to store in the tree.
//!
//! Each [`Node]` is either a `Leaf` or a [`Branch`].
//! Each [`Node`] is identified by a (random) [`NodeId`].
//! The nodes are stored in [`Nodes`].
//!
//! The entire state is stored in a single [`Dock`] struct which consists of a [`Nodes`] and a root [`NodeId`].
//!
//! The behavior and the look of the dock is controlled by the [`Behavior`] `trait`.
//! The user needs to implement this in order to specify the `ui` of each `Leaf` and
//! the tab name of leaves (if there are tab nodes).
//!
//! ## Implementation notes
//! In many places we want to recursively visit all noted, while also mutating them.
//! In order to not get into trouble with the borrow checker a trick is used:
//! each [`Node`] is removed, mutated, recursed, and then re-added.
//! You'll see this pattern many times reading the following code.
//!
//! ## Shortcomings
//! We use real recursion, so if your trees get too deep you will get a stack overflow.
//!
//! ## Future improvements
//! * A new ui for each node, nested
//! * Per-tab close-buttons
//! * Scrolling of tab-bar
//! * Vertical tab bar
//! * Auto-grid layouts (re-arange as parent is resized)

use egui::{Id, Pos2, Rect};

mod behavior;
mod branch;
mod dock_struct;
mod nodes;

pub use behavior::Behavior;
pub use branch::{Branch, Grid, GridLoc, Layout, Linear, LinearDir, Tabs};
pub use dock_struct::Dock;
pub use nodes::Nodes;

// ----------------------------------------------------------------------------

/// An identifier for a [`Node`] in the dock tree, be it a branch or a leaf.
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NodeId(u128);

impl NodeId {
    /// Generate a new random [`NodeId`].
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

// ----------------------------------------------------------------------------

/// A node in the tree. Either a leaf or a [`Branch`].
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResizeState {
    Idle,
    Hovering,
    Dragging,
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LayoutInsertion {
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

#[derive(PartialEq, Eq)]
enum GcAction {
    Keep,
    Remove,
}

#[must_use]
enum SimplifyAction {
    Remove,
    Keep,
    Replace(NodeId),
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

struct DropContext {
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
