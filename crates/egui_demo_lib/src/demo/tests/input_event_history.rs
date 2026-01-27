//! Show the history of all the input events to

struct HistoryEntry {
    summary: String,
    entries: Vec<String>,
}

#[derive(Default)]
struct DeduplicatedHistory {
    history: std::collections::VecDeque<HistoryEntry>,
}

impl DeduplicatedHistory {
    fn add(&mut self, summary: String, full: String) {
        if let Some(entry) = self.history.back_mut()
            && entry.summary == summary
        {
            entry.entries.push(full);
            return;
        }
        self.history.push_back(HistoryEntry {
            summary,
            entries: vec![full],
        });
        if self.history.len() > 100 {
            self.history.pop_front();
        }
    }

    fn ui(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

                for HistoryEntry { summary, entries } in self.history.iter().rev() {
                    ui.horizontal(|ui| {
                        let response = ui.code(summary);
                        if entries.len() < 2 {
                            response
                        } else {
                            response | ui.weak(format!(" x{}", entries.len()))
                        }
                    })
                    .inner
                    .on_hover_ui(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        for entry in entries.iter().rev() {
                            ui.code(entry);
                        }
                    });
                }
            });
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Default)]
pub struct InputEventHistory {
    #[cfg_attr(feature = "serde", serde(skip))]
    history: DeduplicatedHistory,

    include_pointer_movements: bool,
}

impl crate::Demo for InputEventHistory {
    fn name(&self) -> &'static str {
        "Input Event History"
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

impl crate::View for InputEventHistory {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.input(|i| {
            for event in &i.raw.events {
                if !self.include_pointer_movements
                    && matches!(
                        event,
                        egui::Event::PointerMoved { .. }
                            | egui::Event::MouseMoved { .. }
                            | egui::Event::Touch { .. }
                    )
                {
                    continue;
                }

                let summary = event_summary(event);
                let full = format!("{event:#?}");
                self.history.add(summary, full);
            }
        });

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });

        ui.label("Recent history of raw input events to egui.");
        ui.label("Hover any entry for details.");
        ui.checkbox(
            &mut self.include_pointer_movements,
            "Include pointer/mouse movements",
        );

        ui.add_space(8.0);

        self.history.ui(ui);
    }
}

fn event_summary(event: &egui::Event) -> String {
    match event {
        egui::Event::PointerMoved { .. } => "PointerMoved { .. }".to_owned(),
        egui::Event::MouseMoved { .. } => "MouseMoved { .. }".to_owned(),
        egui::Event::Zoom { .. } => "Zoom { .. }".to_owned(),
        egui::Event::Touch { phase, .. } => format!("Touch {{ phase: {phase:?}, .. }}"),
        egui::Event::MouseWheel { unit, .. } => format!("MouseWheel {{ unit: {unit:?}, .. }}"),

        _ => format!("{event:?}"),
    }
}
