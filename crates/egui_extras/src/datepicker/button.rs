use super::DateLocale;
use super::popup::DatePickerPopup;
use core::ops::RangeInclusive;
use egui::{Area, Button, Frame, InnerResponse, Key, Order, RichText, Ui, Widget};
use jiff::civil::Date;

#[derive(Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct DatePickerButtonState {
    pub picker_visible: bool,
}

/// Shows a date, and will open a date picker popup when clicked.
pub struct DatePickerButton<'a> {
    selection: &'a mut Date,
    id_salt: Option<&'a str>,
    combo_boxes: bool,
    arrows: bool,
    calendar: bool,
    calendar_week: bool,
    show_icon: bool,
    format: String,
    highlight_weekends: bool,
    start_end_years: Option<RangeInclusive<i16>>,
    reverse_years: bool,
    year_scroll_to: Option<i16>,
    locale: Option<DateLocale>,
}

impl<'a> DatePickerButton<'a> {
    pub fn new(selection: &'a mut Date) -> Self {
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
            reverse_years: false,
            year_scroll_to: None,
            locale: None,
        }
    }

    /// Add id source.
    /// Must be set if multiple date picker buttons are in the same Ui.
    #[inline]
    pub fn id_salt(mut self, id_salt: &'a str) -> Self {
        self.id_salt = Some(id_salt);
        self
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
    /// See [`jiff::fmt::strtime`] for valid formats.
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
    pub fn start_end_years(mut self, start_end_years: RangeInclusive<i16>) -> Self {
        self.start_end_years = Some(start_end_years);
        self
    }

    /// List years in descending order in the year dropdown. (Default: false)
    #[inline]
    pub fn reverse_years(mut self, reverse_years: bool) -> Self {
        self.reverse_years = reverse_years;
        self
    }

    /// Scroll the year dropdown to this year when the picker first opens.
    /// Defaults to the currently selected year.
    #[inline]
    pub fn year_scroll_to(mut self, year: i16) -> Self {
        self.year_scroll_to = Some(year);
        self
    }

    /// Set the locale used by the date picker popup: weekday and month names,
    /// the first day of the week, the weekend days, the "Week" label and the
    /// reading direction.
    ///
    /// When not set, the locale is resolved from the UI language
    /// ([`Ui::language`], e.g. `"ar-EG"` → Arabic); unset or unknown languages
    /// fall back to [`DateLocale::default`] (English).
    ///
    /// Fork-side only: [`Ui::language`] does not exist upstream, where the
    /// field is a plain `DateLocale` defaulting to [`DateLocale::default`].
    #[inline]
    pub fn locale(mut self, locale: DateLocale) -> Self {
        self.locale = Some(locale);
        self
    }
}

impl Widget for DatePickerButton<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let id = ui.make_persistent_id(self.id_salt);
        let mut button_state = ui
            .data_mut(|data| data.get_persisted::<DatePickerButtonState>(id))
            .unwrap_or_default();

        // Locale resolution chain (fork-side; uses `Ui::language`, which does
        // not exist upstream): an explicit `.locale(...)` wins, else resolve
        // from the UI language, else the English default.
        // `DateLocale::from_language` matches on the primary BCP 47 subtag and
        // falls back to `DateLocale::default()` for unknown codes.
        let locale = self.locale.clone().unwrap_or_else(|| {
            ui.language()
                .map(|code| DateLocale::from_language(&code))
                .unwrap_or_default()
        });

        let mut text = if self.show_icon {
            if locale.rtl {
                // Under RTL the icon leads so the glyph stays at the visual
                // reading start. Superseded by epaint bidi support
                // (https://github.com/emilk/egui/pull/8818 and
                // https://github.com/emilk/egui/pull/8822) once that lands.
                RichText::new(format!("📆 {}", self.selection.strftime(&self.format)))
            } else {
                RichText::new(format!("{} 📆", self.selection.strftime(&self.format)))
            }
        } else {
            RichText::new(format!("{}", self.selection.strftime(&self.format)))
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
            let margin = ui.style().spacing.window_margin;
            let item_spacing = ui.style().spacing.item_spacing.x;
            let mut pos = if locale.rtl {
                // RTL: the popup hangs off the button's right (reading-start)
                // edge and extends toward the reading end — the mirror of the
                // LTR `left_bottom()` anchor below.
                button_response.rect.right_bottom() - egui::vec2(width, 0.0)
            } else {
                button_response.rect.left_bottom()
            };
            let width_with_padding = width + item_spacing + margin.leftf() + margin.rightf();
            if locale.rtl {
                if pos.x - item_spacing - margin.leftf() - margin.rightf() < ui.clip_rect().left() {
                    // Left-edge overflow: flip to the button's other side, the
                    // mirror of the LTR flip below.
                    pos.x = button_response.rect.left()
                        + item_spacing
                        + margin.leftf()
                        + margin.rightf();
                }
                pos.x = pos
                    .x
                    .min(ui.clip_rect().right() - margin.rightf() - width)
                    .max(margin.leftf());
            } else {
                if pos.x + width_with_padding > ui.clip_rect().right() {
                    pos.x = button_response.rect.right() - width_with_padding;
                }

                pos.x = pos.x.max(margin.leftf());
            }

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
                                reverse_years: self.reverse_years,
                                year_scroll_to: self.year_scroll_to,
                                locale,
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
