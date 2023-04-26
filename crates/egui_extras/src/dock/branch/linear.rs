use egui::{pos2, vec2, NumExt, Rect};

use crate::dock::{
    is_being_dragged, Behavior, DropContext, InsertionPoint, LayoutInsertion, NodeId, Nodes,
};

use super::Shares;

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

    pub fn layout<Leaf>(
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

    pub fn ui<Leaf>(
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
