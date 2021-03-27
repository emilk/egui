use egui::plot::{Curve, Plot, Value};
use egui::*;
use std::f64::consts::TAU;

#[derive(PartialEq)]
pub struct PlotDemo {
    animate: bool,
    time: f64,
    circle_radius: f32,
    circle_center: Pos2,
    square: bool,
    proportional: bool,
}

impl Default for PlotDemo {
    fn default() -> Self {
        Self {
            animate: true,
            time: 0.0,
            circle_radius: 0.5,
            circle_center: Pos2::new(0.0, 0.0),
            square: false,
            proportional: true,
        }
    }
}

impl super::Demo for PlotDemo {
    fn name(&self) -> &'static str {
        "🗠 Plot"
    }

    fn show(&mut self, ctx: &CtxRef, open: &mut bool) {
        use super::View;
        Window::new(self.name())
            .open(open)
            .default_size(vec2(400.0, 400.0))
            .scroll(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl PlotDemo {
    fn options_ui(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            egui::reset_button(ui, self);
            ui.add(crate::__egui_github_link_file!());
        });
        ui.separator();

        let Self {
            animate,
            time: _,
            circle_radius,
            circle_center,
            square,
            proportional,
        } = self;

        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("Circle:");
                    ui.add(
                        egui::DragValue::new(circle_radius)
                            .speed(0.1)
                            .clamp_range(0.0..=f32::INFINITY)
                            // .logarithmic(true)
                            // .smallest_positive(1e-2)
                            .prefix("r: "),
                    );
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut circle_center.x)
                                .speed(0.1)
                                // .logarithmic(true)
                                // .smallest_positive(1e-2)
                                .prefix("x: "),
                        );
                        ui.add(
                            egui::DragValue::new(&mut circle_center.y)
                                .speed(1.0)
                                // .logarithmic(true)
                                // .smallest_positive(1e-2)
                                .prefix("y: "),
                        );
                    });
                });
            });

            ui.vertical(|ui| {
                ui.style_mut().wrap = Some(false);
                ui.checkbox(animate, "animate");
                ui.advance_cursor(8.0);
                ui.checkbox(square, "square view");
                ui.checkbox(proportional, "proportional data axes");
            });
        });
    }

    fn circle(&self) -> Curve {
        let n = 512;
        let circle = (0..=n).map(|i| {
            let t = remap(i as f64, 0.0..=(n as f64), 0.0..=TAU);
            let r = self.circle_radius as f64;
            Value::new(
                r * t.cos() + self.circle_center.x as f64,
                r * t.sin() + self.circle_center.y as f64,
            )
        });
        Curve::from_values_iter(circle)
            .color(Color32::from_rgb(100, 200, 100))
            .name("circle")
    }

    fn sin(&self) -> Curve {
        let n = 512;
        let circle = (0..=n).map(|i| {
            let t = remap(i as f64, 0.0..=(n as f64), -TAU..=TAU);
            Value::new(t / 5.0, 0.5 * (self.time + t).sin())
        });
        Curve::from_values_iter(circle)
            .color(Color32::from_rgb(200, 100, 100))
            .name("0.5 * sin(x / 5)")
    }

    fn thingy(&self) -> Curve {
        let n = 512;
        let complex_curve = (0..=n).map(|i| {
            let t = remap(i as f64, 0.0..=(n as f64), 0.0..=TAU);
            Value::new((2.0 * t + self.time).sin(), (3.0 * t).sin())
        });
        Curve::from_values_iter(complex_curve)
            .color(Color32::from_rgb(100, 150, 250))
            .name("x = sin(2t), y = sin(3t)")
    }
}

impl super::View for PlotDemo {
    fn ui(&mut self, ui: &mut Ui) {
        self.options_ui(ui);

        if self.animate {
            ui.ctx().request_repaint();
            self.time += ui.input().unstable_dt.at_most(1.0 / 30.0) as f64;
        };

        let mut plot = Plot::default()
            .curve(self.circle())
            .curve(self.sin())
            .curve(self.thingy())
            .min_size(Vec2::new(256.0, 200.0));
        if self.square {
            plot = plot.view_aspect(1.0);
        }
        if self.proportional {
            plot = plot.data_aspect(1.0);
        }
        ui.add(plot);
    }
}
