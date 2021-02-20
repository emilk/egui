use egui::{containers::*, *};

#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct DancingStrings {
    two_notes: TwoNotes,
}

impl super::Demo for DancingStrings {
    fn name(&self) -> &'static str {
        "♫ Dancing Strings"
    }

    fn show(&mut self, ctx: &CtxRef, open: &mut bool) {
        use super::View;
        Window::new(self.name())
            .open(open)
            .default_size(vec2(512.0, 256.0))
            .scroll(false)
            .show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for DancingStrings {
    fn ui(&mut self, ui: &mut Ui) {
        self.dancing_ui(ui);
        ui.separator();
        self.two_notes.ui(ui);
        ui.vertical_centered(|ui| {
            ui.add(crate::__egui_github_link_file!());
        });
    }
}

impl DancingStrings {
    fn dancing_ui(&mut self, ui: &mut Ui) {
        Frame::dark_canvas(ui.style()).show(ui, |ui| {
            ui.ctx().request_repaint();
            let time = ui.input().time;

            let desired_size = ui.available_width() * vec2(1.0, 0.35);
            let (_id, rect) = ui.allocate_space(desired_size);

            let to_screen =
                emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);

            let mut shapes = vec![];

            for &mode in &[2, 3, 5] {
                let mode = mode as f32;
                let n = 120;
                let speed = 1.5;

                let points: Vec<Pos2> = (0..=n)
                    .map(|i| {
                        let t = i as f32 / (n as f32);
                        let amp = (time as f32 * speed * mode).sin() / mode;
                        let y = amp * (t * std::f32::consts::TAU / 2.0 * mode).sin();
                        to_screen * pos2(t, y)
                    })
                    .collect();

                let thickness = 10.0 / mode;
                shapes.push(epaint::Shape::line(
                    points,
                    Stroke::new(thickness, Color32::from_additive_luminance(196)),
                ));
            }

            ui.painter().extend(shapes);
        });
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
struct TwoNotes {
    height: f32,
    num_repeats: u32,
    rel_freqs: Vec<f32>,
}

impl Default for TwoNotes {
    fn default() -> Self {
        Self {
            height: 75.0,
            num_repeats: 4,
            rel_freqs: vec![1.0, 3.0 / 2.0],
        }
    }
}

impl TwoNotes {
    pub fn ui(&mut self, ui: &mut Ui) {
        self.canvas_frame_ui(ui);
        self.control_ui(ui);
    }

    fn control_ui(&mut self, ui: &mut Ui) {
        let Self {
            height,
            num_repeats,
            rel_freqs,
        } = self;
        ui.style_mut().spacing.slider_width = 500.0;
        ui.add(Slider::f32(height, 10.0..=100.0).text("height"));
        ui.add(Slider::u32(num_repeats, 1..=10).text("num_repeats"));
        ui.add(Slider::f32(&mut rel_freqs[1], 1.0..=2.0).text("Frequency multiplier"));

        ui.horizontal(|ui| {
            let fractions = [(1, 1), (6, 5), (5, 4), (4, 3), (3, 2), (2, 1)];
            for &(t, n) in &fractions {
                if ui.button(format!("{} / {}", t, n)).clicked() {
                    rel_freqs[1] = (t as f32) / (n as f32);
                    *num_repeats = n;
                }
            }
        });
        if ui.button("√2").clicked() {
            rel_freqs[1] = 2.0_f32.sqrt();
        }
    }

    fn canvas_frame_ui(&mut self, ui: &mut Ui) {
        Frame::dark_canvas(ui.style()).show(ui, |ui| {
            self.canvas_content_ui(ui);
        });
    }

    fn canvas_content_ui(&self, ui: &mut Ui) {
        let Self {
            height,
            num_repeats,
            rel_freqs,
        } = self;

        ui.style_mut().spacing.item_spacing = Vec2::ZERO;

        let w = ui.available_width();

        let fundamental_wavelength = w / (*num_repeats as f32);
        let fundamental_freq = 1.0 / fundamental_wavelength;

        let mut shapes = vec![];

        for overtone in 1.. {
            let h = *height / overtone as f32;
            if h <= 1.5 {
                break;
            }
            for (tone_nr, rel_freq) in rel_freqs.iter().enumerate() {
                let freq = fundamental_freq * rel_freq * (overtone as f32);
                let wavelen = 1.0 / freq;
                let stroke_width = h.sqrt();

                // let h = wavelen / 4.0;

                if wavelen <= 1.5 {
                    break;
                }

                let (rect, response) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
                response.on_hover_text(format!("Tone {} * overtone {}", tone_nr, overtone));

                for i in 0.. {
                    let x = rect.left() + wavelen * (i as f32);
                    if x > rect.right() + 0.5 {
                        break;
                    }

                    let stroke = Stroke::new(stroke_width, Color32::from_additive_luminance(150));
                    shapes.push(Shape::line_segment(
                        [pos2(x, rect.top()), pos2(x, rect.bottom())],
                        stroke,
                    ));
                }
            }
        }
        ui.painter().extend(shapes);
    }
}
