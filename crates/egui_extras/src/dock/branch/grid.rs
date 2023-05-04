use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ops::RangeInclusive,
};

use egui::{pos2, vec2, Rect};

use crate::dock::{
    sizes_from_shares, Behavior, DropContext, InsertionPoint, LayoutInsertion, NodeId, Nodes,
};

/// Includive range of floats, i.e. `min..=max`, but more ergonomic than [`RangeInclusive`].
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rangef {
    pub min: f32,
    pub max: f32,
}

impl Rangef {
    #[inline]
    pub fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }
}

impl From<RangeInclusive<f32>> for Rangef {
    #[inline]
    fn from(range: RangeInclusive<f32>) -> Self {
        Self::new(*range.start(), *range.end())
    }
}

impl From<&RangeInclusive<f32>> for Rangef {
    #[inline]
    fn from(range: &RangeInclusive<f32>) -> Self {
        Self::new(*range.start(), *range.end())
    }
}

impl From<Rangef> for RangeInclusive<f32> {
    #[inline]
    fn from(Rangef { min, max }: Rangef) -> Self {
        min..=max
    }
}

/// Where in a grid?
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct GridLoc {
    // Row first for sorting
    pub row: usize,
    pub col: usize,
}

impl GridLoc {
    pub fn from_col_row(col: usize, row: usize) -> Self {
        Self { col, row }
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Grid {
    pub children: Vec<NodeId>,

    pub locations: HashMap<NodeId, GridLoc>,

    /// Share of the available width assigned to each column.
    pub col_shares: Vec<f32>,
    /// Share of the available height assigned to each row.
    pub row_shares: Vec<f32>,

    /// ui point x ranges for each column, recomputed during layout
    #[serde(skip)]
    col_ranges: Vec<Rangef>,

    /// ui point y ranges for each row, recomputed during layout
    #[serde(skip)]
    row_ranges: Vec<Rangef>,
}

impl Grid {
    pub fn new(children: Vec<NodeId>) -> Self {
        Self {
            children,
            ..Default::default()
        }
    }

    pub fn layout<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
        rect: Rect,
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

        {
            let mut x = rect.left();
            self.col_ranges.clear();
            for &width in &col_widths {
                self.col_ranges.push(Rangef::new(x, x + width));
                x += width + gap;
            }
        }
        {
            let mut y = rect.top();
            self.row_ranges.clear();
            for &height in &row_heights {
                self.row_ranges.push(Rangef::new(y, y + height));
                y += height + gap;
            }
        }

        // Each child now has a location. Use this to order them, in case we will ater do auto-layouts:
        self.children.sort_by_key(|&child| self.locations[&child]);

        // Place each child:
        for &child in &self.children {
            let loc = self.locations[&child];
            let child_rect =
                Rect::from_x_y_ranges(self.col_ranges[loc.col], self.row_ranges[loc.row]);
            nodes.layout_node(style, behavior, child_rect, child);
        }
    }

    pub fn ui<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        behavior: &mut dyn Behavior<Leaf>,
        drop_context: &mut DropContext,
        ui: &mut egui::Ui,
        node_id: NodeId,
    ) {
        // Grid drops are handled during layout. TODO: handle here instead.

        for &child in &self.children {
            nodes.node_ui(behavior, drop_context, ui, child);
        }

        // Register drop-zones:
        for (col, &x_range) in self.col_ranges.iter().enumerate() {
            for (row, &y_range) in self.row_ranges.iter().enumerate() {
                let cell_rect = Rect::from_x_y_ranges(x_range, y_range);
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
