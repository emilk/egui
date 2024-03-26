use egui::emath::TSTransform;

#[derive(Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PanZoom {
    transform: TSTransform,
    drag_value: f32,
}

impl Eq for PanZoom {}

impl super::Demo for PanZoom {
    fn name(&self) -> &'static str {
        "🔍 Pan Zoom"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        use super::View as _;
        let window = egui::Window::new("Pan Zoom")
            .default_width(300.0)
            .default_height(300.0)
            .vscroll(false)
            .open(open);
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for PanZoom {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "Pan, zoom in, and zoom out with scrolling (see the plot demo for more instructions). \
                   Double click on the background to reset.",
        );
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
        ui.separator();

        let (id, rect) = ui.allocate_space(ui.available_size());
        let response = ui.interact(rect, id, egui::Sense::click_and_drag());
        // Allow dragging the background as well.
        if response.dragged() {
            self.transform.translation += response.drag_delta();
        }

        // Plot-like reset
        if response.double_clicked() {
            self.transform = TSTransform::default();
        }

        let transform =
            TSTransform::from_translation(ui.min_rect().left_top().to_vec2()) * self.transform;

        if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            // Note: doesn't catch zooming / panning if a button in this PanZoom container is hovered.
            if response.hovered() {
                let pointer_in_layer = transform.inverse() * pointer;
                let zoom_delta = ui.ctx().input(|i| i.zoom_delta());
                let pan_delta = ui.ctx().input(|i| i.smooth_scroll_delta);

                // Zoom in on pointer:
                self.transform = self.transform
                    * TSTransform::from_translation(pointer_in_layer.to_vec2())
                    * TSTransform::from_scaling(zoom_delta)
                    * TSTransform::from_translation(-pointer_in_layer.to_vec2());

                // Pan:
                self.transform = TSTransform::from_translation(pan_delta) * self.transform;
            }
        }

        for (i, (pos, callback)) in [
            (
                egui::Pos2::new(0.0, 0.0),
                Box::new(|ui: &mut egui::Ui, _: &mut Self| ui.button("top left!"))
                    as Box<dyn Fn(&mut egui::Ui, &mut Self) -> egui::Response>,
            ),
            (
                egui::Pos2::new(0.0, 120.0),
                Box::new(|ui: &mut egui::Ui, _| ui.button("bottom left?")),
            ),
            (
                egui::Pos2::new(120.0, 120.0),
                Box::new(|ui: &mut egui::Ui, _| ui.button("right bottom :D")),
            ),
            (
                egui::Pos2::new(120.0, 0.0),
                Box::new(|ui: &mut egui::Ui, _| ui.button("right top ):")),
            ),
            (
                egui::Pos2::new(60.0, 60.0),
                Box::new(|ui, state| {
                    use egui::epaint::*;
                    // Smiley face.
                    let painter = ui.painter();
                    painter.add(CircleShape::filled(pos2(0.0, -10.0), 1.0, Color32::YELLOW));
                    painter.add(CircleShape::filled(pos2(10.0, -10.0), 1.0, Color32::YELLOW));
                    painter.add(QuadraticBezierShape::from_points_stroke(
                        [pos2(0.0, 0.0), pos2(5.0, 3.0), pos2(10.0, 0.0)],
                        false,
                        Color32::TRANSPARENT,
                        Stroke::new(1.0, Color32::YELLOW),
                    ));

                    ui.add(egui::Slider::new(&mut state.drag_value, 0.0..=100.0).text("My value"))
                }),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let id = egui::Area::new(id.with(("subarea", i)))
                .default_pos(pos)
                // Need to cover up the pan_zoom demo window,
                // but may also cover over other windows.
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    ui.set_clip_rect(transform.inverse() * rect);
                    egui::Frame::default()
                        .rounding(egui::Rounding::same(4.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .stroke(ui.ctx().style().visuals.window_stroke)
                        .fill(ui.style().visuals.panel_fill)
                        .show(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            callback(ui, self)
                        });
                })
                .response
                .layer_id;
            ui.ctx().set_transform_layer(id, transform);
        }
    }
}
