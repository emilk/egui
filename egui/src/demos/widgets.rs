use crate::{color::*, demos::Sliders, *};

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
    sliders: Sliders,
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
            sliders: Default::default(),
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
        ui.add(crate::__egui_github_link_file_line!());

        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 0.0;
            ui.add(Label::new("Text can have ").text_color(srgba(110, 255, 110, 255)));
            ui.add(Label::new("color ").text_color(srgba(128, 140, 255, 255)));
            ui.add(Label::new("and tooltips.")).on_hover_text(
                "This is a multiline tooltip that demonstrates that you can easily add tooltips to any element.\nThis is the second line.\nThis is the third.",
            );
        });
        ui.label("Tooltips can be more than just simple text.")
            .on_hover_ui(|ui| {
                ui.heading("The name of the tooltip");
                ui.horizontal(|ui| {
                    ui.label("This tooltip was created with");
                    ui.monospace(".on_hover_ui(...)");
                });
                let _ = ui.button("A button you can never press");
            });

        ui.label("Ευρηκα! τ = 2×π")
            .on_hover_text("The current font supports only a few non-latin characters and Egui does not currently support right-to-left text.");

        ui.horizontal(|ui| {
            ui.radio_value(&mut self.radio, Enum::First, "First");
            ui.radio_value(&mut self.radio, Enum::Second, "Second");
            ui.radio_value(&mut self.radio, Enum::Third, "Third");
        });

        combo_box_with_label(ui, "Combo Box", format!("{:?}", self.radio), |ui| {
            ui.radio_value(&mut self.radio, Enum::First, "First");
            ui.radio_value(&mut self.radio, Enum::Second, "Second");
            ui.radio_value(&mut self.radio, Enum::Third, "Third");
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
            ui.add(label!("The button has been clicked {} times", self.count));
        });

        ui.separator();
        {
            ui.horizontal(|ui| {
                ui.label("Drag this value to change it:");
                ui.add(DragValue::f64(&mut self.sliders.value).speed(0.01));
            });

            ui.add(
                Slider::f64(&mut self.sliders.value, 1.0..=100.0)
                    .logarithmic(true)
                    .text("A slider"),
            );

            CollapsingHeader::new("More sliders")
                .default_open(false)
                .show(ui, |ui| {
                    self.sliders.ui(ui);
                });
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
            ui.label("Single line text input:");
            ui.add(
                TextEdit::new(&mut self.single_line_text_input)
                    .multiline(false)
                    .id_source("single line"),
            );
        }); // TODO: .on_hover_text("Enter text to edit me")

        ui.label("Multiline text input:");
        ui.add(TextEdit::new(&mut self.multiline_text_input).id_source("multiline"));

        ui.separator();
        super::toggle_switch::demo(ui, &mut self.toggle_switch);
    }
}
