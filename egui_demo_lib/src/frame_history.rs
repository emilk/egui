use egui::util::History;

pub struct FrameHistory {
    frame_times: History<f32>,
}

impl Default for FrameHistory {
    fn default() -> Self {
        let max_age: f64 = 1.0;
        Self {
            frame_times: History::from_max_len_age((max_age * 300.0).round() as usize, max_age),
        }
    }
}

impl FrameHistory {
    // Called first
    pub fn on_new_frame(&mut self, now: f64, previus_frame_time: Option<f32>) {
        let previus_frame_time = previus_frame_time.unwrap_or_default();
        if let Some(latest) = self.frame_times.latest_mut() {
            *latest = previus_frame_time; // rewrite history now that we know
        }
        self.frame_times.add(now, previus_frame_time); // projected
    }

    pub fn mean_frame_time(&self) -> f32 {
        self.frame_times.average().unwrap_or_default()
    }

    pub fn fps(&self) -> f32 {
        1.0 / self.frame_times.mean_time_interval().unwrap_or_default()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Total frames painted: {}",
            self.frame_times.total_count()
        ))
        .on_hover_text("Includes this frame.");

        ui.label(format!(
            "Mean CPU usage: {:.2} ms / frame",
            1e3 * self.mean_frame_time()
        ))
        .on_hover_text(
            "Includes egui layout and tessellation time.\n\
            Does not include GPU usage, nor overhead for sending data to GPU.",
        );
        egui::warn_if_debug_build(ui);

        egui::CollapsingHeader::new("📊 CPU usage history")
            .default_open(false)
            .show(ui, |ui| {
                self.graph(ui);
            });
    }

    fn graph(&mut self, ui: &mut egui::Ui) -> egui::Response {
        use egui::*;

        let graph_top_cpu_usage = 0.010;
        ui.label("egui CPU usage history");

        let history = &self.frame_times;

        // TODO: we should not use `slider_width` as default graph width.
        let height = ui.style().spacing.slider_width;
        let size = vec2(ui.available_size_before_wrap_finite().x, height);
        let (rect, response) = ui.allocate_at_least(size, Sense::hover());
        let style = ui.style().noninteractive();

        let mut shapes = vec![Shape::Rect {
            rect,
            corner_radius: style.corner_radius,
            fill: ui.style().visuals.dark_bg_color,
            stroke: ui.style().noninteractive().bg_stroke,
        }];

        let rect = rect.shrink(4.0);
        let line_stroke = Stroke::new(1.0, Color32::from_additive_luminance(128));

        if let Some(pointer_pos) = ui.input().pointer.tooltip_pos() {
            if rect.contains(pointer_pos) {
                let y = pointer_pos.y;
                shapes.push(Shape::line_segment(
                    [pos2(rect.left(), y), pos2(rect.right(), y)],
                    line_stroke,
                ));
                let cpu_usage = remap(y, rect.bottom_up_range(), 0.0..=graph_top_cpu_usage);
                let text = format!("{:.1} ms", 1e3 * cpu_usage);
                shapes.push(Shape::text(
                    ui.fonts(),
                    pos2(rect.left(), y),
                    egui::Align2::LEFT_BOTTOM,
                    text,
                    TextStyle::Monospace,
                    Color32::WHITE,
                ));
            }
        }

        let circle_color = Color32::from_additive_luminance(196);
        let radius = 2.0;
        let right_side_time = ui.input().time; // Time at right side of screen

        for (time, cpu_usage) in history.iter() {
            let age = (right_side_time - time) as f32;
            let x = remap(age, history.max_age()..=0.0, rect.x_range());
            let y = remap_clamp(cpu_usage, 0.0..=graph_top_cpu_usage, rect.bottom_up_range());

            shapes.push(Shape::line_segment(
                [pos2(x, rect.bottom()), pos2(x, y)],
                line_stroke,
            ));

            if cpu_usage < graph_top_cpu_usage {
                shapes.push(Shape::circle_filled(pos2(x, y), radius, circle_color));
            }
        }

        ui.painter().extend(shapes);

        response
    }
}
