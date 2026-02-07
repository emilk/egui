use crate::rust_view_ui;
use egui::color_picker::{Alpha, color_picker_color32};
use egui::containers::menu::{MenuConfig, SubMenuButton};
use egui::{
    Align, Align2, Atom, Button, ComboBox, Frame, Id, Layout, Popup, PopupCloseBehavior, RectAlign,
    RichText, Tooltip, Ui, UiBuilder, include_image,
};

/// Showcase [`Popup`].
#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct PopupsDemo {
    align4: RectAlign,
    gap: f32,
    #[cfg_attr(feature = "serde", serde(skip))]
    close_behavior: PopupCloseBehavior,
    popup_open: bool,
    checked: bool,
    color: egui::Color32,
}

impl Default for PopupsDemo {
    fn default() -> Self {
        Self {
            align4: RectAlign::default(),
            gap: 4.0,
            close_behavior: PopupCloseBehavior::CloseOnClick,
            popup_open: false,
            checked: true,
            color: egui::Color32::RED,
        }
    }
}

impl PopupsDemo {
    fn apply_options<'a>(&self, popup: Popup<'a>) -> Popup<'a> {
        popup
            .align(self.align4)
            .gap(self.gap)
            .close_behavior(self.close_behavior)
    }

    fn nested_menus(&mut self, ui: &mut Ui) {
        ui.set_max_width(200.0); // To make sure we wrap long text

        if ui.button("Open…").clicked() {
            ui.close();
        }
        ui.menu_button("Popups can have submenus", |ui| {
            ui.menu_button("SubMenu", |ui| {
                if ui.button("Open…").clicked() {
                    ui.close();
                }
                let _ = ui.button("Item");
                ui.menu_button("Recursive", |ui| self.nested_menus(ui));
            });
            ui.menu_button("SubMenu", |ui| {
                if ui.button("Open…").clicked() {
                    ui.close();
                }
                let _ = ui.button("Item");
            });
            let _ = ui.button("Item");
            if ui.button("Open…").clicked() {
                ui.close();
            }
        });
        ui.add_enabled_ui(false, |ui| {
            ui.menu_button("SubMenus can be disabled", |_| {});
        });
        ui.menu_image_text_button(
            include_image!("../../data/icon.png"),
            "I have an icon!",
            |ui| {
                let _ = ui.button("Item1");
                let _ = ui.button("Item2");
                let _ = ui.button("Item3");
                let _ = ui.button("Item4");
                if ui.button("Open…").clicked() {
                    ui.close();
                }
            },
        );
        let _ = ui.button("Very long text for this item that should be wrapped");
        SubMenuButton::new("Always CloseOnClickOutside")
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                ui.checkbox(&mut self.checked, "Checkbox");

                // Customized color SubMenuButton
                let is_bright = self.color.intensity() > 0.5;
                let text_color = if is_bright {
                    egui::Color32::BLACK
                } else {
                    egui::Color32::WHITE
                };

                let button = Button::new((
                    RichText::new("Background").color(text_color),
                    Atom::grow(),
                    RichText::new(SubMenuButton::RIGHT_ARROW).color(text_color),
                ))
                .fill(self.color);

                SubMenuButton::from_button(button).ui(ui, |ui| {
                    ui.spacing_mut().slider_width = 200.0;
                    color_picker_color32(ui, &mut self.color, Alpha::Opaque);
                });

                if self.checked {
                    ui.menu_button("Only visible when checked", |ui| {
                        if ui.button("Remove myself").clicked() {
                            self.checked = false;
                        }
                    });
                }

                if ui.button("Open…").clicked() {
                    ui.close();
                }
            });
    }
}

