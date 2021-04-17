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
    group_enabled: bool,
    count: usize,
    radio: Enum,
    angle: f32,
    color: Color32,
    single_line_text_input: String,
    multiline_text_input: String,
}

impl Default for Widgets {
    fn default() -> Self {
        Self {
            group_enabled: true,
            radio: Enum::First,
            count: 0,
            angle: std::f32::consts::TAU / 3.0,
            color: (Rgba::from_rgb(0.0, 1.0, 0.5) * 0.75).into(),
            single_line_text_input: "Hello World!".to_owned(),
            multiline_text_input: "Text can both be so wide that it needs a line break, but you can also add manual line break by pressing enter, creating new paragraphs.\nThis is the start of the next paragraph.\n\nClick me to edit me!".to_owned(),
        }
    }
}

impl Widgets {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add(crate::__egui_github_link_file_line!());
        });

        ui.horizontal_wrapped(|ui| {
            // Trick so we don't have to add spaces in the text below:
            ui.spacing_mut().item_spacing.x = ui.fonts()[TextStyle::Body].glyph_width(' ');

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

        ui.group(|ui| {
            ui.checkbox(&mut self.group_enabled, "Group enabled");
            ui.set_enabled(self.group_enabled);
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.radio, Enum::First, "First");
                ui.radio_value(&mut self.radio, Enum::Second, "Second");
                ui.radio_value(&mut self.radio, Enum::Third, "Third");
            });

            egui::ComboBox::from_label("Combo Box")
                .selected_text(format!("{:?}", self.radio))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.radio, Enum::First, "First");
                    ui.selectable_value(&mut self.radio, Enum::Second, "Second");
                    ui.selectable_value(&mut self.radio, Enum::Third, "Third");
                });
        });

        ui.horizontal(|ui| {
            if ui
                .button("Click me")
                .on_hover_text("This will just increase a counter.")
                .clicked()
            {
                self.count += 1;
            }
            ui.label(format!("The button has been clicked {} times.", self.count));
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("An angle:");
            ui.drag_angle(&mut self.angle);
            ui.label(format!("≈ {:.3}τ", self.angle / std::f32::consts::TAU))
                .on_hover_text("Each τ represents one turn (τ = 2π)");
        })
        .response
        .on_hover_text("The angle is stored in radians, but presented in degrees");

        ui.separator();

        ui.horizontal(|ui| {
            ui.colored_label(self.color, "Click to select a different text color: ");
            ui.color_edit_button_srgba(&mut self.color);
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Password:");
            // We let `egui` store the show/hide password toggle:
            let show_password_id = Id::new("show_password");
            let mut show_password: bool = *ui.memory().id_data.get_or_default(show_password_id);
            let response = ui.add_sized(
                [140.0, 20.0],
                egui::TextEdit::singleline(&mut self.single_line_text_input)
                    .password(!show_password),
            );
            if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                // …
            }
            ui.checkbox(&mut show_password, "Show password");
            ui.memory().id_data.insert(show_password_id, show_password);
        });

        ui.label("Multiline text input:");
        ui.text_edit_multiline(&mut self.multiline_text_input);
    }
}
