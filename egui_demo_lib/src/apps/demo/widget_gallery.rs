#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
enum Enum {
    First,
    Second,
    Third,
}

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct WidgetGallery {
    enabled: bool,
    boolean: bool,
    radio: Enum,
    scalar: f32,
    string: String,
    color: egui::Color32,
}

impl Default for WidgetGallery {
    fn default() -> Self {
        Self {
            enabled: true,
            boolean: false,
            radio: Enum::First,
            scalar: 42.0,
            string: Default::default(),
            color: egui::Color32::LIGHT_BLUE.linear_multiply(0.5),
        }
    }
}

impl super::Demo for WidgetGallery {
    fn name(&self) -> &str {
        "🗄 Widget Gallery"
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

impl super::View for WidgetGallery {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            enabled,
            boolean,
            radio,
            scalar,
            string,
            color,
        } = self;

        ui.horizontal(|ui| {
            ui.radio_value(enabled, true, "Enabled");
            ui.radio_value(enabled, false, "Disabled");
        });
        ui.set_enabled(*enabled);

        ui.separator();

        let grid = egui::Grid::new("my_grid")
            .striped(true)
            .spacing([40.0, 4.0]);
        grid.show(ui, |ui| {
            ui.label("Label:");
            ui.label("Welcome to the widget gallery!");
            ui.end_row();

            ui.label("Hyperlink:");
            ui.add(egui::Hyperlink::new("https://github.com/emilk/egui").text(" egui home page"));
            ui.end_row();

            ui.label("TextEdit:");
            ui.add(egui::TextEdit::singleline(string).hint_text("Write something here"));
            ui.end_row();

            ui.label("Checkbox:");
            ui.checkbox(boolean, "Checkbox");
            ui.end_row();

            ui.label("RadioButton:");
            ui.horizontal(|ui| {
                ui.radio_value(radio, Enum::First, "First");
                ui.radio_value(radio, Enum::Second, "Second");
                ui.radio_value(radio, Enum::Third, "Third");
            });
            ui.end_row();

            ui.label("SelectableLabel:");
            ui.horizontal(|ui| {
                ui.selectable_value(radio, Enum::First, "First");
                ui.selectable_value(radio, Enum::Second, "Second");
                ui.selectable_value(radio, Enum::Third, "Third");
            });
            ui.end_row();

            ui.label("ComboBox:");
            egui::combo_box_with_label(ui, "Take your pick", format!("{:?}", radio), |ui| {
                ui.selectable_value(radio, Enum::First, "First");
                ui.selectable_value(radio, Enum::Second, "Second");
                ui.selectable_value(radio, Enum::Third, "Third");
            });
            ui.end_row();

            ui.label("Slider:");
            ui.add(egui::Slider::f32(scalar, 0.0..=100.0).text("value"));
            ui.end_row();

            ui.label("DragValue:");
            ui.add(egui::DragValue::f32(scalar).speed(1.0));
            ui.end_row();

            ui.label("Color picker:");
            ui.color_edit_button_srgba(color);
            ui.end_row();

            ui.label("Image:");
            ui.image(egui::TextureId::Egui, [24.0, 16.0])
                .on_hover_text("The font texture");
            ui.end_row();

            ui.label("Button:");
            if ui.button("Toggle boolean").clicked() {
                *boolean = !*boolean;
            }
            ui.end_row();

            ui.label("ImageButton:");
            if ui
                .add(egui::ImageButton::new(egui::TextureId::Egui, [24.0, 16.0]))
                .clicked()
            {
                *boolean = !*boolean;
            }
            ui.end_row();

            ui.label("Separator:");
            ui.separator();
            ui.end_row();

            ui.label("CollapsingHeader:");
            ui.collapsing("Click to see what is hidden!", |ui| {
                ui.horizontal_wrapped_for_text(egui::TextStyle::Body, |ui| {
                    ui.label(
                        "Not much, as it turns out - but here is a gold star for you for checking:",
                    );
                    ui.colored_label(egui::Color32::GOLD, "☆");
                });
            });
            ui.end_row();

            ui.label("Custom widget");
            super::toggle_switch::demo(ui, boolean);
            ui.end_row();
        });

        ui.separator();
        ui.vertical_centered(|ui| {
            ui.add(crate::__egui_github_link_file!());
        });
    }
}
