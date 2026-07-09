use jiff::civil::{Date, Weekday};

use egui::{Align, Button, Color32, ComboBox, Direction, Id, Layout, RichText, Ui, Vec2};

use super::{button::DatePickerButtonState, month_data};

use crate::{Column, Size, StripBuilder, TableBuilder};

#[derive(Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
struct DatePickerPopupState {
    year: i16,
    month: i8,
    day: i8,
    setup: bool,
    year_scroll_needed: bool,
}

impl DatePickerPopupState {
    fn last_day_of_month(&self) -> i8 {
        Date::new(self.year, self.month, 1)
            .expect("Could not create Date")
            .days_in_month()
    }
}

pub(crate) struct DatePickerPopup<'a> {
    pub selection: &'a mut Date,
    pub button_id: Id,
    pub combo_boxes: bool,
    pub arrows: bool,
    pub calendar: bool,
    pub calendar_week: bool,
    pub highlight_weekends: bool,
    pub start_end_years: Option<std::ops::RangeInclusive<i16>>,
    pub reverse_years: bool,
    pub year_scroll_to: Option<i16>,
}

impl DatePickerPopup<'_> {
    /// Returns `true` if user pressed `Save` button.
    pub fn draw(&mut self, ui: &mut Ui) -> bool {
        let id = ui.make_persistent_id("date_picker");
        let today = jiff::Zoned::now().date();
        let mut popup_state = ui
            .data_mut(|data| data.get_persisted::<DatePickerPopupState>(id))
            .unwrap_or_default();
        if !popup_state.setup {
            popup_state.year = self.selection.year();
            popup_state.month = self.selection.month();
            popup_state.day = self.selection.day();
            popup_state.setup = true;
            popup_state.year_scroll_needed = true;
            ui.data_mut(|data| data.insert_persisted(id, popup_state.clone()));
        }

        let weeks = month_data(popup_state.year, popup_state.month);
        let (mut close, mut saved) = (false, false);
        let height = 20.0;
        let spacing = 2.0;
        ui.spacing_mut().item_spacing = Vec2::splat(spacing);

        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

        StripBuilder::new(ui)
            .clip(false)
            .sizes(
                Size::exact(height),
                match (self.combo_boxes, self.arrows) {
                    (true, true) => 2,
                    (true, false) | (false, true) => 1,
                    (false, false) => 0,
                },
            )
            .sizes(
                Size::exact((spacing + height) * (weeks.len() + 1) as f32),
                self.calendar as usize,
            )
            .size(Size::exact(height))
            .vertical(|mut strip| {
                if self.combo_boxes {
                    strip.strip(|builder| {
                        builder.sizes(Size::remainder(), 3).horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ComboBox::from_id_salt("date_picker_year")
                                    .selected_text(popup_state.year.to_string())
                                    .show_ui(ui, |ui| {
                                        let (start_year, end_year) = match &self.start_end_years {
                                            Some(range) => (*range.start(), *range.end()),
                                            None => (today.year() - 100, today.year() + 10),
                                        };
                                        let scroll_to_year =
                                            self.year_scroll_to.unwrap_or(popup_state.year);
                                        let years: Vec<i16> = if self.reverse_years {
                                            (start_year..=end_year).rev().collect()
                                        } else {
                                            (start_year..=end_year).collect()
                                        };
                                        for year in years {
                                            let resp = ui.selectable_value(
                                                &mut popup_state.year,
                                                year,
                                                year.to_string(),
                                            );
                                            if popup_state.year_scroll_needed
                                                && year == scroll_to_year
                                            {
                                                resp.scroll_to_me(Some(Align::Center));
                                                popup_state.year_scroll_needed = false;
                                                ui.memory_mut(|mem| {
                                                    mem.data
                                                        .insert_persisted(id, popup_state.clone());
                                                });
                                            }
                                            if resp.changed() {
                                                popup_state.day = popup_state
                                                    .day
                                                    .min(popup_state.last_day_of_month());
                                                ui.memory_mut(|mem| {
                                                    mem.data
                                                        .insert_persisted(id, popup_state.clone());
                                                });
                                            }
                                        }
                                    });
                            });
                            strip.cell(|ui| {
                                ComboBox::from_id_salt("date_picker_month")
                                    .selected_text(month_name(popup_state.month))
                                    .show_ui(ui, |ui| {
                                        for month in 1i8..=12 {
                                            if ui
                                                .selectable_value(
                                                    &mut popup_state.month,
                                                    month,
                                                    month_name(month),
                                                )
                                                .changed()
                                            {
                                                popup_state.day = popup_state
                                                    .day
                                                    .min(popup_state.last_day_of_month());
                                                ui.memory_mut(|mem| {
                                                    mem.data
                                                        .insert_persisted(id, popup_state.clone());
                                                });
                                            }
                                        }
                                    });
                            });
                            strip.cell(|ui| {
                                ComboBox::from_id_salt("date_picker_day")
                                    .selected_text(popup_state.day.to_string())
                                    .show_ui(ui, |ui| {
                                        for day in 1i8..=popup_state.last_day_of_month() {
                                            if ui
                                                .selectable_value(
                                                    &mut popup_state.day,
                                                    day,
                                                    day.to_string(),
                                                )
                                                .changed()
                                            {
                                                ui.memory_mut(|mem| {
                                                    mem.data
                                                        .insert_persisted(id, popup_state.clone());
                                                });
                                            }
                                        }
                                    });
                            });
                        });
                    });
                }

                if self.arrows {
                    strip.strip(|builder| {
                        builder.sizes(Size::remainder(), 6).horizontal(|mut strip| {
                            strip.cell(|ui| {
                                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                    if ui
                                        .button("<<<")
                                        .on_hover_text("subtract one year")
                                        .clicked()
                                    {
                                        popup_state.year -= 1;
                                        popup_state.day =
                                            popup_state.day.min(popup_state.last_day_of_month());
                                        ui.data_mut(|data| {
                                            data.insert_persisted(id, popup_state.clone());
                                        });
                                    }
                                });
                            });
                            strip.cell(|ui| {
                                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                    if ui
                                        .button("<<")
                                        .on_hover_text("subtract one month")
                                        .clicked()
                                    {
                                        popup_state.month -= 1;
                                        if popup_state.month == 0 {
                                            popup_state.month = 12;
                                            popup_state.year -= 1;
                                        }
                                        popup_state.day =
                                            popup_state.day.min(popup_state.last_day_of_month());
                                        ui.data_mut(|data| {
                                            data.insert_persisted(id, popup_state.clone());
                                        });
                                    }
                                });
                            });
                            strip.cell(|ui| {
                                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                    if ui.button("<").on_hover_text("subtract one day").clicked() {
                                        popup_state.day -= 1;
                                        if popup_state.day == 0 {
                                            popup_state.month -= 1;
                                            if popup_state.month == 0 {
                                                popup_state.year -= 1;
                                                popup_state.month = 12;
                                            }
                                            popup_state.day = popup_state.last_day_of_month();
                                        }
                                        ui.data_mut(|data| {
                                            data.insert_persisted(id, popup_state.clone());
                                        });
                                    }
                                });
                            });
                            strip.cell(|ui| {
                                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                    if ui.button(">").on_hover_text("add one day").clicked() {
                                        popup_state.day += 1;
                                        if popup_state.day > popup_state.last_day_of_month() {
                                            popup_state.day = 1;
                                            popup_state.month += 1;
                                            if popup_state.month > 12 {
                                                popup_state.month = 1;
                                                popup_state.year += 1;
                                            }
                                        }
                                        ui.data_mut(|data| {
                                            data.insert_persisted(id, popup_state.clone());
                                        });
                                    }
                                });
                            });
                            strip.cell(|ui| {
                                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                    if ui.button(">>").on_hover_text("add one month").clicked() {
                                        popup_state.month += 1;
                                        if popup_state.month > 12 {
                                            popup_state.month = 1;
                                            popup_state.year += 1;
                                        }
                                        popup_state.day =
                                            popup_state.day.min(popup_state.last_day_of_month());
                                        ui.data_mut(|data| {
                                            data.insert_persisted(id, popup_state.clone());
                                        });
                                    }
                                });
                            });
                            strip.cell(|ui| {
                                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                    if ui.button(">>>").on_hover_text("add one year").clicked() {
                                        popup_state.year += 1;
                                        popup_state.day =
                                            popup_state.day.min(popup_state.last_day_of_month());
                                        ui.data_mut(|data| {
                                            data.insert_persisted(id, popup_state.clone());
                                        });
                                    }
                                });
                            });
                        });
                    });
                }

                if self.calendar {
                    strip.cell(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(1.0, 2.0);
                        TableBuilder::new(ui)
                            .vscroll(false)
                            .columns(Column::remainder(), if self.calendar_week { 8 } else { 7 })
                            .header(height, |mut header| {
                                if self.calendar_week {
                                    header.col(|ui| {
                                        ui.with_layout(
                                            Layout::centered_and_justified(Direction::TopDown),
                                            |ui| {
                                                ui.label("Week");
                                            },
                                        );
                                    });
                                }

                                //TODO(elwerene): Locale
                                for name in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
                                    header.col(|ui| {
                                        ui.with_layout(
                                            Layout::centered_and_justified(Direction::TopDown),
                                            |ui| {
                                                ui.label(name);
                                            },
                                        );
                                    });
                                }
                            })
                            .body(|mut body| {
                                for week in weeks {
                                    body.row(height, |mut row| {
                                        if self.calendar_week {
                                            row.col(|ui| {
                                                ui.label(week.number.to_string());
                                            });
                                        }
                                        for day in week.days {
                                            row.col(|ui| {
                                                ui.with_layout(
                                                    Layout::top_down_justified(Align::Center),
                                                    |ui| {
                                                        let fill_color = if popup_state.year
                                                            == day.year()
                                                            && popup_state.month == day.month()
                                                            && popup_state.day == day.day()
                                                        {
                                                            ui.visuals().selection.bg_fill
                                                        } else if (matches!(
                                                            day.weekday(),
                                                            Weekday::Saturday | Weekday::Sunday
                                                        )) && self.highlight_weekends
                                                        {
                                                            if ui.visuals().dark_mode {
                                                                Color32::DARK_RED
                                                            } else {
                                                                Color32::LIGHT_RED
                                                            }
                                                        } else {
                                                            ui.visuals().extreme_bg_color
                                                        };

                                                        let mut text_color = ui
                                                            .visuals()
                                                            .widgets
                                                            .inactive
                                                            .text_color();

                                                        if day.month() != popup_state.month {
                                                            text_color =
                                                                text_color.linear_multiply(0.5);
                                                        }

                                                        let button_response = ui.add(
                                                            Button::new(
                                                                RichText::new(
                                                                    day.day().to_string(),
                                                                )
                                                                .color(text_color),
                                                            )
                                                            .fill(fill_color),
                                                        );

                                                        if day == today {
                                                            let stroke = ui
                                                                .visuals()
                                                                .widgets
                                                                .inactive
                                                                .fg_stroke;
                                                            ui.painter().circle_stroke(
                                                                button_response.rect.center(),
                                                                8.0,
                                                                stroke,
                                                            );
                                                        }

                                                        if button_response.clicked() {
                                                            popup_state.year = day.year();
                                                            popup_state.month = day.month();
                                                            popup_state.day = day.day();
                                                            ui.data_mut(|data| {
                                                                data.insert_persisted(
                                                                    id,
                                                                    popup_state.clone(),
                                                                );
                                                            });
                                                        }
                                                    },
                                                );
                                            });
                                        }
                                    });
                                }
                            });
                    });
                }

                strip.strip(|builder| {
                    builder.sizes(Size::remainder(), 3).horizontal(|mut strip| {
                        strip.empty();
                        strip.cell(|ui| {
                            ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                if ui.button("Cancel").clicked() {
                                    close = true;
                                }
                            });
                        });
                        strip.cell(|ui| {
                            ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                if ui.button("Save").clicked() {
                                    *self.selection = Date::new(
                                        popup_state.year,
                                        popup_state.month,
                                        popup_state.day,
                                    )
                                    .expect("Could not create Date");
                                    saved = true;
                                    close = true;
                                }
                            });
                        });
                    });
                });
            });

        if close {
            popup_state.setup = false;
            ui.data_mut(|data| {
                data.insert_persisted(id, popup_state);
                data.get_persisted_mut_or_default::<DatePickerButtonState>(self.button_id)
                    .picker_visible = false;
            });
        }

        saved && close
    }
}

fn month_name(i: i8) -> &'static str {
    match i {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => panic!("Unknown month: {i}"),
    }
}
