use std::collections::HashMap;

use egui::Rect;

use super::{Behavior, DropContext, NodeId, Nodes, SimplifyAction};

mod grid;
mod linear;
mod tabs;

pub use grid::{Grid, GridLoc};
pub use linear::{Linear, LinearDir};
pub use tabs::Tabs;

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Layout {
    #[default]
    Tabs,
    Horizontal,
    Vertical,
    Grid,
}

impl Layout {
    pub const ALL: [Self; 4] = [Self::Tabs, Self::Horizontal, Self::Vertical, Self::Grid];
}

// ----------------------------------------------------------------------------

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
    pub fn replace_with(&mut self, a: NodeId, b: NodeId) {
        if let Some(share) = self.shares.remove(&a) {
            self.shares.insert(b, share);
        }
    }

    pub fn split(&self, children: &[NodeId], available_width: f32) -> Vec<f32> {
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

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Branch {
    Tabs(Tabs),
    Linear(Linear),
    Grid(Grid),
}

impl Branch {
    pub fn new_linear(dir: LinearDir, children: Vec<NodeId>) -> Self {
        Self::Linear(Linear::new(dir, children))
    }

    pub fn new_tabs(children: Vec<NodeId>) -> Self {
        Self::Tabs(Tabs::new(children))
    }

    pub fn new_grid(children: Vec<NodeId>) -> Self {
        Self::Grid(Grid::new(children))
    }

    pub fn is_empty(&self) -> bool {
        self.children().is_empty()
    }

    pub fn children(&self) -> &[NodeId] {
        match self {
            Self::Tabs(tabs) => &tabs.children,
            Self::Linear(linear) => &linear.children,
            Self::Grid(grid) => &grid.children,
        }
    }

    pub fn get_layout(&self) -> Layout {
        match self {
            Self::Tabs(_) => Layout::Tabs,
            Self::Linear(linear) => match linear.dir {
                LinearDir::Horizontal => Layout::Horizontal,
                LinearDir::Vertical => Layout::Vertical,
            },
            Self::Grid(_) => Layout::Grid,
        }
    }

    pub fn set_layout(&mut self, layout: Layout) {
        if layout == self.get_layout() {
            return;
        }

        *self = match layout {
            Layout::Tabs => Self::Tabs(Tabs::new(self.children().to_vec())),
            Layout::Horizontal => {
                Self::Linear(Linear::new(LinearDir::Horizontal, self.children().to_vec()))
            }
            Layout::Vertical => {
                Self::Linear(Linear::new(LinearDir::Vertical, self.children().to_vec()))
            }
            Layout::Grid => Self::Grid(Grid::new(self.children().to_vec())),
        };
    }

    pub(super) fn retain(&mut self, mut retain: impl FnMut(NodeId) -> bool) {
        let retain = |node_id: &NodeId| retain(*node_id);
        match self {
            Self::Tabs(tabs) => tabs.children.retain(retain),
            Self::Linear(linear) => linear.children.retain(retain),
            Self::Grid(grid) => grid.children.retain(retain),
        }
    }

    pub(super) fn simplify_children(&mut self, mut simplify: impl FnMut(NodeId) -> SimplifyAction) {
        match self {
            Self::Tabs(tabs) => tabs.children.retain_mut(|child| match simplify(*child) {
                SimplifyAction::Remove => false,
                SimplifyAction::Keep => true,
                SimplifyAction::Replace(new) => {
                    if tabs.active == *child {
                        tabs.active = new;
                    }
                    *child = new;
                    true
                }
            }),
            Self::Linear(linear) => linear.children.retain_mut(|child| match simplify(*child) {
                SimplifyAction::Remove => false,
                SimplifyAction::Keep => true,
                SimplifyAction::Replace(new) => {
                    linear.shares.replace_with(*child, new);
                    *child = new;
                    true
                }
            }),
            Self::Grid(grid) => grid.children.retain_mut(|child| match simplify(*child) {
                SimplifyAction::Remove => false,
                SimplifyAction::Keep => true,
                SimplifyAction::Replace(new) => {
                    if let Some(loc) = grid.locations.remove(child) {
                        grid.locations.insert(new, loc);
                    }
                    *child = new;
                    true
                }
            }),
        }
    }
}

impl Branch {
    pub(super) fn layout<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        rect: Rect,
        node_id: NodeId,
    ) {
        if self.is_empty() {
            return;
        }

        match self {
            Branch::Tabs(tabs) => tabs.layout(nodes, style, behavior, drop_context, rect),
            Branch::Linear(linear) => {
                linear.layout(nodes, style, behavior, drop_context, rect);
            }
            Branch::Grid(grid) => grid.layout(nodes, style, behavior, drop_context, rect, node_id),
        }
    }
}

impl Branch {
    pub(super) fn ui<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut egui::Ui,
        rect: Rect,
        node_id: NodeId,
    ) {
        match self {
            Branch::Tabs(tabs) => {
                tabs.ui(nodes, behavior, drop_context, ui, rect, node_id);
            }
            Branch::Linear(linear) => {
                linear.ui(nodes, behavior, drop_context, ui, node_id);
            }
            Branch::Grid(grid) => {
                grid.ui(nodes, behavior, drop_context, ui);
            }
        }
    }
}
