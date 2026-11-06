#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum Enum {
    First,
    Second,
    Third,
}

/// Shows off one example of each major type of widget.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WidgetGallery {
    enabled: bool,
    visible: bool,
    boolean: bool,
    opacity: f32,
    radio: Enum,
    scalar: f32,
    range: egui::Rangef,
    string: String,
    color: egui::Color32,
    animate_progress_bar: bool,

    #[cfg(feature = "jiff")]
    #[cfg_attr(feature = "serde", serde(skip))]
    date: Option<jiff::civil::Date>,

    #[cfg(feature = "jiff")]
    with_date_button: bool,
}

impl Default for WidgetGallery {
    fn default() -> Self {
        Self {
            enabled: true,
            visible: true,
            opacity: 1.0,
            boolean: false,
            radio: Enum::First,
            scalar: 42.0,
            range: egui::Rangef::new(90.0, 250.0),
            string: Default::default(),
            color: egui::Color32::LIGHT_BLUE.linear_multiply(0.5),
            animate_progress_bar: false,
            #[cfg(feature = "jiff")]
            date: None,
            #[cfg(feature = "jiff")]
            with_date_button: true,
        }
    }
}

impl WidgetGallery {
    #[allow(clippy::allow_attributes, unused_mut)] // if not jiff
    #[inline]
    pub fn with_date_button(mut self, _with_date_button: bool) -> Self {
        #[cfg(feature = "jiff")]
        {
            self.with_date_button = _with_date_button;
        }
        self
    }
}

