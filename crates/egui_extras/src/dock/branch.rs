use std::collections::{BTreeMap, HashMap, HashSet};

use egui::{pos2, vec2, NumExt as _, Rect};

use super::{
    is_being_dragged, is_possible_drag, sizes_from_shares, Behavior, DropContext, GridLoc,
    InsertionPoint, LayoutInsertion, NodeId, Nodes, SimplifyAction,
};

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

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Tabs {
    pub children: Vec<NodeId>,
    pub active: NodeId,
}

impl Tabs {
    pub fn new(children: Vec<NodeId>) -> Self {
        let active = children.first().copied().unwrap_or_default();
        Self { children, active }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum LinearDir {
    #[default]
    Horizontal,
    Vertical,
}

/// Horizontal or vertical layout.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Linear {
    pub children: Vec<NodeId>,
    pub dir: LinearDir,
    pub shares: Shares,
}

impl Linear {
    pub fn new(dir: LinearDir, children: Vec<NodeId>) -> Self {
        Self {
            children,
            dir,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Grid {
    pub children: Vec<NodeId>,

    pub locations: HashMap<NodeId, GridLoc>,

    /// Share of the avilable width assigned to each column.
    pub col_shares: Vec<f32>,
    /// Share of the avilable height assigned to each row.
    pub row_shares: Vec<f32>,
}

impl Grid {
    pub fn new(children: Vec<NodeId>) -> Self {
        Self {
            children,
            ..Default::default()
        }
    }
}

// ----------------------------------------------------------------------------
// Layout

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

impl Tabs {
    fn layout<Leaf>(
        &self,
        nodes: &mut Nodes<Leaf>,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        rect: Rect,
    ) {
        let mut active_rect = rect;
        active_rect.min.y += behavior.tab_bar_height(style);

        if false {
            nodes.layout_node(style, behavior, drop_context, active_rect, self.active);
        } else {
            // Layout all nodes in case the user switches active tab
            // TODO: only layout active tab, or don't register drop-zones during layout.
            for &child_id in &self.children {
                nodes.layout_node(style, behavior, drop_context, active_rect, child_id);
            }
        }
    }
}

impl Linear {
    fn layout<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        rect: Rect,
    ) {
        match self.dir {
            LinearDir::Horizontal => {
                self.layout_horizontal(nodes, style, behavior, drop_context, rect);
            }
            LinearDir::Vertical => self.layout_vertical(nodes, style, behavior, drop_context, rect),
        }
    }

    fn layout_horizontal<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        rect: Rect,
    ) {
        let num_gaps = self.children.len().saturating_sub(1);
        let gap_width = behavior.gap_width(style);
        let total_gap_width = gap_width * num_gaps as f32;
        let available_width = (rect.width() - total_gap_width).at_least(0.0);

        let widths = self.shares.split(&self.children, available_width);

        let mut x = rect.min.x;
        for (child, width) in self.children.iter().zip(widths) {
            let child_rect = Rect::from_min_size(pos2(x, rect.min.y), vec2(width, rect.height()));
            nodes.layout_node(style, behavior, drop_context, child_rect, *child);
            x += width + gap_width;
        }
    }

    fn layout_vertical<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        rect: Rect,
    ) {
        let num_gaps = self.children.len().saturating_sub(1);
        let gap_height = behavior.gap_width(style);
        let total_gap_height = gap_height * num_gaps as f32;
        let available_height = (rect.height() - total_gap_height).at_least(0.0);

        let heights = self.shares.split(&self.children, available_height);

        let mut y = rect.min.y;
        for (child, height) in self.children.iter().zip(heights) {
            let child_rect = Rect::from_min_size(pos2(rect.min.x, y), vec2(rect.width(), height));
            nodes.layout_node(style, behavior, drop_context, child_rect, *child);
            y += height + gap_height;
        }
    }
}

impl Grid {
    fn layout<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        rect: Rect,
        node_id: NodeId,
    ) {
        let child_ids: HashSet<NodeId> = self.children.iter().copied().collect();

        let mut num_cols = 2.min(self.children.len()); // les than 2 and it is not a grid

        // Where to place each node?
        let mut node_id_from_location: BTreeMap<GridLoc, NodeId> = Default::default();
        self.locations.retain(|&child_id, &mut loc| {
            if child_ids.contains(&child_id) {
                match node_id_from_location.entry(loc) {
                    std::collections::btree_map::Entry::Occupied(_) => {
                        false // two nodes assigned to the same position - forget this one for now
                    }
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(child_id);
                        num_cols = num_cols.max(loc.col + 1);
                        true
                    }
                }
            } else {
                false // child no longer exists
            }
        });

