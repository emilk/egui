use egui::{color::*, *};

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
enum Enum {
    First,
    Second,
    Third,
}

impl Default for Enum {
    fn default() -> Self {
        Enum::First
    }
}

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Widgets {
    button_enabled: bool,
    count: usize,
    radio: Enum,
    sliders: super::Sliders,
    angle: f32,
    color: Color32,
    single_line_text_input: String,
    multiline_text_input: String,
    toggle_switch: bool,
}

impl Default for Widgets {
    fn default() -> Self {
        Self {
            button_enabled: true,
            radio: Enum::First,
            count: 0,
            sliders: Default::default(),
            angle: std::f32::consts::TAU / 3.0,
            color: (Rgba::from_rgb(0.0, 1.0, 0.5) * 0.75).into(),
            single_line_text_input: "Hello World!".to_owned(),
            multiline_text_input: "Text can both be so wide that it needs a line break, but you can also add manual line break by pressing enter, creating new paragraphs.\nThis is the start of the next paragraph.\n\nClick me to edit me!".to_owned(),
            toggle_switch: false,
        }
    }
}

impl Widgets {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.add(crate::__egui_github_link_file_line!());

        ui.horizontal_wrapped_for_text(TextStyle::Body, |ui| {
            ui.add(Label::new("Text can have").text_color(Color32::from_rgb(110, 255, 110)));
            ui.colored_label(Color32::from_rgb(128, 140, 255), "color"); // Shortcut version
            ui.label("and tooltips.").on_hover_text(
                "This is a multiline tooltip that demonstrates that you can easily add tooltips to any element.\nThis is the second line.\nThis is the third.",
            );

            ui.label("You can mix in other widgets into text, like");
            let _ = ui.small_button("this button");
            ui.label(".");

            ui.label("The default font supports all latin and cyrillic characters (ИÅđ…), common math symbols (∫√∞²⅓…), and many emojis (💓🌟🖩…).")
                .on_hover_text("There is currently no support for right-to-left languages.");
            ui.label("See the 🔤 Font Book for more!");

            ui.monospace("There is also a monospace font.");
        });

        let tooltip_ui = |ui: &mut Ui| {
            ui.heading("The name of the tooltip");
            ui.horizontal(|ui| {
                ui.label("This tooltip was created with");
                ui.monospace(".on_hover_ui(...)");
            });
            let _ = ui.button("A button you can never press");
        };
        ui.label("Tooltips can be more than just simple text.")
            .on_hover_ui(tooltip_ui);

        ui.horizontal(|ui| {
            ui.radio_value(&mut self.radio, Enum::First, "First");
            ui.radio_value(&mut self.radio, Enum::Second, "Second");
            ui.radio_value(&mut self.radio, Enum::Third, "Third");
        });

        egui::combo_box_with_label(ui, "Combo Box", format!("{:?}", self.radio), |ui| {
            ui.selectable_value(&mut self.radio, Enum::First, "First");
            ui.selectable_value(&mut self.radio, Enum::Second, "Second");
            ui.selectable_value(&mut self.radio, Enum::Third, "Third");
        });

        ui.checkbox(&mut self.button_enabled, "Button enabled");

        ui.horizontal(|ui| {
            if ui
                .add(Button::new("Click me").enabled(self.button_enabled))
                .on_hover_text("This will just increase a counter.")
                .clicked
            {
                self.count += 1;
            }
            ui.label(format!("The button has been clicked {} times.", self.count));
        });

        ui.separator();
        {
            ui.horizontal(|ui| {
                ui.label("Drag this value to change it:");
                ui.add(DragValue::f64(&mut self.sliders.value).speed(0.01));
            });

            ui.add(
                egui::Slider::f64(&mut self.sliders.value, 1.0..=100.0)
                    .logarithmic(true)
                    .text("A slider"),
            );

            egui::CollapsingHeader::new("More sliders")
                .default_open(false)
                .show(ui, |ui| {
                    self.sliders.ui(ui);
                });
        }

        ui.separator();

        ui.horizontal_for_text(TextStyle::Body, |ui| {
            ui.label("An angle:");
            ui.drag_angle(&mut self.angle);
            ui.label(format!("≈ {:.3}τ", self.angle / std::f32::consts::TAU))
                .on_hover_text("Each τ represents one turn (τ = 2π)");
        })
        .1
        .on_hover_text("The angle is stored in radians, but presented in degrees");

        ui.separator();

        ui.horizontal(|ui| {
            ui.colored_label(self.color, "Click to select a different text color: ");
            ui.color_edit_button_srgba(&mut self.color);
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Single line text input:");
            let response = ui.text_edit_singleline(&mut self.single_line_text_input);
            if response.lost_kb_focus {
                // The user pressed enter.
            }
        });

        ui.label("Multiline text input:");
        ui.text_edit_multiline(&mut self.multiline_text_input);

        ui.separator();
        super::toggle_switch::demo(ui, &mut self.toggle_switch);
    }
}
