use chrono::{Date, Datelike, Duration, NaiveDate, Utc, Weekday};
use egui::{
    Align, Area, Button, Color32, ComboBox, Direction, Frame, Id, Key, Label, Layout, Order,
    RichText, Ui, Widget,
};
use egui_dynamic_grid::{GridBuilder, Padding, Size, TableBuilder};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct Week {
    number: u8,
    days: Vec<Date<Utc>>,
}

fn month_data(year: i32, month: u32) -> Vec<Week> {
    let first = Date::from_utc(NaiveDate::from_ymd(year, month, 1), Utc);
    let mut start = first;
    while start.weekday() != Weekday::Mon {
        start = start.checked_sub_signed(Duration::days(1)).unwrap();
    }
    let mut weeks = vec![];
    let mut week = vec![];
    while start < first || start.month() == first.month() || start.weekday() != Weekday::Mon {
        week.push(start);

        if start.weekday() == Weekday::Sun {
            weeks.push(Week {
                number: start.iso_week().week() as u8,
                days: week.drain(..).collect(),
            });
        }
        start = start.checked_add_signed(Duration::days(1)).unwrap();
    }

    weeks
}

#[derive(Default, Clone, Serialize, Deserialize)]
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

struct DatePickerPopup<'a> {
    selection: &'a mut Date<Utc>,
    button_id: Id,
    combo_boxes: bool,
    arrows: bool,
    calendar: bool,
    calendar_week: bool,
}