        // Find location for nodes that don't have one yet
        let mut next_pos = 0;
        for &child_id in &self.children {
            if let std::collections::hash_map::Entry::Vacant(entry) = self.locations.entry(child_id)
            {
                // find a position:
                loop {
                    let loc = GridLoc::from_col_row(next_pos % num_cols, next_pos / num_cols);
                    if node_id_from_location.contains_key(&loc) {
                        next_pos += 1;
                        continue;
                    }
                    entry.insert(loc);
                    node_id_from_location.insert(loc, child_id);
                    break;
                }
            }
        }

        // Everything has a location - now we know how many rows we have:
        let num_rows = node_id_from_location.keys().last().unwrap().row + 1;

        // Figure out where each column and row goes:
        self.col_shares.resize(num_cols, 1.0);
        self.row_shares.resize(num_rows, 1.0);

        let gap = behavior.gap_width(style);
        let col_widths = sizes_from_shares(&self.col_shares, rect.width(), gap);
        let row_heights = sizes_from_shares(&self.row_shares, rect.height(), gap);

        let mut col_x = vec![rect.left()];
        for &width in &col_widths {
            col_x.push(col_x.last().unwrap() + width + gap);
        }

        let mut row_y = vec![rect.top()];
        for &height in &row_heights {
            row_y.push(row_y.last().unwrap() + height + gap);
        }

        // Each child now has a location. Use this to order them, in case we will ater do auto-layouts:
        self.children.sort_by_key(|&child| self.locations[&child]);

        // Place each child:
        for &child in &self.children {
            let loc = self.locations[&child];
            let child_rect = Rect::from_min_size(
                pos2(col_x[loc.col], row_y[loc.row]),
                vec2(col_widths[loc.col], row_heights[loc.row]),
            );
            nodes.layout_node(style, behavior, drop_context, child_rect, child);
        }

        // Register drop-zones:
        for col in 0..num_cols {
            for row in 0..num_rows {
                let cell_rect = Rect::from_min_size(
                    pos2(col_x[col], row_y[row]),
                    vec2(col_widths[col], row_heights[row]),
                );
                drop_context.suggest_rect(
                    InsertionPoint::new(
                        node_id,
                        LayoutInsertion::Grid(GridLoc::from_col_row(col, row)),
                    ),
                    cell_rect,
                );
            }
        }
    }
}

// ----------------------------------------------------------------------------
// UI
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
                grid.grid_ui(nodes, behavior, drop_context, ui);
            }
        }
    }
}

