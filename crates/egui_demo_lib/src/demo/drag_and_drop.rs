use egui::*;

pub fn drag_source(ui: &mut Ui, id: Id, body: impl FnOnce(&mut Ui)) -> egui::Response {
    let is_being_dragged = ui.memory(|mem| mem.is_being_dragged(id));

    if !is_being_dragged {
        let response = ui.scope(body).response;

        // Check for drags:
        let response = ui.interact(response.rect, id, Sense::drag());
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Grab);
        }
        response
    } else {
        // Paint the body to a new layer:
        let layer_id = LayerId::new(Order::Tooltip, id);
        let response = ui.with_layer_id(layer_id, body).response;

        // Now we move the visuals of the body to where the mouse is.
        // Normally you need to decide a location for a widget first,
        // because otherwise that widget cannot interact with the mouse.
        // However, a dragged component cannot be interacted with anyway
        // (anything with `Order::Tooltip` always gets an empty [`Response`])
        // So this is fine!

        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            let delta = pointer_pos - response.rect.center();
            ui.ctx().translate_layer(layer_id, delta);
        }

        response
    }
}

pub fn drop_target<R>(
    ui: &mut Ui,
    can_accept_what_is_being_dragged: bool,
    body: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let is_being_dragged = ui.memory(|mem| mem.is_anything_being_dragged());

    let margin = Vec2::splat(4.0);

    let outer_rect_bounds = ui.available_rect_before_wrap();
    let inner_rect = outer_rect_bounds.shrink2(margin);
    let where_to_put_background = ui.painter().add(Shape::Noop);
    let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
    let ret = body(&mut content_ui);
    let outer_rect = Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
    let (rect, response) = ui.allocate_at_least(outer_rect.size(), Sense::hover());

    // NOTE: we use `response.contains_pointer` here instead of `hovered`, because
    // `hovered` is always false when another widget is being dragged.
    let style =
        if is_being_dragged && can_accept_what_is_being_dragged && response.contains_pointer() {
            ui.visuals().widgets.active
        } else {
            ui.visuals().widgets.inactive
        };

    let mut fill = style.bg_fill;
    let mut stroke = style.bg_stroke;
    if is_being_dragged && !can_accept_what_is_being_dragged {
        fill = ui.visuals().gray_out(fill);
        stroke.color = ui.visuals().gray_out(stroke.color);
    }

    ui.painter().set(
        where_to_put_background,
        epaint::RectShape::new(rect, style.rounding, fill, stroke),
    );

    InnerResponse::new(ret, response)
}

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
                vec!["Item A", "Item B", "Item C"],
                vec!["Item D", "Item E"],
                vec!["Item F", "Item G", "Item H"],
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

struct DragInfo {
    col_idx: usize,
    row_idx: usize,
}

impl super::View for DragAndDropDemo {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("This is a proof-of-concept of drag-and-drop in egui.");
        ui.label("Drag items between columns.");

        let id_source = "my_drag_and_drop_demo";

        ui.columns(self.columns.len(), |uis| {
            for (col_idx, column) in self.columns.clone().into_iter().enumerate() {
                let ui = &mut uis[col_idx];
                let can_accept_what_is_being_dragged =
                    DragAndDrop::has_payload::<DragInfo>(ui.ctx());

                let response = drop_target(ui, can_accept_what_is_being_dragged, |ui| {
                    ui.set_min_size(vec2(64.0, 100.0));
                    for (row_idx, item) in column.iter().enumerate() {
                        let item_id = Id::new(id_source).with(col_idx).with(row_idx);
                        let response = drag_source(ui, item_id, |ui| {
                            ui.add(Label::new(item).sense(Sense::click()));
                        });

                        if response.drag_started() {
                            DragAndDrop::set_payload(ui.ctx(), DragInfo { col_idx, row_idx });
                        }
                    }
                })
                .response;

                // NOTE: we use `response.contains_pointer` here instead of `hovered`, because
                // `hovered` is always false when another widget is being dragged.
                if response.contains_pointer() && ui.input(|i| i.pointer.any_released()) {
                    if let Some(source) = DragAndDrop::payload::<DragInfo>(ui.ctx()) {
                        let item = self.columns[source.col_idx].remove(source.row_idx);
                        self.columns[col_idx].push(item);
                    }
                }
            }
        });

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}
