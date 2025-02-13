use egui::{
    Align, Align2, Align4, ComboBox, Direction, Frame, Id, Label, Layout, Popup,
    PopupCloseBehavior, Rect, RichText, Sense, Tooltip, Ui, UiBuilder, Vec2, Widget,
};

#[derive(Clone, PartialEq, Debug)]
enum Position {
    Default,
    Pointer,
    PointerFixed,
}

/// Showcase [`Popup`].
#[derive(Clone, PartialEq)]
// #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
// #[cfg_attr(feature = "serde", serde(default))]
pub struct PopupsDemo {
    align4: Align4,
    gap: f32,
    close_behavior: PopupCloseBehavior,
    popup_open: bool,
}

impl PopupsDemo {
    fn apply_options<'a>(&self, popup: Popup<'a>) -> Popup<'a> {
        popup
            .position(self.align4)
            .gap(self.gap)
            .close_behavior(self.close_behavior)
    }
}

impl Default for PopupsDemo {
    fn default() -> Self {
        Self {
            align4: Align4::default(),
            gap: 4.0,
            close_behavior: PopupCloseBehavior::CloseOnClick,
            popup_open: false,
        }
    }
}

impl crate::Demo for PopupsDemo {
    fn name(&self) -> &'static str {
        "\u{20E3} Popups"
    }

    fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .default_width(250.0)
            .constrain(false)
            .show(ctx, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for PopupsDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = 0.0;
            let align_combobox = |ui: &mut Ui, label: &str, align: &mut Align2| {
                let aligns = [
                    (Align2::LEFT_TOP, "Left top"),
                    (Align2::LEFT_CENTER, "Left center"),
                    (Align2::LEFT_BOTTOM, "Left bottom"),
                    (Align2::CENTER_TOP, "Center top"),
                    (Align2::CENTER_CENTER, "Center center"),
                    (Align2::CENTER_BOTTOM, "Center bottom"),
                    (Align2::RIGHT_TOP, "Right top"),
                    (Align2::RIGHT_CENTER, "Right center"),
                    (Align2::RIGHT_BOTTOM, "Right bottom"),
                ];

                ui.label(label);
                ComboBox::new(label, "")
                    .selected_text(aligns.iter().find(|(a, _)| a == align).unwrap().1)
                    .show_ui(ui, |ui| {
                        for (align2, name) in &aligns {
                            ui.selectable_value(align, *align2, *name);
                        }
                    });
            };

            ui.label("Align4(");
            align_combobox(ui, "align: ", &mut self.align4.align);
            ui.label(", ");
            align_combobox(ui, "focus: ", &mut self.align4.focus);
            ui.label(") ");

            let presets = [
                (Align4::TOP_START, "Top start"),
                (Align4::TOP, "Top"),
                (Align4::TOP_END, "Top end"),
                (Align4::RIGHT_START, "Right start"),
                (Align4::RIGHT, "Right Center"),
                (Align4::RIGHT_END, "Right end"),
                (Align4::BOTTOM_START, "Bottom start"),
                (Align4::BOTTOM, "Bottom"),
                (Align4::BOTTOM_END, "Bottom end"),
                (Align4::LEFT_START, "Left start"),
                (Align4::LEFT, "Left"),
                (Align4::LEFT_END, "Left end"),
            ];

            ui.label(" Presets: ");

            ComboBox::new("Preset", "")
                .selected_text(
                    presets
                        .iter()
                        .find(|(a, _)| a == &self.align4)
                        .map_or("Select", |(_, name)| *name),
                )
                .show_ui(ui, |ui| {
                    for (align4, name) in &presets {
                        ui.selectable_value(&mut self.align4, *align4, *name);
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Gap:");
            ui.add(egui::DragValue::new(&mut self.gap));
        });
        ui.horizontal(|ui| {
            ui.label("Close behavior:");
            ui.selectable_value(
                &mut self.close_behavior,
                PopupCloseBehavior::CloseOnClick,
                "Close on click",
            )
            .on_hover_text("Closes when the user clicks anywhere (inside or outside)");
            ui.selectable_value(
                &mut self.close_behavior,
                PopupCloseBehavior::CloseOnClickOutside,
                "Close on click outside",
            )
            .on_hover_text("Closes when the user clicks outside the popup");
            ui.selectable_value(
                &mut self.close_behavior,
                PopupCloseBehavior::IgnoreClicks,
                "Ignore clicks",
            )
            .on_hover_text("Close only when the button is clicked again");
        });

        ui.checkbox(&mut self.popup_open, "Show popup");

        let response = Frame::group(ui.style())
            .inner_margin(50.0)
            .show(ui, |ui| ui.button("Click, right-click and hover me!"))
            .inner;

        self.apply_options(Popup::menu(&response).id(Id::new("menu")))
            .show(ui.ctx(), |ui| {
                ui.button("Menu item 1");
                ui.button("Menu item 2");
            });

        self.apply_options(Popup::context_menu(&response).id(Id::new("context_menu")))
            .show(ui.ctx(), |ui| {
                ui.button("Context menu item 1");
                ui.button("Context menu item 2");
            });

        if self.popup_open {
            self.apply_options(Popup::from_response(&response).id(Id::new("popup")))
                .show(ui.ctx(), |ui| {
                    ui.label("Popup contents");
                });
        }

        let mut tooltip = Tooltip::for_enabled(&response);
        tooltip.popup = self.apply_options(tooltip.popup);
        tooltip.show(ui.ctx(), |ui| {
            ui.label("Tooltips are popups too!");
        });

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}
