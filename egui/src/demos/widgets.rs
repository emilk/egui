use crate::{color::*, *};

#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Widgets {
    button_enabled: bool,
    count: usize,
    radio: Enum,
    slider_value: f32,
    angle: f32,
    color: Srgba,
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
            slider_value: 3.4,
            angle: TAU / 8.0,
            color: (Rgba::new(0.0, 1.0, 0.5, 1.0) * 0.75).into(),
            single_line_text_input: "Hello World!".to_owned(),
            multiline_text_input: "Text can both be so wide that it needs a line break, but you can also add manual line break by pressing enter, creating new paragraphs.\nThis is the start of the next paragraph.\n\nClick me to edit me!".to_owned(),
            toggle_switch: false,
        }
    }
}

impl Widgets {
    pub fn ui(&mut self, ui: &mut Ui) {
        let url = format!("https://github.com/emilk/egui/blob/master/{}", file!());
        ui.add(Hyperlink::new(url).text("Click here to read the source code for this section"));

        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 0.0;
            ui.add(label!("Text can have ").text_color(srgba(110, 255, 110, 255)));
            ui.add(label!("color ").text_color(srgba(128, 140, 255, 255)));
            ui.add(label!("and tooltips")).tooltip_text(
                "This is a multiline tooltip that demonstrates that you can easily add tooltips to any element.\nThis is the second line.\nThis is the third.",
            );
        });

        ui.add(label!("Some non-latin characters: Ευρηκα τ = 2×π"))
            .tooltip_text("The current font supports only a few non-latin characters and Egui does not currently support right-to-left text.");

        ui.horizontal(|ui| {
            ui.radio_value("First", &mut self.radio, Enum::First);
            ui.radio_value("Second", &mut self.radio, Enum::Second);
            ui.radio_value("Third", &mut self.radio, Enum::Third);
        });

        combo_box_with_label(ui, "Combo Box", format!("{:?}", self.radio), |ui| {
            ui.radio_value("First", &mut self.radio, Enum::First);
            ui.radio_value("Second", &mut self.radio, Enum::Second);
            ui.radio_value("Third", &mut self.radio, Enum::Third);
        });

        ui.add(Checkbox::new(&mut self.button_enabled, "Button enabled"));

        ui.horizontal(|ui| {
            if ui
                .add(Button::new("Click me").enabled(self.button_enabled))
                .tooltip_text("This will just increase a counter.")
                .clicked
            {
                self.count += 1;
            }
            ui.add(label!("The button has been clicked {} times", self.count));
        });

        ui.separator();
        {
            ui.label(
                "The slider will show as many decimals as needed, \
                and will intelligently help you select a round number when you interact with it.\n\
                You can click a slider value to edit it with the keyboard.",
            );
            ui.add(Slider::f32(&mut self.slider_value, -10.0..=10.0).text("value"));
            ui.horizontal(|ui| {
                ui.label("More compact as a value you drag:");
                ui.add(DragValue::f32(&mut self.slider_value).speed(0.01));
            });
            if ui.add(Button::new("Assign PI")).clicked {
                self.slider_value = std::f32::consts::PI;
            }
        }
        ui.separator();
        {
            ui.label("An angle stored as radians, but edited in degrees:");
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 0.0;
                ui.drag_angle(&mut self.angle);
                ui.label(format!(" = {} radians", self.angle));
            });
        }
        ui.separator();

        ui.horizontal(|ui| {
            ui.add(Label::new("Click to select a different text color: ").text_color(self.color));
            ui.color_edit_button_srgba(&mut self.color);
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.add(label!("Single line text input:"));
            ui.add(
                TextEdit::new(&mut self.single_line_text_input)
                    .multiline(false)
                    .id_source("single line"),
            );
        }); // TODO: .tooltip_text("Enter text to edit me")

        ui.add(label!("Multiline text input:"));
        ui.add(TextEdit::new(&mut self.multiline_text_input).id_source("multiline"));

        ui.separator();
        super::toggle_switch::demo(ui, &mut self.toggle_switch);
    }
}