impl crate::Demo for WidgetGallery {
    fn name(&self) -> &'static str {
        "🗄️ Widget Gallery"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(crate::i18n::t(self.name()))
            .open(open)
            .resizable([true, false]) // resizable so we can shrink if the text edit grows
            .default_width(280.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for WidgetGallery {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut ui_builder = egui::UiBuilder::new();
        if !self.enabled {
            ui_builder = ui_builder.disabled();
        }
        if !self.visible {
            ui_builder = ui_builder.invisible();
        }

        ui.scope_builder(ui_builder, |ui| {
            ui.multiply_opacity(self.opacity);

            egui::Grid::new("my_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .striped(true)
                .show(ui, |ui| {
                    self.gallery_grid_contents(ui);
                });
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.visible, crate::i18n::t("Visible"))
                .on_hover_text(crate::i18n::t("Uncheck to hide all the widgets."));
            if self.visible {
                ui.checkbox(&mut self.enabled, crate::i18n::t("Interactive"))
                    .on_hover_text(crate::i18n::t(
                        "Uncheck to inspect how the widgets look when disabled.",
                    ));
                let drag_value = ui.add(
                    egui::DragValue::new(&mut self.opacity)
                        .speed(0.01)
                        .range(0.0..=1.0),
                );
                let label = ui.label(crate::i18n::t("Opacity"));
                (drag_value.labelled_by(label.id) | label).on_hover_text(crate::i18n::t(
                    "Reduce this value to make widgets semi-transparent",
                ));
            }
        });

        ui.separator();

        ui.vertical_centered(|ui| {
            let tooltip_text = crate::i18n::t(
                "The full egui documentation.\nYou can also click the different widgets names in the left column.",
            );
            ui.hyperlink("https://docs.rs/egui/").on_hover_text(tooltip_text);
            ui.add(crate::egui_github_link_file!(crate::i18n::t(
                "Source code of the widget gallery"
            )));
        });
    }
}

impl WidgetGallery {
    fn gallery_grid_contents(&mut self, ui: &mut egui::Ui) {
        let Self {
            enabled: _,
            visible: _,
            opacity: _,
            boolean,
            radio,
            scalar,
            range,
            string,
            color,
            animate_progress_bar,
            #[cfg(feature = "jiff")]
            date,
            #[cfg(feature = "jiff")]
            with_date_button,
        } = self;

        ui.add(doc_link_label(crate::i18n::t("Label"), "label"));
        ui.label(crate::i18n::t("Welcome to the widget gallery!"));
        ui.end_row();

        ui.add(doc_link_label(crate::i18n::t("Hyperlink"), "Hyperlink"));
        use egui::special_emojis::GITHUB;
        ui.hyperlink_to(
            crate::i18n::t(&format!("{GITHUB} egui on GitHub")),
            "https://github.com/emilk/egui",
        );
        ui.end_row();

        ui.add(doc_link_label(crate::i18n::t("TextEdit"), "TextEdit"));
        ui.add(
            egui::TextEdit::singleline(string).hint_text(crate::i18n::t("Write something here")),
        );
        ui.end_row();

        ui.add(doc_link_label(crate::i18n::t("Button"), "button"));
        if ui.button(crate::i18n::t("Click me!")).clicked() {
            *boolean = !*boolean;
        }
        ui.end_row();

        ui.add(doc_link_label(crate::i18n::t("Link"), "link"));
        if ui.link(crate::i18n::t("Click me!")).clicked() {
            *boolean = !*boolean;
        }
        ui.end_row();

        ui.add(doc_link_label(crate::i18n::t("Checkbox"), "checkbox"));
        ui.checkbox(boolean, crate::i18n::t("Checkbox"));
        ui.end_row();

        ui.add(doc_link_label(crate::i18n::t("RadioButton"), "radio"));
        ui.horizontal(|ui| {
            ui.radio_value(radio, Enum::First, crate::i18n::t("First"));
            ui.radio_value(radio, Enum::Second, crate::i18n::t("Second"));
            ui.radio_value(radio, Enum::Third, crate::i18n::t("Third"));
        });
        ui.end_row();

        ui.add(doc_link_label(
            crate::i18n::t("SelectableLabel"),
            "SelectableLabel",
        ));
        ui.horizontal(|ui| {
            ui.selectable_value(radio, Enum::First, crate::i18n::t("First"));
            ui.selectable_value(radio, Enum::Second, crate::i18n::t("Second"));
            ui.selectable_value(radio, Enum::Third, crate::i18n::t("Third"));
        });
        ui.end_row();

        ui.add(doc_link_label(crate::i18n::t("ComboBox"), "ComboBox"));

        egui::ComboBox::from_label(crate::i18n::t("Take your pick"))
            .selected_text(format!("{radio:?}"))
            .show_ui(ui, |ui| {
                ui.selectable_value(radio, Enum::First, crate::i18n::t("First"));
                ui.selectable_value(radio, Enum::Second, crate::i18n::t("Second"));
                ui.selectable_value(radio, Enum::Third, crate::i18n::t("Third"));
            });
        ui.end_row();

        let label = ui.add(doc_link_label(crate::i18n::t("Slider"), "Slider"));
        ui.add(egui::Slider::new(scalar, 0.0..=360.0).suffix("°"))
            .labelled_by(label.id);
        ui.end_row();

        let label = ui.add(doc_link_label(crate::i18n::t("RangeSlider"), "RangeSlider"));
        ui.add(egui::RangeSlider::new(&mut range.min, &mut range.max, 0.0..=360.0).suffix("°"))
            .labelled_by(label.id);
        ui.end_row();

        let label = ui.add(doc_link_label(crate::i18n::t("DragValue"), "DragValue"));
        ui.add(egui::DragValue::new(scalar).speed(1.0))
            .labelled_by(label.id);
        ui.end_row();

        ui.add(doc_link_label(crate::i18n::t("ProgressBar"), "ProgressBar"));
        let progress = *scalar / 360.0;
        let progress_bar = egui::ProgressBar::new(progress)
            .show_percentage()
            .animate(*animate_progress_bar);
        *animate_progress_bar = ui
            .add(progress_bar)
            .on_hover_text(crate::i18n::t("The progress bar can be animated!"))
            .hovered();
        ui.end_row();

        ui.add(doc_link_label(crate::i18n::t("Color picker"), "color_edit"));
        ui.color_edit_button_srgba(color);
        ui.end_row();

        ui.add(doc_link_label(crate::i18n::t("Image"), "Image"));
        let egui_icon = egui::include_image!("../../data/icon.svg");
        ui.add(egui::Image::new(egui_icon.clone()));
        ui.end_row();

        ui.add(doc_link_label(
            crate::i18n::t("Button with image"),
            "Button::image_and_text",
        ));
        if ui
            .button((egui_icon, crate::i18n::t("Click me!")))
            .clicked()
        {
            *boolean = !*boolean;
        }
        ui.end_row();

        #[cfg(feature = "jiff")]
        if *with_date_button {
            let date = date.get_or_insert_with(|| jiff::Zoned::now().date());
            ui.add(doc_link_label_with_crate(
                "egui_extras",
                crate::i18n::t("DatePickerButton"),
                "DatePickerButton",
            ));
            ui.add(egui_extras::DatePickerButton::new(date));
            ui.end_row();
        }

        ui.add(doc_link_label(crate::i18n::t("Separator"), "separator"));
        ui.separator();
        ui.end_row();

        ui.add(doc_link_label(
            crate::i18n::t("CollapsingHeader"),
            "collapsing",
        ));
        ui.collapsing(crate::i18n::t("Click to see what is hidden!"), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label(crate::i18n::t("It's a "));
                ui.add(doc_link_label(crate::i18n::t("Spinner"), "spinner"));
                ui.add_space(4.0);
                ui.add(egui::Spinner::new());
            });
        });
        ui.end_row();

        ui.hyperlink_to(
            crate::i18n::t("Custom widget"),
            super::toggle_switch::url_to_file_source_code(),
        );
        ui.add(super::toggle_switch::toggle(boolean))
            .on_hover_text(crate::i18n::t(
                "It's easy to create your own widgets!\n\
            This toggle switch is just 15 lines of code.",
            ));
        ui.end_row();
    }
}

