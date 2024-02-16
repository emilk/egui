use egui::emath::TSTransform;

#[derive(Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PanZoom {
    transform: TSTransform,
}

impl Eq for PanZoom {}

impl super::Demo for PanZoom {
    fn name(&self) -> &'static str {
        "🗖 Pan Zoom"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        use super::View as _;
        let window = egui::Window::new("Pan Zoom")
            .default_width(200.0)
            .default_height(200.0)
            .vscroll(false)
            .open(open);
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for PanZoom {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let (id, rect) = ui.allocate_space(ui.available_size());
        let response = ui.interact(rect, id, egui::Sense::click_and_drag());
        // Allow dragging the background as well.
        self.transform.translation += response.drag_delta();

        // Plot-like reset
        if response.double_clicked() {
            self.transform = TSTransform::default();
        }

        if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            // Note: doesn't catch zooming / panning if a button in this PanZoom container is hovered.
            if response.hovered() {
                let pointer_in_layer = self.transform.inverse() * pointer;
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

        let current_size = ui.min_rect();
        for (pos, msg) in [
            (
                current_size.left_top() + egui::Vec2::new(10.0, 10.0),
                "top left!",
            ),
            (
                current_size.left_bottom() + egui::Vec2::new(10.0, -10.0),
                "bottom left?",
            ),
            (
                current_size.right_bottom() + egui::Vec2::new(-10.0, -10.0),
                "right bottom :D",
            ),
            (
                current_size.right_top() + egui::Vec2::new(-10.0, 10.0),
                "right top ):",
            ),
        ] {
            let id = egui::Area::new(msg)
                .default_pos(pos)
                // Need to cover up the pan_zoom demo window,
                // but may also cover over other windows.
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    ui.set_clip_rect(self.transform.inverse() * rect);
                    egui::Frame::default()
                        .rounding(egui::Rounding::same(4.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .stroke(ui.ctx().style().visuals.window_stroke)
                        .fill(ui.style().visuals.panel_fill)
                        .show(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.button(msg).clicked();
                        });
                })
                .response
                .layer_id;
            ui.ctx().set_transform_layer(id, self.transform);
        }
    }
}
