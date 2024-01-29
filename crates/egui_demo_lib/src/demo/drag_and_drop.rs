use egui::*;

// <<<<<<< HEAD
// pub fn drag_source(ui: &mut Ui, id: Id, body: impl FnOnce(&mut Ui)) {
//     let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(id));

//     if !is_being_dragged {
//         let response = ui.scope(body).response;

//         // Check for drags:
//         let response = ui.interact(response.rect, id, Sense::drag());
//         if response.hovered() {
//             ui.ctx().set_cursor_icon(CursorIcon::Grab);
//         }
//     } else {
//         ui.ctx().set_cursor_icon(CursorIcon::Grabbing);

//         // Paint the body to a new layer:
//         let layer_id = LayerId::new(Order::Tooltip, id);
//         let response = ui.with_layer_id(layer_id, body).response;

//         // Now we move the visuals of the body to where the mouse is.
//         // Normally you need to decide a location for a widget first,
//         // because otherwise that widget cannot interact with the mouse.
//         // However, a dragged component cannot be interacted with anyway
//         // (anything with `Order::Tooltip` always gets an empty [`Response`])
//         // So this is fine!

//         if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
//             let delta = pointer_pos - response.rect.center();
//             ui.ctx().transform_layer(layer_id, delta, 1.0);
//         }
//     }
// }

// pub fn drop_target<R>(
//     ui: &mut Ui,
//     can_accept_what_is_being_dragged: bool,
//     body: impl FnOnce(&mut Ui) -> R,
// ) -> InnerResponse<R> {
//     let is_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());

//     let margin = Vec2::splat(4.0);

//     let outer_rect_bounds = ui.available_rect_before_wrap();
//     let inner_rect = outer_rect_bounds.shrink2(margin);
//     let where_to_put_background = ui.painter().add(Shape::Noop);
//     let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
//     let ret = body(&mut content_ui);
//     let outer_rect = Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
//     let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());

//     // NOTE: we use `response.contains_pointer` here instead of `hovered`, because
//     // `hovered` is always false when another widget is being dragged.
//     let style =
//         if is_being_dragged && can_accept_what_is_being_dragged && response.contains_pointer() {
//             ui.visuals().widgets.active
//         } else {
//             ui.visuals().widgets.inactive
//         };

//     let mut fill = style.bg_fill;
//     let mut stroke = style.bg_stroke;
//     if is_being_dragged && !can_accept_what_is_being_dragged {
//         fill = ui.visuals().gray_out(fill);
//         stroke.color = ui.visuals().gray_out(stroke.color);
//     }

//     ui.painter().set(
//         where_to_put_background,
//         epaint::RectShape::new(rect, style.rounding, fill, stroke),
//     );

//     InnerResponse::new(ret, response)
// }

// =======
// >>>>>>> master
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DragAndDropDemo {
    /// columns with items
    columns: Vec<Vec<String>>,
}

impl Default for DragAndDropDemo {
    fn default() -> Self {
        Self {
            columns: vec![
                vec!["Item A", "Item B", "Item C", "Item D"],
                vec!["Item E", "Item F", "Item G"],
                vec!["Item H", "Item I", "Item J", "Item K"],
            ]
            .into_iter()
            .map(|v| v.into_iter().map(ToString::to_string).collect())
            .collect(),
        }
    }
}

impl super::Demo for DragAndDropDemo {
    fn name(&self) -> &'static str {
        "✋ Drag and Drop"
    }

    fn show(&mut self, ctx: &Context, open: &mut bool) {
        use super::View as _;
        Window::new(self.name())
            .open(open)
            .default_size(vec2(256.0, 256.0))
            .vscroll(false)
            .resizable(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

/// What is being dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Location {
    col: usize,
    row: usize,
}

impl super::View for DragAndDropDemo {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("This is a simple example of drag-and-drop in egui.");
        ui.label("Drag items between columns.");

        // If there is a drop, store the location of the item being dragged, and the destination for the drop.
        let mut from = None;
        let mut to = None;

        ui.columns(self.columns.len(), |uis| {
            for (col_idx, column) in self.columns.clone().into_iter().enumerate() {
                let ui = &mut uis[col_idx];

                let frame = Frame::default().inner_margin(4.0);

                let (_, dropped_payload) = ui.dnd_drop_zone::<Location>(frame, |ui| {
                    ui.set_min_size(vec2(64.0, 100.0));
                    for (row_idx, item) in column.iter().enumerate() {
                        let item_id = Id::new(("my_drag_and_drop_demo", col_idx, row_idx));
                        let item_location = Location {
                            col: col_idx,
                            row: row_idx,
                        };
                        let response = ui
                            .dnd_drag_source(item_id, item_location, |ui| {
                                ui.label(item);
                            })
                            .response;

                        // Detect drops onto this item:
                        if let (Some(pointer), Some(hovered_payload)) = (
                            ui.input(|i| i.pointer.interact_pos()),
                            response.dnd_hover_payload::<Location>(),
                        ) {
                            let rect = response.rect;

                            // Preview insertion:
                            let stroke = egui::Stroke::new(1.0, Color32::WHITE);
                            let insert_row_idx = if *hovered_payload == item_location {
                                // We are dragged onto ourselves
                                ui.painter().hline(rect.x_range(), rect.center().y, stroke);
                                row_idx
                            } else if pointer.y < rect.center().y {
                                // Above us
                                ui.painter().hline(rect.x_range(), rect.top(), stroke);
                                row_idx
                            } else {
                                // Below us
                                ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                                row_idx + 1
                            };

                            if let Some(dragged_payload) = response.dnd_release_payload() {
                                // The user dropped onto this item.
                                from = Some(dragged_payload);
                                to = Some(Location {
                                    col: col_idx,
                                    row: insert_row_idx,
                                });
                            }
                        }
                    }
                });

                if let Some(dragged_payload) = dropped_payload {
                    // The user dropped onto the column, but not on any one item.
                    from = Some(dragged_payload);
                    to = Some(Location {
                        col: col_idx,
                        row: usize::MAX, // Inset last
                    });
                }
            }
        });

        if let (Some(from), Some(mut to)) = (from, to) {
            if from.col == to.col {
                // Dragging within the same column.
                // Adjust row index if we are re-ordering:
                to.row -= (from.row < to.row) as usize;
            }

            let item = self.columns[from.col].remove(from.row);

            let column = &mut self.columns[to.col];
            to.row = to.row.min(column.len());
            column.insert(to.row, item);
        }

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}
