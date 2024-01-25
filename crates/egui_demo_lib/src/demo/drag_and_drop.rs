use egui::*;

fn drop_zone<R>(
    ui: &mut Ui,
    can_accept_what_is_being_dragged: bool,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let is_anything_being_dragged = DragAndDrop::has_any_payload(ui.ctx());

    let frame = Frame::default().inner_margin(4.0);
    let mut frame = frame.begin(ui);
    let ret = add_contents(&mut frame.content_ui);
    let response = frame.allocate_space(ui);

    // NOTE: we use `response.contains_pointer` here instead of `hovered`, because
    // `hovered` is always false when another widget is being dragged.
    let style = if is_anything_being_dragged
        && can_accept_what_is_being_dragged
        && response.contains_pointer()
    {
        ui.visuals().widgets.active
    } else {
        ui.visuals().widgets.inactive
    };

    let mut fill = style.bg_fill;
    let mut stroke = style.bg_stroke;

    if is_anything_being_dragged && !can_accept_what_is_being_dragged {
        // When dragging something else, show that it can't be dropped here.
        fill = ui.visuals().gray_out(fill);
        stroke.color = ui.visuals().gray_out(stroke.color);
    }

    frame.frame.fill = fill;
    frame.frame.stroke = stroke;

    frame.paint(ui);

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
        ui.label("This is a simple example of drag-and-drop in egui.");
        ui.label("Drag items between columns.");

        let id_source = "my_drag_and_drop_demo";

        ui.columns(self.columns.len(), |uis| {
            for (col_idx, column) in self.columns.clone().into_iter().enumerate() {
                let ui = &mut uis[col_idx];
                let can_accept_what_is_being_dragged =
                    DragAndDrop::has_payload_of_type::<DragInfo>(ui.ctx());

                let response = drop_zone(ui, can_accept_what_is_being_dragged, |ui| {
                    ui.set_min_size(vec2(64.0, 100.0));
                    for (row_idx, item) in column.iter().enumerate() {
                        let item_id = Id::new(id_source).with(col_idx).with(row_idx);
                        let payload = DragInfo { col_idx, row_idx };
                        ui.dnd_drag_source(item_id, payload, |ui| {
                            ui.add(Label::new(item).sense(Sense::click()));
                        });
                    }
                })
                .response;

                if let Some(source) = response.dnd_release_payload::<DragInfo>() {
                    let item = self.columns[source.col_idx].remove(source.row_idx);
                    self.columns[col_idx].push(item);
                }
            }
        });

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}