impl<'a> DatePickerPopup<'a> {
    fn draw(&mut self, ui: &mut Ui) {
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
        GridBuilder::new(ui, Padding::new(2.0, 0.0)).vertical(|builder| {
            builder
                .rows(
                    Size::Absolute(height),
                    match (self.combo_boxes, self.arrows) {
                        (true, true) => 2,
                        (true, false) | (false, true) => 1,
                        (false, false) => 0,
                    },
                )
                .rows(
                    Size::Absolute(2.0 + (height + 2.0) * weeks.len() as f32),
                    if self.calendar { 1 } else { 0 },
                )
                .row(Size::Absolute(height))
                .build(|mut grid| {
                    if self.combo_boxes {
                        grid.horizontal_noclip(|builder| {
                            builder.columns(Size::Remainder, 3).build(|mut grid| {
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
                        grid.horizontal(|builder| {
                            builder.columns(Size::Remainder, 6).build(|mut grid| {
                                grid.cell(|ui| {
                                    ui.with_layout(
                                        Layout::top_down_justified(Align::Center),
                                        |ui| {
                                            if ui
                                                .button("<<<")
                                                .on_hover_text("substract one year")
                                                .clicked()
                                            {
                                                popup_state.year -= 1;
                                                popup_state.day = popup_state
                                                    .day
                                                    .min(popup_state.last_day_of_month());
                                                ui.memory()
                                                    .data
                                                    .insert_persisted(id, popup_state.clone());
                                            }
                                        },
                                    );
                                });
                                grid.cell(|ui| {
                                    ui.with_layout(
                                        Layout::top_down_justified(Align::Center),
                                        |ui| {
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
                                                popup_state.day = popup_state
                                                    .day
                                                    .min(popup_state.last_day_of_month());
                                                ui.memory()
                                                    .data
                                                    .insert_persisted(id, popup_state.clone());
                                            }
                                        },
                                    );
                                });
                                grid.cell(|ui| {
                                    ui.with_layout(
                                        Layout::top_down_justified(Align::Center),
                                        |ui| {
                                            if ui
                                                .button("<")
                                                .on_hover_text("substract one day")
                                                .clicked()
                                            {
                                                popup_state.day -= 1;
                                                if popup_state.day == 0 {
                                                    popup_state.month -= 1;
                                                    if popup_state.month == 0 {
                                                        popup_state.year -= 1;
                                                        popup_state.month = 12;
                                                    }
                                                    popup_state.day =
                                                        popup_state.last_day_of_month();
                                                }
                                                ui.memory()
                                                    .data
                                                    .insert_persisted(id, popup_state.clone());
                                            }
                                        },
                                    );
                                });
                                grid.cell(|ui| {
                                    ui.with_layout(
                                        Layout::top_down_justified(Align::Center),
                                        |ui| {
                                            if ui.button(">").on_hover_text("add one day").clicked()
                                            {
                                                popup_state.day += 1;
                                                if popup_state.day > popup_state.last_day_of_month()
                                                {
                                                    popup_state.day = 1;
                                                    popup_state.month += 1;
                                                    if popup_state.month > 12 {
                                                        popup_state.month = 1;
                                                        popup_state.year += 1;
                                                    }
                                                }
                                                ui.memory()
                                                    .data
                                                    .insert_persisted(id, popup_state.clone());
                                            }
                                        },
                                    );
                                });
                                grid.cell(|ui| {
                                    ui.with_layout(
                                        Layout::top_down_justified(Align::Center),
                                        |ui| {
                                            if ui
                                                .button(">>")
                                                .on_hover_text("add one month")
                                                .clicked()
                                            {
                                                popup_state.month += 1;
                                                if popup_state.month > 12 {
                                                    popup_state.month = 1;
                                                    popup_state.year += 1;
                                                }
                                                popup_state.day = popup_state
                                                    .day
                                                    .min(popup_state.last_day_of_month());
                                                ui.memory()
                                                    .data
                                                    .insert_persisted(id, popup_state.clone());
                                            }
                                        },
                                    );
                                });
                                grid.cell(|ui| {
                                    ui.with_layout(
                                        Layout::top_down_justified(Align::Center),
                                        |ui| {
                                            if ui
                                                .button(">>>")
                                                .on_hover_text("add one year")
                                                .clicked()
                                            {
                                                popup_state.year += 1;
                                                popup_state.day = popup_state
                                                    .day
                                                    .min(popup_state.last_day_of_month());
                                                ui.memory()
                                                    .data
                                                    .insert_persisted(id, popup_state.clone());
                                            }
                                        },
                                    );
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
                                    for name in ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"] {
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
                                                            let fill_color = if popup_state.year
                                                                == day.year()
                                                                && popup_state.month == day.month()
                                                                && popup_state.day == day.day()
                                                            {
                                                                Color32::DARK_BLUE
                                                            } else if day.weekday() == Weekday::Sat
                                                                || day.weekday() == Weekday::Sun
                                                            {
                                                                Color32::DARK_RED
                                                            } else {
                                                                Color32::BLACK
                                                            };
                                                            let text_color = if day == today {
                                                                Color32::RED
                                                            } else if day.month()
                                                                == popup_state.month
                                                            {
                                                                Color32::WHITE
                                                            } else {
                                                                Color32::from_gray(80)
                                                            };

                                                            let button = Button::new(
                                                                RichText::new(format!(
                                                                    "{}",
                                                                    day.day()
                                                                ))
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

                    grid.horizontal(|builder| {
                        builder.columns(Size::Remainder, 3).build(|mut grid| {
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

#[derive(Default, Clone, Serialize, Deserialize)]
struct DatePickerButtonState {
    picker_visible: bool,
}

pub struct DatePickerButton<'a> {
    selection: &'a mut Date<Utc>,
    id_source: Option<&'a str>,
    combo_boxes: bool,
    arrows: bool,
    calendar: bool,
    calendar_week: bool,
}

impl<'a> DatePickerButton<'a> {
    pub fn new(selection: &'a mut Date<Utc>) -> Self {
        Self {
            selection,
            id_source: None,
            combo_boxes: true,
            arrows: true,
            calendar: true,
            calendar_week: true,
        }
    }

    /// Add id source.
    /// Must be set if multiple date picker buttons are in the same Ui.
    pub fn id_source(mut self, id_source: &'a str) -> Self {
        self.id_source = Some(id_source);
        self
    }

    /// Show combo boxes in date picker popup. (Default: true)
    pub fn combo_boxes(mut self, combo_boxes: bool) -> Self {
        self.combo_boxes = combo_boxes;
        self
    }

    /// Show arrows in date picker popup. (Default: true)
    pub fn arrows(mut self, arrows: bool) -> Self {
        self.arrows = arrows;
        self
    }

    /// Show calendar in date picker popup. (Default: true)
    pub fn calendar(mut self, calendar: bool) -> Self {
        self.calendar = calendar;
        self
    }

    /// Show calendar week in date picker popup. (Default: true)
    pub fn calendar_week(mut self, week: bool) -> Self {
        self.calendar_week = week;
        self
    }
}

impl<'a> Widget for DatePickerButton<'a> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let id = ui.make_persistent_id(&self.id_source);
        let mut button_state = ui
            .memory()
            .data
            .get_persisted::<DatePickerButtonState>(id)
            .unwrap_or_default();

        let mut text = RichText::new(format!("{} 📆", self.selection.format("%d.%m.%Y")));
        let visuals = ui.visuals().widgets.open;
        if button_state.picker_visible {
            text = text.color(visuals.text_color());
        }
        let mut button = Button::new(text);
        if button_state.picker_visible {
            button = button.fill(visuals.bg_fill).stroke(visuals.bg_stroke);
        }
        let button_response = ui.add(button);
        if button_response.clicked() {
            button_state.picker_visible = true;
            ui.memory().data.insert_persisted(id, button_state.clone());
        }

        if button_state.picker_visible {
            let width = 333.0;
            let mut pos = button_response.rect.left_bottom();
            let width_with_padding =
                width + ui.style().spacing.item_spacing.x + ui.style().spacing.window_padding.x;
            if pos.x + width_with_padding > ui.clip_rect().right() {
                pos.x = button_response.rect.right() - width_with_padding;
            }

            let area_response = Area::new(ui.make_persistent_id(&self.id_source))
                .order(Order::Foreground)
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
                    let frame = Frame::popup(ui.style());
                    frame.show(ui, |ui| {
                        ui.set_min_width(width);
                        ui.set_max_width(width);

                        DatePickerPopup {
                            selection: self.selection,
                            button_id: id,
                            combo_boxes: self.combo_boxes,
                            arrows: self.arrows,
                            calendar: self.calendar,
                            calendar_week: self.calendar_week,
                        }
                        .draw(ui)
                    })
                })
                .response;

            if !button_response.clicked()
                && (ui.input().key_pressed(Key::Escape) || area_response.clicked_elsewhere())
            {
                button_state.picker_visible = false;
                ui.memory().data.insert_persisted(id, button_state);
            }
        }

        button_response
    }
}
