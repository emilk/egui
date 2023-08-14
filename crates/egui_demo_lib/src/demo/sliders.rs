use egui::*;
use std::f64::INFINITY;

/// Showcase sliders
#[derive(PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Sliders {
    pub min: f64,
    pub max: f64,
    pub logarithmic: bool,
    pub clamp_to_range: bool,
    pub smart_aim: bool,
    pub step: f64,
    pub use_steps: bool,
    pub integer: bool,
    pub vertical: bool,
    pub value: f64,
    pub trailing_fill: bool,
}

impl Default for Sliders {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 10000.0,
            logarithmic: true,
            clamp_to_range: false,
            smart_aim: true,
            step: 10.0,
            use_steps: false,
            integer: false,
            vertical: false,
            value: 10.0,
            trailing_fill: false,
        }
    }
}

impl super::Demo for Sliders {
    fn name(&self) -> &'static str {
        "⬌ Sliders"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .show(ctx, |ui| {
                use super::View as _;
                self.ui(ui);
            });
    }
}

impl super::View for Sliders {
    fn ui(&mut self, ui: &mut Ui) {
        let Self {
            min,
            max,
            logarithmic,
            clamp_to_range,
            smart_aim,
            step,
            use_steps,
            integer,
            vertical,
            value,
            trailing_fill,
        } = self;

        ui.label("You can click a slider value to edit it with the keyboard.");

        let (type_min, type_max) = if *integer {
            ((i32::MIN as f64), (i32::MAX as f64))
        } else if *logarithmic {
            (-INFINITY, INFINITY)
        } else {
            (-1e5, 1e5) // linear sliders make little sense with huge numbers
        };

        *min = min.clamp(type_min, type_max);
        *max = max.clamp(type_min, type_max);

        let orientation = if *vertical {
            SliderOrientation::Vertical
        } else {
            SliderOrientation::Horizontal
        };

        let istep = if *use_steps { *step } else { 0.0 };
        if *integer {
            let mut value_i32 = *value as i32;
            ui.add(
                Slider::new(&mut value_i32, (*min as i32)..=(*max as i32))
                    .logarithmic(*logarithmic)
                    .clamp_to_range(*clamp_to_range)
                    .smart_aim(*smart_aim)
                    .orientation(orientation)
                    .text("i32 demo slider")
                    .step_by(istep)
                    .trailing_fill(*trailing_fill),
            );
            *value = value_i32 as f64;
        } else {
            ui.add(
                Slider::new(value, (*min)..=(*max))
                    .logarithmic(*logarithmic)
                    .clamp_to_range(*clamp_to_range)
                    .smart_aim(*smart_aim)
                    .orientation(orientation)
                    .text("f64 demo slider")
                    .step_by(istep)
                    .trailing_fill(*trailing_fill),
            );

            ui.label(
                "Sliders will intelligently pick how many decimals to show. \
                You can always see the full precision value by hovering the value.",
            );

            if ui.button("Assign PI").clicked() {
                self.value = std::f64::consts::PI;
            }
        }

        ui.separator();

        ui.label("Slider range:");
        ui.add(
            Slider::new(min, type_min..=type_max)
                .logarithmic(true)
                .smart_aim(*smart_aim)
                .text("left")
                .trailing_fill(*trailing_fill),
        );
        ui.add(
            Slider::new(max, type_min..=type_max)
                .logarithmic(true)
                .smart_aim(*smart_aim)
                .text("right")
                .trailing_fill(*trailing_fill),
        );

        ui.separator();

        ui.checkbox(trailing_fill, "Toggle trailing color");
        ui.label("When enabled, trailing color will be painted up until the circle.");

        ui.separator();

        ui.checkbox(use_steps, "Use steps");
        ui.label("When enabled, the minimal value change would be restricted to a given step.");
        if *use_steps {
            ui.add(egui::DragValue::new(step).speed(1.0));
        }

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Slider type:");
            ui.radio_value(integer, true, "i32");
            ui.radio_value(integer, false, "f64");
        })
        .response
        .on_hover_text("All numeric types (f32, usize, …) are supported.");

        ui.horizontal(|ui| {
            ui.label("Slider orientation:");
            ui.radio_value(vertical, false, "Horizontal");
            ui.radio_value(vertical, true, "Vertical");
        });
        ui.add_space(8.0);

        ui.checkbox(logarithmic, "Logarithmic");
        ui.label("Logarithmic sliders are great for when you want to span a huge range, i.e. from zero to a million.");
        ui.label("Logarithmic sliders can include infinity and zero.");
        ui.add_space(8.0);

        ui.checkbox(clamp_to_range, "Clamp to range");
        ui.label("If true, the slider will clamp incoming and outgoing values to the given range.");
        ui.label("If false, the slider can shows values outside its range, and you can manually enter values outside the range.");
        ui.add_space(8.0);

        ui.checkbox(smart_aim, "Smart Aim");
        ui.label("Smart Aim will guide you towards round values when you drag the slider so you you are more likely to hit 250 than 247.23");
        ui.add_space(8.0);

        ui.vertical_centered(|ui| {
            egui::reset_button(ui, self);
            ui.add(crate::egui_github_link_file!());
        });
    }
}
