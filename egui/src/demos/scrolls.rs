use crate::{color::*, *};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Scrolls {
    track_item: usize,
    tracking: bool,
    offset: f32,
    center_factor: f32,
}

impl Default for Scrolls {
    fn default() -> Self {
        Self {
            track_item: 25,
            tracking: true,
            offset: 0.0,
            center_factor: 0.3,
        }
    }
}

impl Scrolls {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.tracking, "Track")
                .on_hover_text("The scroll position will track the selected item");
            ui.add(Slider::usize(&mut self.track_item, 1..=50).text("Track Item"));
        });
        ui.add(Slider::f32(&mut self.center_factor, 0.0..=1.0).text("Custom scroll center factor"));
        let (scroll_offset, _) = ui.horizontal(|ui| {
            let scroll_offset = ui.small_button("Scroll Offset").clicked;
            ui.add(DragValue::f32(&mut self.offset).speed(1.0).suffix("px"));
            scroll_offset
        });

        let titles = ["Top", "25%", "Middle", "75%", "Bottom", "Custom"];
        ui.columns(6, |cols| {
            for (i, col) in cols.iter_mut().enumerate() {
                col.label(titles[i]);
                let mut scroll_area = ScrollArea::from_max_height(200.0).id_source(i);
                if scroll_offset {
                    self.tracking = false;
                    scroll_area = scroll_area.scroll_offset(Vec2::new(0.0, self.offset));
                }

                let (current_scroll, max_scroll) = scroll_area.show(col, |ui| {
                    ui.vertical(|ui| {
                        for item in 1..=50 {
                            if self.tracking && item == self.track_item {
                                let response = ui.colored_label(YELLOW, format!("Item {}", item));
                                let scroll_center_factor = if i == 5 {
                                    self.center_factor
                                } else {
                                    0.25 * i as f32
                                };
                                response.scroll_to_me(scroll_center_factor);
                            } else {
                                ui.label(format!("Item {}", item));
                            }
                        }
                    });

                    let margin = ui.style().visuals.clip_rect_margin;
                    (
                        ui.clip_rect().top() - ui.min_rect().top() + margin,
                        ui.min_rect().height() - ui.clip_rect().height() + 2.0 * margin,
                    )
                });
                col.label(format!("{:.0}/{:.0}", current_scroll, max_scroll));
            }
        });
    }
}
