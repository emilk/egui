use egui::{color::*, *};

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
#[derive(PartialEq)]
pub struct Scrolling {
    track_item: usize,
    tack_item_align: Align,
    offset: f32,
}

impl Default for Scrolling {
    fn default() -> Self {
        Self {
            track_item: 25,
            tack_item_align: Align::Center,
            offset: 0.0,
        }
    }
}

impl super::Demo for Scrolling {
    fn name(&self) -> &str {
        "↕ Scrolling"
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

impl super::View for Scrolling {
    fn ui(&mut self, ui: &mut Ui) {
        ui.label("This shows how you can scroll to a specific item or pixel offset");

        let mut track_item = false;
        let mut go_to_scroll_offset = false;
        let mut scroll_top = false;
        let mut scroll_bottom = false;

        ui.horizontal(|ui| {
            ui.label("Scroll to a specific item index:");
            track_item |= ui
                .add(Slider::usize(&mut self.track_item, 1..=50).text("Track Item"))
                .dragged();
        });

        ui.horizontal(|ui| {
            ui.label("Item align:");
            track_item |= ui
                .radio_value(&mut self.tack_item_align, Align::Min, "Top")
                .clicked();
            track_item |= ui
                .radio_value(&mut self.tack_item_align, Align::Center, "Center")
                .clicked();
            track_item |= ui
                .radio_value(&mut self.tack_item_align, Align::Max, "Bottom")
                .clicked();
        });

        ui.horizontal(|ui| {
            ui.label("Scroll to a specific offset:");
            go_to_scroll_offset |= ui
                .add(DragValue::f32(&mut self.offset).speed(1.0).suffix("px"))
                .dragged();
        });

        ui.horizontal(|ui| {
            scroll_top |= ui.button("Scroll to top").clicked();
            scroll_bottom |= ui.button("Scroll to bottom").clicked();
        });

        let mut scroll_area = ScrollArea::from_max_height(200.0);
        if go_to_scroll_offset {
            scroll_area = scroll_area.scroll_offset(self.offset);
        }

        ui.separator();
        let (current_scroll, max_scroll) = scroll_area.show(ui, |ui| {
            if scroll_top {
                ui.scroll_to_cursor(Align::TOP);
            }
            ui.vertical(|ui| {
                for item in 1..=50 {
                    if track_item && item == self.track_item {
                        let response =
                            ui.colored_label(Color32::YELLOW, format!("This is item {}", item));
                        response.scroll_to_me(self.tack_item_align);
                    } else {
                        ui.label(format!("This is item {}", item));
                    }
                }
            });

            if scroll_bottom {
                ui.scroll_to_cursor(Align::BOTTOM);
            }

            let margin = ui.visuals().clip_rect_margin;

            let current_scroll = ui.clip_rect().top() - ui.min_rect().top() + margin;
            let max_scroll = ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin;
            (current_scroll, max_scroll)
        });
        ui.separator();

        ui.label(format!(
            "Scroll offset: {:.0}/{:.0} px",
            current_scroll, max_scroll
        ));

        ui.separator();
        ui.vertical_centered(|ui| {
            egui::reset_button(ui, self);
            ui.add(crate::__egui_github_link_file!());
        });
    }
}
