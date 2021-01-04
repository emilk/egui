use egui::*;
use std::f64::INFINITY;

/// Showcase sliders
#[derive(PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Sliders {
    pub min: f64,
    pub max: f64,
    pub logarithmic: bool,
    pub smart_aim: bool,
    pub integer: bool,
    pub value: f64,
}

impl Default for Sliders {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 10000.0,
            logarithmic: true,
            smart_aim: true,
            integer: false,
            value: 10.0,
        }
    }
}

impl Sliders {
    pub fn ui(&mut self, ui: &mut Ui) {
        let Self {
            min,
            max,
            logarithmic,
            smart_aim,
            integer,
            value,
        } = self;

        ui.label("You can click a slider value to edit it with the keyboard.");

        let full_range = if *integer {
            (i32::MIN as f64)..=(i32::MAX as f64)
        } else if *logarithmic {
            -INFINITY..=INFINITY
        } else {
            -1e5..=1e5 // linear sliders make little sense with huge numbers
        };

        *min = clamp(*min, full_range.clone());
        *max = clamp(*max, full_range.clone());

        if *integer {
            let mut value_i32 = *value as i32;
            ui.add(
                Slider::i32(&mut value_i32, (*min as i32)..=(*max as i32))
                    .logarithmic(*logarithmic)
                    .smart_aim(*smart_aim)
                    .text("i32 demo slider"),
            );
            *value = value_i32 as f64;
        } else {
            ui.add(
                Slider::f64(value, (*min)..=(*max))
                    .logarithmic(*logarithmic)
                    .smart_aim(*smart_aim)
                    .text("f64 demo slider"),
            );

            ui.label(
                "Sliders will intelligently pick how many decimals to show. \
                You can always see the full precision value by hovering the value.",
            );

            if ui.button("Assign PI").clicked {
                self.value = std::f64::consts::PI;
            }
        }

        ui.separator();
        ui.label("Demo slider range:");
        ui.add(
            Slider::f64(min, full_range.clone())
                .logarithmic(true)
                .smart_aim(*smart_aim)
                .text("left"),
        );
        ui.add(
            Slider::f64(max, full_range)
                .logarithmic(true)
                .smart_aim(*smart_aim)
                .text("right"),
        );

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Slider type:");
            ui.radio_value(integer, true, "i32");
            ui.radio_value(integer, false, "f64");
        });
        ui.label("(f32, usize etc are also possible)");

        ui.checkbox(logarithmic, "Logarithmic");
        ui.label("Logarithmic sliders are great for when you want to span a huge range, i.e. from zero to a million.");
        ui.label("Logarithmic sliders can include infinity and zero.");

        ui.checkbox(smart_aim, "Smart Aim");
        ui.label("Smart Aim will guide you towards round values when you drag the slider so you you are more likely to hit 250 than 247.23");

        egui::reset_button(ui, self);
    }
}