impl crate::Demo for PopupsDemo {
    fn name(&self) -> &'static str {
        "\u{2755} Popups"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .open(open)
            .resizable(false)
            .default_width(250.0)
            .constrain(false)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for PopupsDemo {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let response = Frame::group(ui.style())
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| ui.button("Click, right-click and hover me!"))
                    .inner
            })
            .inner;

        self.apply_options(Popup::menu(&response).id(Id::new("menu")))
            .show(|ui| self.nested_menus(ui));

        self.apply_options(Popup::context_menu(&response).id(Id::new("context_menu")))
            .show(|ui| self.nested_menus(ui));

        if self.popup_open {
            self.apply_options(Popup::from_response(&response).id(Id::new("popup")))
                .show(|ui| {
                    ui.label("Popup contents");
                });
        }

        let mut tooltip = Tooltip::for_enabled(&response);
        tooltip.popup = self.apply_options(tooltip.popup);
        tooltip.show(|ui| {
            ui.label("Tooltips are popups, too!");
        });

        Frame::canvas(ui.style()).show(ui, |ui| {
            let mut reset_btn_ui = ui.new_child(
                UiBuilder::new()
                    .max_rect(ui.max_rect())
                    .layout(Layout::right_to_left(Align::Min)),
            );
            if reset_btn_ui
                .button("⟲")
                .on_hover_text("Reset to defaults")
                .clicked()
            {
                *self = Self::default();
            }

            ui.set_width(ui.available_width());
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
            ui.spacing_mut().item_spacing.x = 0.0;
            let align_combobox = |ui: &mut Ui, label: &str, align: &mut Align2| {
                let aligns = [
                    (Align2::LEFT_TOP, "LEFT_TOP"),
                    (Align2::LEFT_CENTER, "LEFT_CENTER"),
                    (Align2::LEFT_BOTTOM, "LEFT_BOTTOM"),
                    (Align2::CENTER_TOP, "CENTER_TOP"),
                    (Align2::CENTER_CENTER, "CENTER_CENTER"),
                    (Align2::CENTER_BOTTOM, "CENTER_BOTTOM"),
                    (Align2::RIGHT_TOP, "RIGHT_TOP"),
                    (Align2::RIGHT_CENTER, "RIGHT_CENTER"),
                    (Align2::RIGHT_BOTTOM, "RIGHT_BOTTOM"),
                ];

                ComboBox::new(label, "")
                    .selected_text(aligns.iter().find(|(a, _)| a == align).unwrap().1)
                    .show_ui(ui, |ui| {
                        for (align2, name) in &aligns {
                            ui.selectable_value(align, *align2, *name);
                        }
                    });
            };

            rust_view_ui(ui, "let align = RectAlign {");
            ui.horizontal(|ui| {
                rust_view_ui(ui, "    parent: Align2::");
                align_combobox(ui, "parent", &mut self.align4.parent);
                rust_view_ui(ui, ",");
            });
            ui.horizontal(|ui| {
                rust_view_ui(ui, "    child: Align2::");
                align_combobox(ui, "child", &mut self.align4.child);
                rust_view_ui(ui, ",");
            });
            rust_view_ui(ui, "};");

            ui.horizontal(|ui| {
                rust_view_ui(ui, "let align = RectAlign::");

                let presets = [
                    (RectAlign::TOP_START, "TOP_START"),
                    (RectAlign::TOP, "TOP"),
                    (RectAlign::TOP_END, "TOP_END"),
                    (RectAlign::RIGHT_START, "RIGHT_START"),
                    (RectAlign::RIGHT, "RIGHT"),
                    (RectAlign::RIGHT_END, "RIGHT_END"),
                    (RectAlign::BOTTOM_START, "BOTTOM_START"),
                    (RectAlign::BOTTOM, "BOTTOM"),
                    (RectAlign::BOTTOM_END, "BOTTOM_END"),
                    (RectAlign::LEFT_START, "LEFT_START"),
                    (RectAlign::LEFT, "LEFT"),
                    (RectAlign::LEFT_END, "LEFT_END"),
                ];

                ComboBox::new("Preset", "")
                    .selected_text(
                        presets
                            .iter()
                            .find(|(a, _)| a == &self.align4)
                            .map_or("<Select Preset>", |(_, name)| *name),
                    )
                    .show_ui(ui, |ui| {
                        for (align4, name) in &presets {
                            ui.selectable_value(&mut self.align4, *align4, *name);
                        }
                    });
                rust_view_ui(ui, ";");
            });

            ui.horizontal(|ui| {
                rust_view_ui(ui, "let gap = ");
                ui.add(egui::DragValue::new(&mut self.gap));
                rust_view_ui(ui, ";");
            });

            rust_view_ui(ui, "let close_behavior");
            ui.horizontal(|ui| {
                rust_view_ui(ui, "    = PopupCloseBehavior::");
                let close_behaviors = [
                    (
                        PopupCloseBehavior::CloseOnClick,
                        "CloseOnClick",
                        "Closes when the user clicks anywhere (inside or outside)",
                    ),
                    (
                        PopupCloseBehavior::CloseOnClickOutside,
                        "CloseOnClickOutside",
                        "Closes when the user clicks outside the popup",
                    ),
                    (
                        PopupCloseBehavior::IgnoreClicks,
                        "IgnoreClicks",
                        "Close only when the button is clicked again",
                    ),
                ];
                ComboBox::new("Close behavior", "")
                    .selected_text(
                        close_behaviors
                            .iter()
                            .find_map(|(behavior, text, _)| {
                                (behavior == &self.close_behavior).then_some(*text)
                            })
                            .unwrap(),
                    )
                    .show_ui(ui, |ui| {
                        for (close_behavior, name, tooltip) in &close_behaviors {
                            ui.selectable_value(&mut self.close_behavior, *close_behavior, *name)
                                .on_hover_text(*tooltip);
                        }
                    });
                rust_view_ui(ui, ";");
            });

            ui.horizontal(|ui| {
                rust_view_ui(ui, "let popup_open = ");
                ui.checkbox(&mut self.popup_open, "");
                rust_view_ui(ui, ";");
            });
            ui.monospace("");
            rust_view_ui(ui, "let response = ui.button(\"Click me!\");");
            rust_view_ui(ui, "Popup::menu(&response)");
            rust_view_ui(ui, "    .gap(gap).align(align)");
            rust_view_ui(ui, "    .close_behavior(close_behavior)");
            rust_view_ui(ui, "    .show(|ui| { /* menu contents */ });");
        });

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}
