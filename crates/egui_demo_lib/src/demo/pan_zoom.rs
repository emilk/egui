use egui::{Area, Sense};

#[derive(Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PanZoom {
    pan: egui::Vec2,
    zoom: f32,
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
        // On initialization, zoom is 0
        if self.zoom == 0.0 {
            self.zoom = 1.0;
        }

        let (id, rect) = ui.allocate_space(ui.available_size());
        let response = ui.interact(rect, id, egui::Sense::click_and_drag());
        // Uncomment to allow dragging the background as well.
        // self.pan += response.drag_delta() / self.zoom;

        // Plot-like reset
        if response.double_clicked() {
            self.zoom = 1.0;
            self.pan = egui::Vec2::ZERO;
        }

        if let Some(pointer) = ui.ctx().input(|i| i.pointer.hover_pos()) {
            // Ignore if some other widget is covering this container.
            if response.rect.contains(pointer) {
                let original_zoom = self.zoom;
                self.zoom *= ui.ctx().input(|i| i.zoom_delta());
                let delta = pointer / self.zoom - pointer / original_zoom;
                self.pan += delta;

                // Keep mouse centered.
                self.pan += ui.ctx().input(|i| i.raw_scroll_delta) / self.zoom;
            }
        }

        let current_size = ui.min_rect();
        let layer_ids = [
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
        ]
        .iter()
        .map(|(pos, msg)| {
            Area::new(*msg)
                .default_pos(*pos)
                // Need to cover up the pan_zoom demo window,
                // but may also cover over other windows.
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    let rect = egui::Rect::from_min_max(
                        (rect.min / self.zoom) - self.pan,
                        (rect.max / self.zoom) - self.pan,
                    );
                    ui.set_clip_rect(rect);
                    egui::Frame::default()
                        .rounding(egui::Rounding::same(4.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .stroke(ui.ctx().style().visuals.window_stroke)
                        .fill(ui.style().visuals.panel_fill)
                        .show(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.button(*msg).clicked();
                        });
                })
                .response
                .layer_id
        })
        .for_each(|id| {
            ui.ctx().transform_layer(id, self.pan, self.zoom);
        });
    }
}
