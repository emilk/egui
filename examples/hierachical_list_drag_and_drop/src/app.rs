//! This demo is a stripped-down version of the drag-and-drop implementation in the
//! [rerun viewer](https://github.com/rerun-io/rerun).

use std::collections::HashMap;

use eframe::{egui, egui::NumExt as _};

#[derive(Hash, Clone, Copy, PartialEq, Eq)]
struct ItemId(u32);

impl ItemId {
    fn new() -> Self {
        Self(rand::random())
    }
}

impl std::fmt::Debug for ItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{:04x}", self.0)
    }
}

impl From<ItemId> for egui::Id {
    fn from(id: ItemId) -> Self {
        Self::new(id)
    }
}

enum Item {
    Container(Vec<ItemId>),
    Leaf(String),
}

#[derive(Debug)]
enum Command {
    /// Move the currently dragged item to the given container and position.
    MoveItem {
        moved_item_id: ItemId,
        target_container_id: ItemId,
        target_position_index: usize,
    },

    /// Specify the currently identified target container to be highlighted.
    HighlightTargetContainer(ItemId),
}

pub struct HierarchicalDragAndDrop {
    /// All items
    items: HashMap<ItemId, Item>,

    /// Id of the root item (not displayed in the UI)
    root_id: ItemId,

    /// If a drag is ongoing, this is the id of the destination container (if any was identified)
    ///
    /// This is used to highlight the target container.
    target_container: Option<ItemId>,

    /// Channel to receive commands from the UI
    command_receiver: std::sync::mpsc::Receiver<Command>,

    /// Channel to send commands from the UI
    command_sender: std::sync::mpsc::Sender<Command>,
}

impl Default for HierarchicalDragAndDrop {
    fn default() -> Self {
        let root_item = Item::Container(Vec::new());
        let root_id = ItemId::new();

        let (command_sender, command_receiver) = std::sync::mpsc::channel();

        let mut res = Self {
            items: std::iter::once((root_id, root_item)).collect(),
            root_id,
            target_container: None,
            command_receiver,
            command_sender,
        };

        res.populate();

        res
    }
}

impl eframe::App for HierarchicalDragAndDrop {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui(ui);
        });
    }
}

//
// Data stuff
//
impl HierarchicalDragAndDrop {
    /// Add a bunch of items in the hierarchy.
    fn populate(&mut self) {
        let c1 = self.add_container(self.root_id);
        let c2 = self.add_container(self.root_id);
        let c3 = self.add_container(self.root_id);
        self.add_leaf(self.root_id);
        self.add_leaf(self.root_id);

        let c11 = self.add_container(c1);
        let c12 = self.add_container(c1);
        self.add_leaf(c11);
        self.add_leaf(c11);
        self.add_leaf(c12);
        self.add_leaf(c12);

        self.add_leaf(c2);
        self.add_leaf(c2);

        self.add_leaf(c3);
    }

    fn container(&self, id: ItemId) -> Option<&Vec<ItemId>> {
        match self.items.get(&id) {
            Some(Item::Container(children)) => Some(children),
            _ => None,
        }
    }

    /// Does some container contain the given item?
    ///
    /// Used to test if a target location is suitable for a given dragged item.
    fn contains(&self, container_id: ItemId, item_id: ItemId) -> bool {
        if let Some(children) = self.container(container_id) {
            if container_id == item_id {
                return true;
            }

            if children.contains(&item_id) {
                return true;
            }

            for child_id in children {
                if self.contains(*child_id, item_id) {
                    return true;
                }
            }

            return false;
        }

        false
    }

    /// Move item `item_id` to `container_id` at position `pos`.
    fn move_item(&mut self, item_id: ItemId, container_id: ItemId, mut pos: usize) {
        println!("Moving {item_id:?} to {container_id:?} at position {pos:?}");

        // Remove the item from its current location. Note: we must adjust the target position if the item is
        // moved within the same container, as the removal might shift the positions by one.
        if let Some((source_parent_id, source_pos)) = self.parent_and_pos(item_id) {
            if let Some(Item::Container(children)) = self.items.get_mut(&source_parent_id) {
                children.remove(source_pos);
            }

            if source_parent_id == container_id && source_pos < pos {
                pos -= 1;
            }
        }

        if let Some(Item::Container(children)) = self.items.get_mut(&container_id) {
            children.insert(pos.at_most(children.len()), item_id);
        }
    }

    /// Find the parent of an item, and the index of that item within the parent's children.
    fn parent_and_pos(&self, id: ItemId) -> Option<(ItemId, usize)> {
        if id == self.root_id {
            None
        } else {
            self.parent_and_pos_impl(id, self.root_id)
        }
    }

    fn parent_and_pos_impl(&self, id: ItemId, container_id: ItemId) -> Option<(ItemId, usize)> {
        if let Some(children) = self.container(container_id) {
            for (idx, child_id) in children.iter().enumerate() {
                if child_id == &id {
                    return Some((container_id, idx));
                } else if self.container(*child_id).is_some() {
                    let res = self.parent_and_pos_impl(id, *child_id);
                    if res.is_some() {
                        return res;
                    }
                }
            }
        }

        None
    }

    fn add_container(&mut self, parent_id: ItemId) -> ItemId {
        let id = ItemId::new();
        let item = Item::Container(Vec::new());

        self.items.insert(id, item);

        if let Some(Item::Container(children)) = self.items.get_mut(&parent_id) {
            children.push(id);
        }

        id
    }

