use crate::{containers::*, demos::*, *};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct DancingStrings {}

impl Default for DancingStrings {
    fn default() -> Self {
        Self {}
    }
}

impl Demo for DancingStrings {
    fn name(&self) -> &str {
        "♫ Dancing Strings"
    }

    fn show(&mut self, ctx: &CtxRef, open: &mut bool) {
        Window::new(self.name())
            .open(open)
            .default_size(vec2(512.0, 256.0))
            .scroll(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl View for DancingStrings {
    fn ui(&mut self, ui: &mut Ui) {
        Frame::dark_canvas(ui.style()).show(ui, |ui| {
            ui.ctx().request_repaint();
            let time = ui.input().time;

            let desired_size = ui.available_width() * vec2(1.0, 0.35);
            let (_id, rect) = ui.allocate_space(desired_size);

            let mut cmds = vec![];

            for &mode in &[2, 3, 5] {
                let mode = mode as f32;
                let n = 120;
                let speed = 1.5;

                let points: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let t = i as f32 / (n as f32);
                        let amp = (time as f32 * speed * mode).sin() / mode;
                        let y = amp * (t * std::f32::consts::TAU / 2.0 * mode).sin();

                        pos2(
                            lerp(rect.x_range(), t),
                            remap(y, -1.0..=1.0, rect.y_range()),
                        )
                    })
                    .collect();

                let thickness = 10.0 / mode;
                cmds.push(paint::PaintCmd::line(
                    points,
                    Stroke::new(thickness, Srgba::additive_luminance(196)),
                ));
            }

            ui.painter().extend(cmds);
        });
        ui.add(__egui_github_link_file!());
    }
}