impl Tabs {
    fn ui<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut egui::Ui,
        rect: Rect,
        node_id: NodeId,
    ) {
        if !self.children.iter().any(|&child| child == self.active) {
            // Make sure something is active:
            self.active = self.children.first().copied().unwrap_or_default();
        }

        let tab_bar_height = behavior.tab_bar_height(ui.style());
        let tab_bar_rect = rect.split_top_bottom_at_y(rect.top() + tab_bar_height).0;
        let mut tab_bar_ui = ui.child_ui(tab_bar_rect, *ui.layout());

        // Show tab bar:
        tab_bar_ui.horizontal(|ui| {
            let mut prev_tab_rect: Option<Rect> = None;
            let mut insertion_index = 0; // skips over drag-source, if any, beacuse it will be removed then re-inserted

            for (i, &child_id) in self.children.iter().enumerate() {
                if is_being_dragged(ui.ctx(), child_id) {
                    continue; // leave a gap!
                }

                let selected = child_id == self.active;
                let id = child_id.id();

                let response = behavior.tab_ui(nodes, ui, id, child_id, selected);
                let response = response.on_hover_cursor(egui::CursorIcon::Grab);
                if response.clicked() {
                    self.active = child_id;
                }

                if let Some(mouse_pos) = drop_context.mouse_pos {
                    if drop_context.dragged_node_id.is_some() && response.rect.contains(mouse_pos) {
                        // Expand this tab - maybe the user wants to drop something into it!
                        self.active = child_id;
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
                        InsertionPoint::new(node_id, LayoutInsertion::Tabs(insertion_index)),
                        Rect::from_center_size(before_point, vec2(4.0, rect.height())),
                    );
                }

                if i + 1 == self.children.len() {
                    // suggest dropping after last tab:
                    drop_context.suggest_rect(
                        InsertionPoint::new(node_id, LayoutInsertion::Tabs(insertion_index + 1)),
                        Rect::from_center_size(rect.right_center(), vec2(4.0, rect.height())),
                    );
                }

                prev_tab_rect = Some(rect);
                insertion_index += 1;
            }
        });

        // When dragged, don't show it (it is "being held")
        let is_active_being_dragged =
            ui.memory(|mem| mem.is_being_dragged(self.active.id())) && is_possible_drag(ui.ctx());
        if !is_active_being_dragged {
            nodes.node_ui(behavior, drop_context, ui, self.active);
        }
    }
}

impl Linear {
    fn ui<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut egui::Ui,
        node_id: NodeId,
    ) {
        match self.dir {
            LinearDir::Horizontal => self.horizontal_ui(nodes, behavior, drop_context, ui, node_id),
            LinearDir::Vertical => self.vertical_ui(nodes, behavior, drop_context, ui, node_id),
        }
    }

    fn horizontal_ui<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut egui::Ui,
        parent_id: NodeId,
    ) {
        let mut prev_rect: Option<Rect> = None;
        let mut insertion_index = 0; // skips over drag-source, if any, beacuse it will be removed then re-inserted

        for (i, &child) in self.children.iter().enumerate() {
            let Some(rect) = nodes.rect(child) else { continue; };

            if is_being_dragged(ui.ctx(), child) {
                // Leave a hole, and suggest that hole as drop-target:
                drop_context.suggest_rect(
                    InsertionPoint::new(parent_id, LayoutInsertion::Horizontal(i)),
                    rect,
                );
            } else {
                nodes.node_ui(behavior, drop_context, ui, child);

                if let Some(prev_rect) = prev_rect {
                    // Suggest dropping between the rects:
                    drop_context.suggest_rect(
                        InsertionPoint::new(
                            parent_id,
                            LayoutInsertion::Horizontal(insertion_index),
                        ),
                        Rect::from_min_max(prev_rect.center_top(), rect.center_bottom()),
                    );
                } else {
                    // Suggest dropping before the first child:
                    drop_context.suggest_rect(
                        InsertionPoint::new(parent_id, LayoutInsertion::Horizontal(0)),
                        rect.split_left_right_at_fraction(0.66).0,
                    );
                }

                if i + 1 == self.children.len() {
                    // Suggest dropping after the last child:
                    drop_context.suggest_rect(
                        InsertionPoint::new(
                            parent_id,
                            LayoutInsertion::Horizontal(insertion_index + 1),
                        ),
                        rect.split_left_right_at_fraction(0.33).1,
                    );
                }
                insertion_index += 1;
            }

            prev_rect = Some(rect);
        }
    }

    fn vertical_ui<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut egui::Ui,
        parent_id: NodeId,
    ) {
        // TODO: drag-and-drop
        for child in &self.children {
            nodes.node_ui(behavior, drop_context, ui, *child);
        }
    }
}

impl Grid {
    fn grid_ui<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut egui::Ui,
    ) {
        // Grid drops are handled during layout. TODO: handle here instead.

        for &child in &self.children {
            nodes.node_ui(behavior, drop_context, ui, child);
        }
    }
}
