use super::{button::DatePickerButtonState, month_data};
use crate::{GridBuilder, Padding, Size, TableBuilder};
use chrono::{Date, Datelike, NaiveDate, Utc, Weekday};
use egui::{Align, Button, Color32, ComboBox, Direction, Id, Label, Layout, RichText, Ui};

#[derive(Default, Clone)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct DatePickerPopupState {
    year: i32,
    month: u32,
    day: u32,
    setup: bool,
}

impl DatePickerPopupState {
    fn last_day_of_month(&self) -> u32 {
        let date: Date<Utc> = Date::from_utc(NaiveDate::from_ymd(self.year, self.month, 1), Utc);
        date.with_day(31)
            .map(|_| 31)
            .or_else(|| date.with_day(30).map(|_| 30))
            .or_else(|| date.with_day(29).map(|_| 29))
            .unwrap_or(28)
    }
}

pub(crate) struct DatePickerPopup<'a> {
    pub selection: &'a mut Date<Utc>,
    pub button_id: Id,
    pub combo_boxes: bool,
    pub arrows: bool,
    pub calendar: bool,
    pub calendar_week: bool,
}

impl<'a> DatePickerPopup<'a> {
    pub fn draw(&mut self, ui: &mut Ui) {
        let id = ui.make_persistent_id("date_picker");
        let today = chrono::offset::Utc::now().date();
        let mut popup_state = ui
            .memory()
            .data
            .get_persisted::<DatePickerPopupState>(id)
            .unwrap_or_default();
        if !popup_state.setup {
            popup_state.year = self.selection.year();
            popup_state.month = self.selection.month();
            popup_state.day = self.selection.day();
            popup_state.setup = true;
            ui.memory().data.insert_persisted(id, popup_state.clone());
        }

        let weeks = month_data(popup_state.year, popup_state.month);
        let mut close = false;
        let height = 20.0;
        GridBuilder::new(ui, Padding::new(2.0, 0.0))
            .sizes(
                Size::Absolute(height),
                match (self.combo_boxes, self.arrows) {
                    (true, true) => 2,
                    (true, false) | (false, true) => 1,
                    (false, false) => 0,
                },
            )
            .sizes(
                Size::Absolute(2.0 + (height + 2.0) * weeks.len() as f32),
                if self.calendar { 1 } else { 0 },
            )
            .size(Size::Absolute(height))
            .vertical(|mut grid| {
                if self.combo_boxes {
                    grid.grid_noclip(|builder| {
                        builder.sizes(Size::Remainder, 3).horizontal(|mut grid| {
                            grid.cell_noclip(|ui| {
                                ComboBox::from_id_source("date_picker_year")
                                    .selected_text(format!("{}", popup_state.year))
                                    .show_ui(ui, |ui| {
                                        for year in today.year() - 5..today.year() + 10 {
                                            if ui
                                                .selectable_value(
                                                    &mut popup_state.year,
                                                    year,
                                                    format!("{}", year),
                                                )
                                                .changed()
                                            {
                                                ui.memory()
                                                    .data
                                                    .insert_persisted(id, popup_state.clone());
                                            }
                                        }
                                    });
                            });
                            grid.cell_noclip(|ui| {
                                ComboBox::from_id_source("date_picker_month")
                                    .selected_text(format!("{}", popup_state.month))
                                    .show_ui(ui, |ui| {
                                        for month in 1..=12 {
                                            if ui
                                                .selectable_value(
                                                    &mut popup_state.month,
                                                    month,
                                                    format!("{}", month),
                                                )
                                                .changed()
                                            {
                                                ui.memory()
                                                    .data
                                                    .insert_persisted(id, popup_state.clone());
                                            }
                                        }
                                    });
                            });
                            grid.cell_noclip(|ui| {
                                ComboBox::from_id_source("date_picker_day")
                                    .selected_text(format!("{}", popup_state.day))
                                    .show_ui(ui, |ui| {
                                        for day in 1..=popup_state.last_day_of_month() {
                                            if ui
                                                .selectable_value(
                                                    &mut popup_state.day,
                                                    day,
                                                    format!("{}", day),
                                                )
                                                .changed()
                                            {
                                                ui.memory()
                                                    .data
                                                    .insert_persisted(id, popup_state.clone());
                                            }
                                        }
                                    });
                            });
                        })
                    });
                }

                if self.arrows {
                    grid.grid(|builder| {
                        builder.sizes(Size::Remainder, 6).horizontal(|mut grid| {
                            grid.cell(|ui| {
                                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                    if ui
                                        .button("<<<")
                                        .on_hover_text("substract one year")
                                        .clicked()
                                    {
                                        popup_state.year -= 1;
                                        popup_state.day =
                                            popup_state.day.min(popup_state.last_day_of_month());
                                        ui.memory().data.insert_persisted(id, popup_state.clone());
                                    }
                                });
                            });
                            grid.cell(|ui| {
                                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                    if ui
                                        .button("<<")
                                        .on_hover_text("substract one month")
                                        .clicked()
                                    {
                                        popup_state.month -= 1;
                                        if popup_state.month == 0 {
                                            popup_state.month = 12;
                                            popup_state.year -= 1;
                                        }
                                        popup_state.day =
                                            popup_state.day.min(popup_state.last_day_of_month());
                                        ui.memory().data.insert_persisted(id, popup_state.clone());
                                    }
                                });
                            });
                            grid.cell(|ui| {
                                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                    if ui.button("<").on_hover_text("substract one day").clicked() {
                                        popup_state.day -= 1;
                                        if popup_state.day == 0 {
                                            popup_state.month -= 1;
                                            if popup_state.month == 0 {
                                                popup_state.year -= 1;
                                                popup_state.month = 12;
                                            }
                                            popup_state.day = popup_state.last_day_of_month();
                                        }
                                        ui.memory().data.insert_persisted(id, popup_state.clone());
                                    }
                                });
                            });
                            grid.cell(|ui| {
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
                                        ui.memory().data.insert_persisted(id, popup_state.clone());
                                    }
                                });
                            });
                            grid.cell(|ui| {
                                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                    if ui.button(">>").on_hover_text("add one month").clicked() {
                                        popup_state.month += 1;
                                        if popup_state.month > 12 {
                                            popup_state.month = 1;
                                            popup_state.year += 1;
                                        }
                                        popup_state.day =
                                            popup_state.day.min(popup_state.last_day_of_month());
                                        ui.memory().data.insert_persisted(id, popup_state.clone());
                                    }
                                });
                            });
                            grid.cell(|ui| {
                                ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                    if ui.button(">>>").on_hover_text("add one year").clicked() {
                                        popup_state.year += 1;
                                        popup_state.day =
                                            popup_state.day.min(popup_state.last_day_of_month());
                                        ui.memory().data.insert_persisted(id, popup_state.clone());
                                    }
                                });
                            });
                        })
                    });
                }

                if self.calendar {
                    grid.cell(|ui| {
                        TableBuilder::new(ui, Padding::new(2.0, 0.0))
                            .scroll(false)
                            .columns(Size::Remainder, if self.calendar_week { 8 } else { 7 })
                            .header(height, |mut header| {
                                if self.calendar_week {
                                    header.col(|ui| {
                                        ui.with_layout(
                                            Layout::centered_and_justified(Direction::TopDown),
                                            |ui| {
                                                ui.add(Label::new("Week"));
                                            },
                                        );
                                    });
                                }

                                //TODO: Locale
                                for name in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] {
                                    header.col(|ui| {
                                        ui.with_layout(
                                            Layout::centered_and_justified(Direction::TopDown),
                                            |ui| {
                                                ui.add(Label::new(name));
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
                                                ui.add(Label::new(format!("{}", week.number)));
                                            });
                                        }
                                        for day in week.days {
                                            row.col(|ui| {
                                                ui.with_layout(
                                                    Layout::top_down_justified(Align::Center),
                                                    |ui| {
                                                        //TODO: Colors from egui style
                                                        let fill_color = if popup_state.year
                                                            == day.year()
                                                            && popup_state.month == day.month()
                                                            && popup_state.day == day.day()
                                                        {
                                                            ui.visuals().selection.bg_fill
                                                        } else if day.weekday() == Weekday::Sat
                                                            || day.weekday() == Weekday::Sun
                                                        {
                                                            Color32::DARK_RED
                                                        } else {
                                                            Color32::BLACK
                                                        };
                                                        let text_color = if day == today {
                                                            Color32::RED
                                                        } else if day.month() == popup_state.month {
                                                            Color32::WHITE
                                                        } else {
                                                            Color32::from_gray(80)
                                                        };

                                                        let button = Button::new(
                                                            RichText::new(format!("{}", day.day()))
                                                                .color(text_color),
                                                        )
                                                        .fill(fill_color);

                                                        if ui.add(button).clicked() {
                                                            popup_state.year = day.year();
                                                            popup_state.month = day.month();
                                                            popup_state.day = day.day();
                                                            ui.memory().data.insert_persisted(
                                                                id,
                                                                popup_state.clone(),
                                                            );
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

                grid.grid(|builder| {
                    builder.sizes(Size::Remainder, 3).horizontal(|mut grid| {
                        grid.empty();
                        grid.cell(|ui| {
                            ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                if ui.button("Abbrechen").clicked() {
                                    close = true;
                                }
                            });
                        });
                        grid.cell(|ui| {
                            ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                                if ui.button("Speichern").clicked() {
                                    *self.selection = Date::from_utc(
                                        NaiveDate::from_ymd(
                                            popup_state.year,
                                            popup_state.month,
                                            popup_state.day,
                                        ),
                                        Utc,
                                    );
                                    close = true;
                                }
                            });
                        });
                    })
                });
            });

        if close {
            popup_state.setup = false;
            ui.memory().data.insert_persisted(id, popup_state);

            ui.memory()
                .data
                .get_persisted_mut_or_default::<DatePickerButtonState>(self.button_id)
                .picker_visible = false;
        }
    }
}
