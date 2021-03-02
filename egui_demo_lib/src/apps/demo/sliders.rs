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
    pub clamp_to_range: bool,
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
            clamp_to_range: false,
            smart_aim: true,
            integer: false,
            value: 10.0,
        }
    }
}

impl super::Demo for Sliders {
    fn name(&self) -> &'static str {
        "⬌ Sliders"
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .show(ctx, |ui| {
                use super::View;
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
                    .clamp_to_range(*clamp_to_range)
                    .smart_aim(*smart_aim)
                    .text("i32 demo slider"),
            );
            *value = value_i32 as f64;
        } else {
            ui.add(
                Slider::f64(value, (*min)..=(*max))
                    .logarithmic(*logarithmic)
                    .clamp_to_range(*clamp_to_range)
                    .smart_aim(*smart_aim)
                    .text("f64 demo slider"),
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
        ui.advance_cursor(8.0);

        ui.checkbox(logarithmic, "Logarithmic");
        ui.label("Logarithmic sliders are great for when you want to span a huge range, i.e. from zero to a million.");
        ui.label("Logarithmic sliders can include infinity and zero.");
        ui.advance_cursor(8.0);

        ui.checkbox(clamp_to_range, "Clamp to range");
        ui.label("If true, the slider will clamp incoming and outgoing values to the given range.");
        ui.label("If false, the slider can shows values outside its range, and you can manually enter values outside the range.");
        ui.advance_cursor(8.0);

        ui.checkbox(smart_aim, "Smart Aim");
        ui.label("Smart Aim will guide you towards round values when you drag the slider so you you are more likely to hit 250 than 247.23");
        ui.advance_cursor(8.0);

        ui.vertical_centered(|ui| {
            egui::reset_button(ui, self);
            ui.add(crate::__egui_github_link_file!());
        });
    }
}
