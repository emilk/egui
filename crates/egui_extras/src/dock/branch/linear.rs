use std::collections::HashMap;

use egui::{pos2, vec2, NumExt, Rect};
use itertools::Itertools as _;

use crate::dock::{
    is_being_dragged, Behavior, DropContext, InsertionPoint, LayoutInsertion, NodeId, Nodes,
    ResizeState,
};

// ----------------------------------------------------------------------------

/// How large of a share of space each child has, on a 1D axis.
///
/// Used for [`Linear`] layouts (horizontal and vertical).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Shares {
    /// How large of a share each child has.
    ///
    /// For instance, the shares `[1, 2, 3]` means that the first child gets 1/6 of the space,
    /// the second gets 2/6 and the third gets 3/6.
    shares: HashMap<NodeId, f32>,
}

impl Shares {
    pub fn replace_with(&mut self, remove: NodeId, new: NodeId) {
        if let Some(share) = self.shares.remove(&remove) {
            self.shares.insert(new, share);
        }
    }

    /// Split the given width based on the share of the children.
    pub fn split(&self, children: &[NodeId], available_width: f32) -> Vec<f32> {
        let mut num_shares = 0.0;
        for &child in children {
            num_shares += self[child];
        }
        if num_shares == 0.0 {
            num_shares = 1.0;
        }
        children
            .iter()
            .map(|&child| available_width * self[child] / num_shares)
            .collect()
    }
}

impl std::ops::Index<NodeId> for Shares {
    type Output = f32;

    #[inline]
    fn index(&self, id: NodeId) -> &Self::Output {
        self.shares.get(&id).unwrap_or(&1.0)
    }
}

impl std::ops::IndexMut<NodeId> for Shares {
    #[inline]
    fn index_mut(&mut self, id: NodeId) -> &mut Self::Output {
        self.shares.entry(id).or_insert(1.0)
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
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

    pub fn add_child(&mut self, child: NodeId) {
        self.children.push(child);
    }

    pub fn layout<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
        rect: Rect,
    ) {
        match self.dir {
            LinearDir::Horizontal => {
                self.layout_horizontal(nodes, style, behavior, rect);
            }
            LinearDir::Vertical => self.layout_vertical(nodes, style, behavior, rect),
        }
    }

