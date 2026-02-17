struct HistoryEntry {
    text: String,
    repeated: usize,
}

#[derive(Default)]
struct DeduplicatedHistory {
    history: std::collections::VecDeque<HistoryEntry>,
}

impl DeduplicatedHistory {
    fn add(&mut self, text: String) {
        if let Some(entry) = self.history.back_mut()
            && entry.text == text
        {
            entry.repeated += 1;
            return;
        }
        self.history.push_back(HistoryEntry { text, repeated: 1 });
        if self.history.len() > 100 {
            self.history.pop_front();
        }
    }

    fn ui(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for HistoryEntry { text, repeated } in self.history.iter().rev() {
                    ui.horizontal(|ui| {
                        if text.is_empty() {
                            ui.weak("(empty)");
                        } else {
                            ui.label(text);
                        }
                        if 1 < *repeated {
                            ui.weak(format!(" x{repeated}"));
                        }
                    });
                }
            });
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default)]
pub struct InputTest {
    #[cfg_attr(feature = "serde", serde(skip))]
    history: [DeduplicatedHistory; 4],

    late_interaction: bool,

    show_hovers: bool,
}

impl crate::Demo for InputTest {
    fn name(&self) -> &'static str {
        "Input Test"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .default_width(800.0)
            .open(open)
            .resizable(true)
            .scroll(false)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for InputTest {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 8.0;

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.horizontal(|ui| {
            if ui.button("Clear").clicked() {
                *self = Default::default();
            }

            ui.checkbox(&mut self.show_hovers, "Show hover state");
        });

        ui.checkbox(&mut self.late_interaction, "Use Response::interact");

        ui.label("This tests how egui::Response reports events.\n\
            The different buttons are sensitive to different things.\n\
            Try interacting with them with any mouse button by clicking, double-clicking, triple-clicking, or dragging them.");

        ui.columns(4, |columns| {
            for (i, (sense_name, sense)) in [
                ("Sense::hover", egui::Sense::hover()),
                ("Sense::click", egui::Sense::click()),
                ("Sense::drag", egui::Sense::drag()),
                ("Sense::click_and_drag", egui::Sense::click_and_drag()),
            ]
            .into_iter()
            .enumerate()
            {
                columns[i].push_id(i, |ui| {
                    let response = if self.late_interaction {
                        let first_response =
                            ui.add(egui::Button::new(sense_name).sense(egui::Sense::hover()));
                        first_response.interact(sense)
                    } else {
                        ui.add(egui::Button::new(sense_name).sense(sense))
                    };
                    let info = response_summary(&response, self.show_hovers);
                    self.history[i].add(info.trim().to_owned());
                    self.history[i].ui(ui);
                });
            }
        });
    }
}

fn response_summary(response: &egui::Response, show_hovers: bool) -> String {
    use std::fmt::Write as _;

    let mut new_info = String::new();

    if show_hovers {
        if response.hovered() {
            writeln!(new_info, "hovered").ok();
        }
        if response.contains_pointer() {
            writeln!(new_info, "contains_pointer").ok();
        }
        if response.is_pointer_button_down_on() {
            writeln!(new_info, "pointer_down_on").ok();
        }
        if let Some(pos) = response.interact_pointer_pos() {
            writeln!(new_info, "response.interact_pointer_pos: {pos:?}").ok();
        }
    }

    for &button in &[
        egui::PointerButton::Primary,
        egui::PointerButton::Secondary,
        egui::PointerButton::Middle,
        egui::PointerButton::Extra1,
        egui::PointerButton::Extra2,
    ] {
        let button_suffix = if button == egui::PointerButton::Primary {
            // Reduce visual clutter in common case:
            String::default()
        } else {
            format!(" by {button:?} button")
        };

        // These are in inverse logical/chonological order, because we show them in the ui that way:

        if response.triple_clicked_by(button) {
            writeln!(new_info, "Triple-clicked{button_suffix}").ok();
        }
        if response.double_clicked_by(button) {
            writeln!(new_info, "Double-clicked{button_suffix}").ok();
        }
        if response.clicked_by(button) {
            writeln!(new_info, "Clicked{button_suffix}").ok();
        }

        if response.drag_stopped_by(button) {
            writeln!(new_info, "Drag stopped{button_suffix}").ok();
        }
        if response.dragged_by(button) {
            writeln!(new_info, "Dragged{button_suffix}").ok();
        }
        if response.drag_started_by(button) {
            writeln!(new_info, "Drag started{button_suffix}").ok();
        }
    }

    if response.long_touched() {
        writeln!(new_info, "Clicked with long-press").ok();
    }

    new_info
}
