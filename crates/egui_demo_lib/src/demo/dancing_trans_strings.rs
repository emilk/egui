use colorgrad::{Color, CustomGradient, Gradient};
use egui::{containers::*, epaint::PathStroke, *};
use noise::{NoiseFn, OpenSimplex};
use once_cell::sync::Lazy;

static GRADIENT: Lazy<Gradient> = Lazy::new(|| {
    CustomGradient::new()
        .colors(&[
            Color::from_html("#5BCEFA").unwrap(),
            Color::from_html("#F5A9B8").unwrap(),
            Color::from_html("#FFFFFF").unwrap(),
            Color::from_html("#F5A9B8").unwrap(),
            Color::from_html("#5BCEFA").unwrap(),
        ])
        .build()
        .unwrap_or(colorgrad::rainbow())
});
static NOISE: Lazy<OpenSimplex> = Lazy::new(|| OpenSimplex::new(6940));

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DancingStrings {}

impl super::Demo for DancingStrings {
    fn name(&self) -> &'static str {
        "♫ Dancing Strings (Colored)"
    }

    fn show(&mut self, ctx: &Context, open: &mut bool) {
        use super::View as _;
        Window::new(self.name())
            .open(open)
            .default_size(vec2(512.0, 256.0))
            .vscroll(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for DancingStrings {
    fn ui(&mut self, ui: &mut Ui) {
        Frame::canvas(ui.style()).show(ui, |ui| {
            ui.ctx().request_repaint();
            let time = ui.input(|i| i.time);

            let desired_size = ui.available_width() * vec2(1.0, 0.35);
            let (_id, rect) = ui.allocate_space(desired_size);

            let to_screen =
                emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);

            let mut shapes = vec![];

            for &mode in &[2, 3, 5] {
                let mode = mode as f64;
                let n = 120;
                let speed = 1.5;

                let points: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let t = i as f64 / (n as f64);
                        let amp = (time * speed * mode).sin() / mode;
                        let y = amp * (t * std::f64::consts::TAU / 2.0 * mode).sin();
                        to_screen * pos2(t as f32, y as f32)
                    })
                    .collect();

                let thickness = 10.0 / mode as f32;
                shapes.push(epaint::Shape::line(
                    points,
                    PathStroke::new_uv(thickness, move |_r, p| {
                        let time = time / 10.0;
                        let x = remap(p.x, rect.x_range(), 0.0..=1.0) as f64;
                        let y = remap(p.y, rect.y_range(), 0.0..=1.0) as f64;

                        let noise = NOISE.get([x * 1.25 + time, y * 1.25 + time]);
                        let color = GRADIENT.at(noise).to_rgba8();

                        Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3])
                    }),
                ));
            }

            ui.painter().extend(shapes);
        });
        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}