    fn add_leaf(&mut self, parent_id: ItemId) {
        let id = ItemId::new();
        let item = Item::Leaf(format!("Item {id:?}"));

        self.items.insert(id, item);

        if let Some(Item::Container(children)) = self.items.get_mut(&parent_id) {
            children.push(id);
        }
    }

    fn send_command(&self, command: Command) {
        // The only way this can fail is if the receiver has been dropped.
        self.command_sender.send(command).ok();
    }
}

//
// UI stuff
//
impl HierarchicalDragAndDrop {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        if let Some(top_level_items) = self.container(self.root_id) {
            self.container_children_ui(ui, top_level_items);
        }

        // always reset the target container
        self.target_container = None;

        while let Ok(command) = self.command_receiver.try_recv() {
            //println!("Received command: {command:?}");
            match command {
                Command::MoveItem {
                    moved_item_id,
                    target_container_id,
                    target_position_index,
                } => self.move_item(moved_item_id, target_container_id, target_position_index),
                Command::HighlightTargetContainer(item_id) => {
                    self.target_container = Some(item_id);
                }
            }
        }
    }

    fn container_ui(&self, ui: &mut egui::Ui, item_id: ItemId, children: &Vec<ItemId>) {
        let (response, head_response, body_resp) =
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                item_id.into(),
                true,
            )
            .show_header(ui, |ui| {
                ui.add(
                    egui::Label::new(format!("Container {item_id:?}"))
                        .selectable(false)
                        .sense(egui::Sense::drag()),
                )
            })
            .body(|ui| {
                self.container_children_ui(ui, children);
            });

        self.handle_drag_and_drop_interaction(
            ui,
            item_id,
            true,
            &head_response.inner.union(response),
            body_resp.as_ref().map(|r| &r.response),
        );
    }

    fn container_children_ui(&self, ui: &mut egui::Ui, children: &Vec<ItemId>) {
        for child_id in children {
            match self.items.get(child_id) {
                Some(Item::Container(children)) => {
                    self.container_ui(ui, *child_id, children);
                }
                Some(Item::Leaf(label)) => {
                    self.leaf_ui(ui, *child_id, label);
                }
                None => {}
            }
        }
    }

    fn leaf_ui(&self, ui: &mut egui::Ui, item_id: ItemId, label: &str) {
        let response = ui.add(
            egui::Label::new(label)
                .selectable(false)
                .sense(egui::Sense::drag()),
        );

        self.handle_drag_and_drop_interaction(ui, item_id, false, &response, None);
    }

    fn handle_drag_and_drop_interaction(
        &self,
        ui: &egui::Ui,
        item_id: ItemId,
        is_container: bool,
        response: &egui::Response,
        body_response: Option<&egui::Response>,
    ) {
        //
        // handle start of drag
        //

        if response.drag_started() {
            egui::DragAndDrop::set_payload(ui.ctx(), item_id);
        }

        //
        // handle candidate drop
        //

        // find the item being dragged
        let Some(dragged_item_id) = egui::DragAndDrop::payload(ui.ctx()).map(|payload| (*payload))
        else {
            // nothing is being dragged, we're done here
            return;
        };

        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);

        let Some((parent_id, position_index_in_parent)) = self.parent_and_pos(item_id) else {
            // this shouldn't happen
            return;
        };

        let previous_container_id = if position_index_in_parent > 0 {
            self.container(parent_id)
                .map(|c| c[position_index_in_parent - 1])
                .filter(|id| self.container(*id).is_some())
        } else {
            None
        };

        let item_desc = crate::drag_and_drop::DropItemDescription {
            id: item_id,
            is_container,
            parent_id,
            position_index_in_parent,
            previous_container_id,
        };

        //
        // compute the drag target areas based on the item and body responses
        //

        // adjust the drop target to account for the spacing between items
        let item_rect = response
            .rect
            .expand2(egui::Vec2::new(0.0, ui.spacing().item_spacing.y / 2.0));
        let body_rect = body_response.map(|r| {
            r.rect
                .expand2(egui::Vec2::new(0.0, ui.spacing().item_spacing.y))
        });

        //
        // find the candidate drop target
        //

        let drop_target = crate::drag_and_drop::find_drop_target(
            ui,
            &item_desc,
            item_rect,
            body_rect,
            response.rect.height(),
        );

        if let Some(drop_target) = drop_target {
            // We cannot allow the target location to be "inside" the dragged item, because that would amount moving
            // myself inside of me.

            if self.contains(dragged_item_id, drop_target.target_parent_id) {
                return;
            }

            // extend the cursor to the right of the enclosing container
            let mut span_x = drop_target.indicator_span_x;
            span_x.max = ui.cursor().right();

            ui.painter().hline(
                span_x,
                drop_target.indicator_position_y,
                (2.0, egui::Color32::BLACK),
            );

            // note: can't use `response.drag_released()` because we not the item which
            // started the drag
            if ui.input(|i| i.pointer.any_released()) {
                self.send_command(Command::MoveItem {
                    moved_item_id: dragged_item_id,
                    target_container_id: drop_target.target_parent_id,
                    target_position_index: drop_target.target_position_index,
                });

                egui::DragAndDrop::clear_payload(ui.ctx());
            } else {
                self.send_command(Command::HighlightTargetContainer(
                    drop_target.target_parent_id,
                ));
            }
        }
    }
}
