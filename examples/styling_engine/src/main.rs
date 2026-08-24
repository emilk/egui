#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! A small styling engine: a custom [`StyleProvider`] that styles every [`egui::Button`]
//! based on the _classes_ set on it, and that can be edited live.

use eframe::egui::{
    self, CentralPanel, Color32, Frame, Panel,
    theme::StyleProvider,
    widget_style::{BaseStyle, ButtonStyle, HasClasses as _, StyleArgs, WidgetState},
};

/// Buttons with this class are styled as a destructive action.
const DANGER: &str = "danger";

/// Our styling engine: it decides what every button looks like.
#[derive(Clone, Copy, PartialEq)]
struct MyTheme {
    normal: Color32,
    danger: Color32,
    corner_radius: u8,
}

impl MyTheme {
    fn preset(preset: Preset) -> Self {
        match preset {
            Preset::Ocean => Self {
                normal: Color32::from_rgb(0x1E, 0x5A, 0x8A),
                danger: Color32::from_rgb(0x9B, 0x2C, 0x2C),
                corner_radius: 4,
            },
            Preset::Candy => Self {
                normal: Color32::from_rgb(0xB8, 0x3B, 0x9E),
                danger: Color32::from_rgb(0xD9, 0x6A, 0x1F),
                corner_radius: 16,
            },
        }
    }
}

impl StyleProvider<ButtonStyle> for MyTheme {
    fn style(&mut self, args: &StyleArgs<'_>) -> ButtonStyle {
        let StyleArgs {
            classes,
            state,
            ctx,
            ..
        } = args;

        // Start from the style egui computed for a generic widget, so we inherit e.g. the font:
        let base: BaseStyle = ctx.get_widget_style(args);

        let fill = if classes.has(DANGER) {
            self.danger
        } else {
            self.normal
        };

        // React to what the user is doing with the button:
        let fill = match state {
            WidgetState::Hovered => fill.gamma_multiply(1.4),
            WidgetState::Active => fill.gamma_multiply(0.7),
            _ => fill,
        };

        ButtonStyle {
            frame: Frame::new()
                .fill(fill)
                .corner_radius(self.corner_radius)
                .inner_margin(8),
            text_style: base.text,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Preset {
    Ocean,
    Candy,
}

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 340.0]),
        ..Default::default()
    };

    let mut preset = Preset::Ocean;
    let mut theme = MyTheme::preset(preset);
    let mut last_click = "nothing";

    eframe::run_ui_native("Styling engine", options, move |ui, _frame| {
        // Register our theme for all buttons. This is a no-op after the first frame.
        ui.add_widget_theme::<ButtonStyle>(theme);

        Panel::left("controls").default_size(260.0).show(ui, |ui| {
            // The custom theme inherits the text color from egui's light/dark theme:
            egui::global_theme_preference_buttons(ui);

            ui.separator();

            ui.heading("Button theme");

            let mut changed = false;

            egui::ComboBox::from_label("Preset")
                .selected_text(match preset {
                    Preset::Ocean => "Ocean",
                    Preset::Candy => "Candy",
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut preset, Preset::Ocean, "Ocean")
                        .changed();
                    changed |= ui
                        .selectable_value(&mut preset, Preset::Candy, "Candy")
                        .changed();
                });
            if changed {
                theme = MyTheme::preset(preset);
            }

            ui.horizontal(|ui| {
                changed |= ui.color_edit_button_srgba(&mut theme.normal).changed();
                ui.label("Normal");
            });
            ui.horizontal(|ui| {
                changed |= ui.color_edit_button_srgba(&mut theme.danger).changed();
                ui.label("Danger");
            });
            changed |= ui
                .add(egui::Slider::new(&mut theme.corner_radius, 0..=24).text("Corner radius"))
                .changed();

            if changed {
                // Overwrite the registered theme with the edited one:
                ui.replace_widget_theme::<ButtonStyle>(theme);
            }
        });

        CentralPanel::default().show(ui, |ui| {
            ui.heading("Buttons");
            ui.label("All buttons below are styled by the custom theme.");
            ui.add_space(8.0);

            if ui.button("Save").clicked() {
                last_click = "Save";
            }
            ui.add_space(4.0);
            if ui
                .add(egui::Button::new("Delete everything").with_class(DANGER))
                .clicked()
            {
                last_click = "Delete everything";
            }

            ui.add_space(8.0);
            ui.label(format!("Last clicked: {last_click}"));
        });
    })
}
