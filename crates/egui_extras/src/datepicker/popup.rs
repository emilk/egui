use jiff::civil::{Date, Weekday};

use egui::{Align, Align2, ComboBox, Id, Rect, Sense, Stroke, TextStyle, Ui, Vec2, pos2, vec2};

use super::{Week, button::DatePickerButtonState, month_data};

use crate::{Size, StripBuilder};

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
    /// Moves the selected date by `span`. Months and years clamp the day to the
    /// last day of the target month; days roll over into the next/previous month.
    fn shift(&mut self, span: jiff::Span) {
        if let Ok(date) =
            Date::new(self.year, self.month, self.day).and_then(|date| date.checked_add(span))
        {
            self.year = date.year();
            self.month = date.month();
            self.day = date.day();
        }
    }

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
    pub start_end_years: Option<core::ops::RangeInclusive<i16>>,
    pub reverse_years: bool,
    pub year_scroll_to: Option<i16>,
}

impl DatePickerPopup<'_> {
    /// Height of one row of the calendar grid (header and weeks).
    const CELL_HEIGHT: f32 = 24.0;

    /// Space between the weekday header and the first week.
    const HEADER_GAP: f32 = 6.0;

    /// Width of the compact navigation buttons.
    const NAV_BUTTON_WIDTH: f32 = 28.0;

    /// Persists the popup state so it survives to the next frame.
    fn store(ui: &Ui, id: Id, popup_state: &DatePickerPopupState) {
        ui.data_mut(|data| data.insert_persisted(id, popup_state.clone()));
    }

    /// One compact row: `<`, the month and year combo boxes, and `>`.
    /// The arrows move the selected date by one month.
    fn header_row(
        &self,
        builder: StripBuilder<'_>,
        popup_state: &mut DatePickerPopupState,
        today: Date,
        id: Id,
    ) {
        let mut builder = builder;
        if self.arrows {
            builder = builder.size(Size::exact(Self::NAV_BUTTON_WIDTH));
        }
        if self.combo_boxes {
            builder = builder.size(Size::remainder());
        }
        if self.arrows {
            builder = builder.size(Size::exact(Self::NAV_BUTTON_WIDTH));
        }

        builder.horizontal(|mut strip| {
            if self.arrows {
                strip.cell(|ui| {
                    if Self::nav_button(ui, "<", "previous month").clicked() {
                        popup_state.shift(jiff::Span::new().months(-1));
                        Self::store(ui, id, popup_state);
                    }
                });
            }
            if self.combo_boxes {
                strip.cell(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let width = (ui.available_width() - 2.0) / 2.0;
                    ui.horizontal(|ui| {
                        Self::month_combo(ui, popup_state, id, width);
                        self.year_combo(ui, popup_state, today, id, width);
                    });
                });
            }
            if self.arrows {
                strip.cell(|ui| {
                    if Self::nav_button(ui, ">", "next month").clicked() {
                        popup_state.shift(jiff::Span::new().months(1));
                        Self::store(ui, id, popup_state);
                    }
                });
            }
        });
    }

    fn year_combo(
        &self,
        ui: &mut Ui,
        popup_state: &mut DatePickerPopupState,
        today: Date,
        id: Id,
        width: f32,
    ) {
        ComboBox::from_id_salt("date_picker_year")
            .width(width)
            .selected_text(popup_state.year.to_string())
            .show_ui(ui, |ui| {
                let (start_year, end_year) = match &self.start_end_years {
                    Some(range) => (*range.start(), *range.end()),
                    None => (today.year() - 100, today.year() + 10),
                };
                let scroll_to_year = self.year_scroll_to.unwrap_or(popup_state.year);
                let years: Vec<i16> = if self.reverse_years {
                    (start_year..=end_year).rev().collect()
                } else {
                    (start_year..=end_year).collect()
                };
                for year in years {
                    let resp = ui.selectable_value(&mut popup_state.year, year, year.to_string());
                    if popup_state.year_scroll_needed && year == scroll_to_year {
                        resp.scroll_to_me(Some(Align::Center));
                        popup_state.year_scroll_needed = false;
                        Self::store(ui, id, popup_state);
                    }
                    if resp.changed() {
                        popup_state.day = popup_state.day.min(popup_state.last_day_of_month());
                        Self::store(ui, id, popup_state);
                    }
                }
            });
    }

    fn month_combo(ui: &mut Ui, popup_state: &mut DatePickerPopupState, id: Id, width: f32) {
        ComboBox::from_id_salt("date_picker_month")
            .width(width)
            .selected_text(month_name(popup_state.month))
            .show_ui(ui, |ui| {
                for month in 1i8..=12 {
                    if ui
                        .selectable_value(&mut popup_state.month, month, month_name(month))
                        .changed()
                    {
                        popup_state.day = popup_state.day.min(popup_state.last_day_of_month());
                        Self::store(ui, id, popup_state);
                    }
                }
            });
    }

    /// A compact, frameless navigation button: text only, with a rounded highlight on hover.
    fn nav_button(ui: &mut Ui, text: &str, tooltip: &str) -> egui::Response {
        let font = TextStyle::Body.resolve(ui.style());
        let size = vec2(Self::NAV_BUTTON_WIDTH, Self::CELL_HEIGHT - 2.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let enabled = ui.is_enabled();
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, tooltip));
        let response = response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(tooltip);

        let visuals = ui.visuals();
        let widget = visuals.widgets.style(&response);
        if response.hovered() || response.is_pointer_button_down_on() {
            ui.painter()
                .rect_filled(rect, rect.height() / 2.0, widget.weak_bg_fill);
        }
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            text,
            font,
            widget.text_color(),
        );
        response
    }

    /// A month spans at most this many weeks. The grid always reserves room for all of them,
    /// so the popup keeps the same size (and doesn't jump) when switching months.
    const MAX_WEEKS: usize = 6;

    /// Total height needed by [`Self::calendar_grid`].
    fn calendar_height() -> f32 {
        Self::CELL_HEIGHT * (Self::MAX_WEEKS + 1) as f32 + Self::HEADER_GAP
    }

    /// Paints the month as a real calendar: weekday header, optional week numbers,
    /// and round day cells with hover, selection and "today" markers.
    ///
    /// Returns `true` if the user picked a day (and `popup_state` was updated).
    fn calendar_grid(
        &self,
        ui: &mut Ui,
        popup_state: &mut DatePickerPopupState,
        today: Date,
        weeks: &[Week],
    ) -> bool {
        let columns = if self.calendar_week { 8 } else { 7 };
        let first_day_column = columns - 7;

        let size = vec2(ui.available_width(), Self::calendar_height());
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let cell_width = rect.width() / columns as f32;
        let body_top = rect.top() + Self::CELL_HEIGHT + Self::HEADER_GAP;

        let cell_rect = |column: usize, top: f32| {
            Rect::from_min_size(
                pos2(rect.left() + cell_width * column as f32, top),
                vec2(cell_width, Self::CELL_HEIGHT),
            )
        };

        let visuals = ui.visuals();
        let painter = ui.painter();
        let body_font = TextStyle::Body.resolve(ui.style());
        let separator = visuals.widgets.noninteractive.bg_stroke;
        let weak_color = visuals.weak_text_color();
        let text_color = visuals.text_color();

        // Header: weekday names and a separator line below them.
        if self.calendar_week {
            painter.text(
                cell_rect(0, rect.top()).center(),
                Align2::CENTER_CENTER,
                "Week",
                body_font.clone(),
                text_color,
            );
        }
        //TODO(elwerene): Locale
        for (index, name) in ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
            .iter()
            .enumerate()
        {
            painter.text(
                cell_rect(first_day_column + index, rect.top()).center(),
                Align2::CENTER_CENTER,
                *name,
                body_font.clone(),
                text_color,
            );
        }
        painter.hline(
            rect.x_range(),
            rect.top() + Self::CELL_HEIGHT + Self::HEADER_GAP / 2.0,
            separator,
        );
        if self.calendar_week {
            painter.vline(
                rect.left() + cell_width,
                body_top..=body_top + weeks.len() as f32 * Self::CELL_HEIGHT,
                separator,
            );
        }

        // Body: one row per week.
        let mut picked = false;
        for (row, week) in weeks.iter().enumerate() {
            let top = body_top + row as f32 * Self::CELL_HEIGHT;

            if self.calendar_week {
                painter.text(
                    cell_rect(0, top).center(),
                    Align2::CENTER_CENTER,
                    week.number.to_string(),
                    body_font.clone(),
                    weak_color,
                );
            }

            for (index, day) in week.days.iter().enumerate() {
                let cell = cell_rect(first_day_column + index, top);
                let response = ui
                    .interact(cell, ui.scope_id().with(*day), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                let is_selected = popup_state.year == day.year()
                    && popup_state.month == day.month()
                    && popup_state.day == day.day();
                let is_in_month = day.month() == popup_state.month;
                let is_weekend = matches!(day.weekday(), Weekday::Saturday | Weekday::Sunday);

                let center = cell.center();
                let radius = cell.width().min(cell.height()) / 2.0 - 1.0;

                let text_color = if is_selected {
                    painter.circle_filled(center, radius, visuals.selection.bg_fill);
                    visuals.selection.stroke.color
                } else {
                    if response.hovered() {
                        painter.circle_filled(center, radius, visuals.widgets.hovered.weak_bg_fill);
                    }
                    if *day == today {
                        painter.circle_stroke(
                            center,
                            radius,
                            Stroke::new(1.0, visuals.selection.bg_fill),
                        );
                    }
                    let color = if self.highlight_weekends && is_weekend {
                        visuals.error_fg_color
                    } else {
                        visuals.text_color()
                    };
                    if is_in_month {
                        color
                    } else {
                        color.gamma_multiply(0.45)
                    }
                };

                painter.text(
                    center,
                    Align2::CENTER_CENTER,
                    day.day().to_string(),
                    body_font.clone(),
                    text_color,
                );

                if response.clicked() {
                    popup_state.year = day.year();
                    popup_state.month = day.month();
                    popup_state.day = day.day();
                    picked = true;
                }
            }
        }

        picked
    }

    /// Returns `true` if user pressed `Save` button.
    pub fn draw(&mut self, ui: &mut Ui) -> bool {
        let id = ui.make_persistent_id("date_picker");
        let today = jiff::Zoned::now().date();
        let mut popup_state = ui
            .data_mut(|data| data.get_persisted::<DatePickerPopupState>(id))
            .unwrap_or_default();
        // The date was edited by typing in the button: jump the calendar to it.
        let resync = ui.data_mut(|data| {
            core::mem::take(
                &mut data
                    .get_persisted_mut_or_default::<DatePickerButtonState>(self.button_id)
                    .resync,
            )
        });
        if resync {
            popup_state.setup = false;
        }
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
        let spacing = 2.0;
        ui.spacing_mut().item_spacing = Vec2::splat(spacing);

        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);

        StripBuilder::new(ui)
            .clip(false)
            .sizes(
                Size::exact(Self::CELL_HEIGHT),
                usize::from(self.combo_boxes || self.arrows),
            )
            .sizes(
                Size::exact(Self::calendar_height()),
                usize::from(self.calendar),
            )
            .vertical(|mut strip| {
                if self.combo_boxes || self.arrows {
                    strip.strip(|builder| {
                        self.header_row(builder, &mut popup_state, today, id);
                    });
                }

                if self.calendar {
                    strip.cell(|ui| {
                        if self.calendar_grid(ui, &mut popup_state, today, &weeks) {
                            ui.data_mut(|data| data.insert_persisted(id, popup_state.clone()));
                            // Picking a day applies it and closes the popup.
                            *self.selection =
                                Date::new(popup_state.year, popup_state.month, popup_state.day)
                                    .expect("Could not create Date");
                            saved = true;
                            close = true;
                        }
                    });
                }
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
