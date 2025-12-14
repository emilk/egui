use super::popup::DatePickerPopup;
use chrono::NaiveDate;
use egui::{Area, Button, Frame, InnerResponse, Key, Order, RichText, Ui, Widget};
use std::ops::RangeInclusive;

#[derive(Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct DatePickerButtonState {
    pub picker_visible: bool,
}

/// Shows a date, and will open a date picker popup when clicked.
pub struct DatePickerButton<'a> {
    selection: &'a mut NaiveDate,
    id_salt: Option<&'a str>,
    combo_boxes: bool,
    arrows: bool,
    calendar: bool,
    calendar_week: bool,
    show_icon: bool,
    format: String,
    highlight_weekends: bool,
    start_end_years: Option<RangeInclusive<i32>>,
}

impl<'a> DatePickerButton<'a> {
    pub fn new(selection: &'a mut NaiveDate) -> Self {
        Self {
            selection,
            id_salt: None,
            combo_boxes: true,
            arrows: true,
            calendar: true,
            calendar_week: true,
            show_icon: true,
            format: "%Y-%m-%d".to_owned(),
            highlight_weekends: true,
            start_end_years: None,
        }
    }

    /// Add id source.
    /// Must be set if multiple date picker buttons are in the same Ui.
    #[inline]
    pub fn id_salt(mut self, id_salt: &'a str) -> Self {
        self.id_salt = Some(id_salt);
        self
    }

    /// Add id source.
    /// Must be set if multiple date picker buttons are in the same Ui.
    #[inline]
    #[deprecated = "Renamed id_salt"]
    pub fn id_source(self, id_salt: &'a str) -> Self {
        self.id_salt(id_salt)
    }

    /// Show combo boxes in date picker popup. (Default: true)
    #[inline]
    pub fn combo_boxes(mut self, combo_boxes: bool) -> Self {
        self.combo_boxes = combo_boxes;
        self
    }

    /// Show arrows in date picker popup. (Default: true)
    #[inline]
    pub fn arrows(mut self, arrows: bool) -> Self {
        self.arrows = arrows;
        self
    }

    /// Show calendar in date picker popup. (Default: true)
    #[inline]
    pub fn calendar(mut self, calendar: bool) -> Self {
        self.calendar = calendar;
        self
    }

    /// Show calendar week in date picker popup. (Default: true)
    #[inline]
    pub fn calendar_week(mut self, week: bool) -> Self {
        self.calendar_week = week;
        self
    }

    /// Show the calendar icon on the button. (Default: true)
    #[inline]
    pub fn show_icon(mut self, show_icon: bool) -> Self {
        self.show_icon = show_icon;
        self
    }

    /// Change the format shown on the button. (Default: %Y-%m-%d)
    /// See [`chrono::format::strftime`] for valid formats.
    #[inline]
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = format.into();
        self
    }

    /// Highlight weekend days. (Default: true)
    #[inline]
    pub fn highlight_weekends(mut self, highlight_weekends: bool) -> Self {
        self.highlight_weekends = highlight_weekends;
        self
    }

    /// Set the start and end years for the date picker. (Default: today's year - 100 to today's year + 10)
    /// This will limit the years you can choose from in the dropdown to the specified range.
    ///
    /// For example, if you want to provide the range of years from 2000 to 2035, you can use:
    /// `start_end_years(2000..=2035)`.
    #[inline]
    pub fn start_end_years(mut self, start_end_years: RangeInclusive<i32>) -> Self {
        self.start_end_years = Some(start_end_years);
        self
    }
}

impl Widget for DatePickerButton<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let id = ui.make_persistent_id(self.id_salt);
        let mut button_state = ui
            .data_mut(|data| data.get_persisted::<DatePickerButtonState>(id))
            .unwrap_or_default();

        let mut text = if self.show_icon {
            RichText::new(format!("{} 📆", self.selection.format(&self.format)))
        } else {
            RichText::new(format!("{}", self.selection.format(&self.format)))
        };
        let visuals = ui.visuals().widgets.open;
        if button_state.picker_visible {
            text = text.color(visuals.text_color());
        }
        let mut button = Button::new(text);
        if button_state.picker_visible {
            button = button.fill(visuals.weak_bg_fill).stroke(visuals.bg_stroke);
        }
        let mut button_response = ui.add(button);
        if button_response.clicked() {
            button_state.picker_visible = true;
            ui.data_mut(|data| data.insert_persisted(id, button_state.clone()));
        }

        if button_state.picker_visible {
            let width = 333.0;
            let mut pos = button_response.rect.left_bottom();
            let width_with_padding = width
                + ui.style().spacing.item_spacing.x
                + ui.style().spacing.window_margin.leftf()
                + ui.style().spacing.window_margin.rightf();
            if pos.x + width_with_padding > ui.clip_rect().right() {
                pos.x = button_response.rect.right() - width_with_padding;
            }

            // Check to make sure the calendar never is displayed out of window
            pos.x = pos.x.max(ui.style().spacing.window_margin.leftf());

            //TODO(elwerene): Better positioning

            let InnerResponse {
                inner: saved,
                response: area_response,
            } = Area::new(ui.make_persistent_id(self.id_salt))
                .kind(egui::UiKind::Picker)
                .order(Order::Foreground)
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
                    let frame = Frame::popup(ui.style());
                    frame
                        .show(ui, |ui| {
                            ui.set_min_width(width);
                            ui.set_max_width(width);

                            DatePickerPopup {
                                selection: self.selection,
                                button_id: id,
                                combo_boxes: self.combo_boxes,
                                arrows: self.arrows,
                                calendar: self.calendar,
                                calendar_week: self.calendar_week,
                                highlight_weekends: self.highlight_weekends,
                                start_end_years: self.start_end_years,
                            }
                            .draw(ui)
                        })
                        .inner
                });

            if saved {
                button_response.mark_changed();
            }

            // We don't want to close our popup if any other popup is open, since other popups would
            // most likely be the combo boxes in the date picker.
            let any_popup_open = ui.any_popup_open();
            if !button_response.clicked()
                && !any_popup_open
                && (ui.input(|i| i.key_pressed(Key::Escape)) || area_response.clicked_elsewhere())
            {
                button_state.picker_visible = false;
                ui.data_mut(|data| data.insert_persisted(id, button_state));
            }
        }

        button_response
    }
}