    fn layout_horizontal<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
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
            nodes.layout_node(style, behavior, child_rect, *child);
            x += width + gap_width;
        }
    }

    fn layout_vertical<Leaf>(
        &mut self,
        nodes: &mut Nodes<Leaf>,
        style: &egui::Style,
        behavior: &mut dyn Behavior<Leaf>,
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
            nodes.layout_node(style, behavior, child_rect, *child);
            y += height + gap_height;
        }
    }

    pub(super) fn ui<Leaf>(
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
        for &child in &self.children {
            if !is_being_dragged(ui.ctx(), child) {
                nodes.node_ui(behavior, drop_context, ui, child);
            }
        }

        linear_drop_zones(ui.ctx(), nodes, &self.children, self.dir, |rect, i| {
            drop_context.suggest_rect(
                InsertionPoint::new(parent_id, LayoutInsertion::Horizontal(i)),
                rect,
            );
        });

        // ------------------------
        // resizing:

        let parent_rect = nodes.rect(parent_id);
        for (i, (left, right)) in self.children.iter().copied().tuple_windows().enumerate() {
            let resize_id = egui::Id::new((parent_id, "resize", i));

            let left_rect = nodes.rect(left);
            let right_rect = nodes.rect(right);
            let x = egui::lerp(left_rect.right()..=right_rect.left(), 0.5);

            let mut resize_state = ResizeState::Idle;
            if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                let line_rect = Rect::from_center_size(
                    pos2(x, parent_rect.center().y),
                    vec2(
                        2.0 * ui.style().interaction.resize_grab_radius_side,
                        parent_rect.height(),
                    ),
                );
                let response = ui.interact(line_rect, resize_id, egui::Sense::click_and_drag());
                resize_state = resize_interaction(
                    behavior,
                    &mut self.shares,
                    &self.children,
                    &response,
                    [left, right],
                    ui.painter().round_to_pixel(pointer.x) - x,
                    i,
                    |node_id: NodeId| nodes.rect(node_id).width(),
                );

                if resize_state != ResizeState::Idle {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }
            }

            let stroke = behavior.resize_stroke(ui.style(), resize_state);
            ui.painter().vline(x, parent_rect.y_range(), stroke);
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
        for &child in &self.children {
            if !is_being_dragged(ui.ctx(), child) {
                nodes.node_ui(behavior, drop_context, ui, child);
            }
        }

        linear_drop_zones(ui.ctx(), nodes, &self.children, self.dir, |rect, i| {
            drop_context.suggest_rect(
                InsertionPoint::new(parent_id, LayoutInsertion::Vertical(i)),
                rect,
            );
        });

        // ------------------------
        // resizing:

        let parent_rect = nodes.rect(parent_id);
        for (i, (top, bottom)) in self.children.iter().copied().tuple_windows().enumerate() {
            let resize_id = egui::Id::new((parent_id, "resize", i));

            let top_rect = nodes.rect(top);
            let bottom_rect = nodes.rect(bottom);
            let y = egui::lerp(top_rect.bottom()..=bottom_rect.top(), 0.5);

            let mut resize_state = ResizeState::Idle;
            if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                let line_rect = Rect::from_center_size(
                    pos2(parent_rect.center().x, y),
                    vec2(
                        parent_rect.width(),
                        2.0 * ui.style().interaction.resize_grab_radius_side,
                    ),
                );
                let response = ui.interact(line_rect, resize_id, egui::Sense::click_and_drag());
                resize_state = resize_interaction(
                    behavior,
                    &mut self.shares,
                    &self.children,
                    &response,
                    [top, bottom],
                    ui.painter().round_to_pixel(pointer.y) - y,
                    i,
                    |node_id: NodeId| nodes.rect(node_id).height(),
                );

                if resize_state != ResizeState::Idle {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
                }
            }

            let stroke = behavior.resize_stroke(ui.style(), resize_state);
            ui.painter().hline(parent_rect.x_range(), y, stroke);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resize_interaction<Leaf>(
    behavior: &mut dyn Behavior<Leaf>,
    shares: &mut Shares,
    children: &[NodeId],
    splitter_response: &egui::Response,
    [left, right]: [NodeId; 2],
    dx: f32,
    i: usize,
    node_width: impl Fn(NodeId) -> f32,
) -> ResizeState {
    if splitter_response.double_clicked() {
        // double-click to center the split between left and right:
        let mean = 0.5 * (shares[left] + shares[right]);
        shares[left] = mean;
        shares[right] = mean;
        ResizeState::Hovering
    } else if splitter_response.dragged() {
        if dx < 0.0 {
            // Expand right, shrink stuff to the left:
            shares[right] += shrink_shares(
                behavior,
                shares,
                &children[0..=i].iter().copied().rev().collect_vec(),
                dx.abs(),
                node_width,
            );
        } else {
            // Expand the left, shrink stuff to the right:
            shares[left] +=
                shrink_shares(behavior, shares, &children[i + 1..], dx.abs(), node_width);
        }
        ResizeState::Dragging
    } else if splitter_response.hovered() {
        ResizeState::Hovering
    } else {
        ResizeState::Idle
    }
}

/// Try shrink the children by a total of `target_in_points`,
/// making sure no child gets smaller than its minimum size.
fn shrink_shares<Leaf>(
    behavior: &dyn Behavior<Leaf>,
    shares: &mut Shares,
    children: &[NodeId],
    target_in_points: f32,
    size_in_point: impl Fn(NodeId) -> f32,
) -> f32 {
    if children.is_empty() {
        return 0.0;
    }

    let mut total_shares = 0.0;
    let mut total_points = 0.0;
    for &child in children {
        total_shares += shares[child];
        total_points += size_in_point(child);
    }

    let shares_per_point = total_shares / total_points;

    let min_size_in_points = shares_per_point * behavior.min_size();

    let target_in_shares = shares_per_point * target_in_points;
    let mut total_shares_lost = 0.0;

    for &child in children {
        let share = &mut shares[child];
        let shrink_by = (target_in_shares - total_shares_lost)
            .min(*share - min_size_in_points)
            .max(0.0);

        *share -= shrink_by;
        total_shares_lost += shrink_by;
    }

    total_shares_lost
}

fn linear_drop_zones<Leaf>(
    egui_ctx: &egui::Context,
    nodes: &Nodes<Leaf>,
    children: &[NodeId],
    dir: LinearDir,
    add_drop_drect: impl FnMut(Rect, usize),
) {
    let preview_thickness = 12.0;
    let dragged_index = children
        .iter()
        .position(|&child| is_being_dragged(egui_ctx, child));

    let after_rect = |rect: Rect| match dir {
        LinearDir::Horizontal => Rect::from_min_max(
            rect.right_top() - vec2(preview_thickness, 0.0),
            rect.right_bottom(),
        ),
        LinearDir::Vertical => Rect::from_min_max(
            rect.left_bottom() - vec2(0.0, preview_thickness),
            rect.right_bottom(),
        ),
    };

    drop_zones(
        preview_thickness,
        children,
        dragged_index,
        dir,
        |node_id| nodes.rect(node_id),
        add_drop_drect,
        after_rect,
    );
}

/// Register drop-zones for a linear layout.
pub(super) fn drop_zones(
    preview_thickness: f32,
    children: &[NodeId],
    dragged_index: Option<usize>,
    dir: LinearDir,
    get_rect: impl Fn(NodeId) -> Rect,
    mut add_drop_drect: impl FnMut(Rect, usize),
    after_rect: impl Fn(Rect) -> Rect,
) {
    let before_rect = |rect: Rect| match dir {
        LinearDir::Horizontal => Rect::from_min_max(
            rect.left_top(),
            rect.left_bottom() + vec2(preview_thickness, 0.0),
        ),
        LinearDir::Vertical => Rect::from_min_max(
            rect.left_top(),
            rect.right_top() + vec2(0.0, preview_thickness),
        ),
    };
    let between_rects = |a: Rect, b: Rect| match dir {
        LinearDir::Horizontal => Rect::from_center_size(
            a.right_center().lerp(b.left_center(), 0.5),
            vec2(preview_thickness, a.height()),
        ),
        LinearDir::Vertical => Rect::from_center_size(
            a.center_bottom().lerp(b.center_top(), 0.5),
            vec2(a.width(), preview_thickness),
        ),
    };

    let mut prev_rect: Option<Rect> = None;
    let mut insertion_index = 0; // skips over drag-source, if any, because it will be removed before its re-inserted

    for (i, &child) in children.iter().enumerate() {
        let rect = get_rect(child);

        if Some(i) == dragged_index {
            // Suggest hole as a drop-target:
            add_drop_drect(rect, i);
        } else {
            if let Some(prev_rect) = prev_rect {
                if Some(i - 1) != dragged_index {
                    // Suggest dropping between the rects:
                    add_drop_drect(between_rects(prev_rect, rect), insertion_index);
                }
            } else {
                // Suggest dropping before the first child:
                add_drop_drect(before_rect(rect), 0);
            }

            insertion_index += 1;
        }

        prev_rect = Some(rect);
    }

    if let Some(last_rect) = prev_rect {
        // Suggest dropping after the last child (unless that's the one being dragged):
        if dragged_index != Some(children.len() - 1) {
            add_drop_drect(after_rect(last_rect), insertion_index + 1);
        }
    }
}
