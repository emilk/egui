use egui::{vec2, Rect};

use crate::dock::{
    is_being_dragged, is_possible_drag, Behavior, DropContext, InsertionPoint, LayoutInsertion,
    NodeId, Nodes,
};

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

    pub fn layout<Leaf>(
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

    pub fn ui<Leaf>(
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