fn doc_link_label<'a>(title: &'a str, search_term: &'a str) -> impl egui::Widget + 'a {
    doc_link_label_with_crate("egui", title, search_term)
}

fn doc_link_label_with_crate<'a>(
    crate_name: &'a str,
    title: &'a str,
    search_term: &'a str,
) -> impl egui::Widget + 'a {
    let url = format!("https://docs.rs/{crate_name}?search={search_term}");
    move |ui: &mut egui::Ui| {
        ui.hyperlink_to(title, url).on_hover_ui(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(crate::i18n::t("Search egui docs for"));
                ui.code(search_term);
            });
        })
    }
}

#[cfg(feature = "jiff")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::View as _;
    use egui::Vec2;
    use egui_kittest::{Harness, SnapshotResults};

    #[test]
    pub fn should_match_screenshot() {
        let mut demo = WidgetGallery {
            // If we don't set a fixed date, the snapshot test will fail.
            date: Some(jiff::civil::date(2024, 1, 1)),
            ..Default::default()
        };

        let mut results = SnapshotResults::new();

        for pixels_per_point in [1, 2] {
            for theme in [egui::Theme::Light, egui::Theme::Dark] {
                let mut harness = Harness::builder()
                    .with_pixels_per_point(pixels_per_point as f32)
                    .with_theme(theme)
                    .with_size(Vec2::new(380.0, 550.0))
                    .build_ui(|ui| {
                        egui_extras::install_image_loaders(ui.ctx());
                        demo.ui(ui);
                    });

                harness.fit_contents();

                let theme_name = match theme {
                    egui::Theme::Light => "light",
                    egui::Theme::Dark => "dark",
                };
                let image_name = format!("widget_gallery_{theme_name}_x{pixels_per_point}");
                harness.snapshot(&image_name);
                results.extend_harness(&mut harness);
            }
        }
    }
}
