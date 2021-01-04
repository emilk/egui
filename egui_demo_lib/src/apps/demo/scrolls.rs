use egui::{color::*, *};

#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Scrolls {
    track_item: usize,
    tracking: bool,
    offset: f32,
}

impl Default for Scrolls {
    fn default() -> Self {
        Self {
            track_item: 25,
            tracking: true,
            offset: 0.0,
        }
    }
}

impl Scrolls {
    pub fn ui(&mut self, ui: &mut Ui) {
        ScrollArea::from_max_height(200.0).show(ui, |ui| {
            ui.label(crate::LOREM_IPSUM_LONG);
            ui.label(crate::LOREM_IPSUM_LONG);
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.tracking, "Track")
                .on_hover_text("The scroll position will track the selected item");
            ui.add(Slider::usize(&mut self.track_item, 1..=50).text("Track Item"));
        });
        let (scroll_offset, _) = ui.horizontal(|ui| {
            let scroll_offset = ui.small_button("Scroll Offset").clicked;
            ui.add(DragValue::f32(&mut self.offset).speed(1.0).suffix("px"));
            scroll_offset
        });

        let scroll_top = ui.button("Scroll to top").clicked;
        let scroll_bottom = ui.button("Scroll to bottom").clicked;
        if scroll_bottom || scroll_top {
            self.tracking = false;
        }

        const TITLES: [&str; 3] = ["Top", "Middle", "Bottom"];
        const ALIGNS: [Align; 3] = [Align::Min, Align::Center, Align::Max];
        ui.columns(3, |cols| {
            for (i, col) in cols.iter_mut().enumerate() {
                col.colored_label(Color32::WHITE, TITLES[i]);
                let mut scroll_area = ScrollArea::from_max_height(200.0).id_source(i);
                if scroll_offset {
                    self.tracking = false;
                    scroll_area = scroll_area.scroll_offset(self.offset);
                }

                let (current_scroll, max_scroll) = scroll_area.show(col, |ui| {
                    if scroll_top {
                        ui.scroll_to_cursor(Align::top());
                    }
                    ui.vertical(|ui| {
                        for item in 1..=50 {
                            if self.tracking && item == self.track_item {
                                let response =
                                    ui.colored_label(Color32::YELLOW, format!("Item {}", item));
                                response.scroll_to_me(ALIGNS[i]);
                            } else {
                                ui.label(format!("Item {}", item));
                            }
                        }
                    });

                    if scroll_bottom {
                        ui.scroll_to_cursor(Align::bottom());
                    }

                    let margin = ui.style().visuals.clip_rect_margin;
                    (
                        ui.clip_rect().top() - ui.min_rect().top() + margin,
                        ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin,
                    )
                });
                col.colored_label(
                    Color32::WHITE,
                    format!("{:.0}/{:.0}", current_scroll, max_scroll),
                );
            }
        });
    }
}
