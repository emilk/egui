use super::{mask::DateMask, popup::DatePickerPopup};
use core::ops::RangeInclusive;
use egui::{
    Area, Atom, Button, Color32, Event, Frame, Id, IdSalt, InnerResponse, Key, Modifiers, Order,
    Rect, Stroke, StrokeKind, TextEdit, TextStyle, Ui, Vec2, Widget, pos2,
    text::{CCursor, CCursorRange},
    vec2,
};
use jiff::civil::Date;

#[derive(Default, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct DatePickerButtonState {
    pub picker_visible: bool,

    /// The date was changed by typing while the popup was open,
    /// so the popup should jump to the selected date.
    #[cfg_attr(feature = "serde", serde(default))]
    pub resync: bool,
}

/// What the always-visible part of the widget tells the popup logic.
struct Header {
    response: egui::Response,

    /// The user clicked inside the header, so that click must not close the popup.
    pressed: bool,
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
    editable: bool,
    format: String,
    highlight_weekends: bool,
    start_end_years: Option<RangeInclusive<i16>>,
    reverse_years: bool,
    year_scroll_to: Option<i16>,
}

impl<'a> DatePickerButton<'a> {
    /// Paints a small calendar icon filling `rect`.
    fn paint_calendar_icon(painter: &egui::Painter, rect: Rect, color: Color32) {
        let size = rect.width().min(rect.height());
        let rect = Rect::from_center_size(rect.center(), Vec2::splat(size));
        let stroke = Stroke::new((size * 0.09).max(1.0), color);

        let body = Rect::from_min_max(
            rect.min + vec2(size * 0.06, size * 0.16),
            rect.max - vec2(size * 0.06, size * 0.06),
        );
        painter.rect_stroke(body, size * 0.14, stroke, StrokeKind::Inside);

        // Header separator. Kept between the sides of the outline so it doesn't poke out.
        let header_y = body.top() + body.height() * 0.30;
        painter.hline(
            (body.left() + stroke.width * 0.5)..=(body.right() - stroke.width * 0.5),
            header_y,
            stroke,
        );

        // Binder rings.
        for t in [0.3, 0.7] {
            let x = body.left() + body.width() * t;
            painter.vline(
                x,
                (rect.top() + size * 0.04)..=(body.top() + size * 0.10),
                stroke,
            );
        }

        // Days.
        for row in 0..2 {
            for col in 0..3 {
                let x = body.left() + body.width() * (0.22 + 0.28 * col as f32);
                let y = header_y + body.height() * (0.20 + 0.25 * row as f32);
                painter.circle_filled(pos2(x, y), size * 0.055, color);
            }
        }
    }

    pub fn new(selection: &'a mut Date) -> Self {
        Self {
            selection,
            id_salt: None,
            combo_boxes: true,
            arrows: true,
            calendar: true,
            calendar_week: true,
            show_icon: true,
            editable: true,
            format: "%Y-%m-%d".to_owned(),
            highlight_weekends: true,
            start_end_years: None,
            reverse_years: false,
            year_scroll_to: None,
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

    /// Let the user type the date, with an input mask taken from the [format](Self::format).
    /// (Default: true)
    ///
    /// When `false` the date can't be changed: the text field is read-only and the
    /// calendar button is disabled.
    ///
    /// Only digits are accepted and separators are inserted automatically.
    /// The date is applied on Enter or when the field loses focus; if what was typed isn't a
    /// valid date, the previous date is restored. `Esc` cancels the edit, up/down change the
    /// part of the date under the cursor, and `Alt`+down opens the calendar.
    ///
    /// The mask supports formats made of `%Y`, `%y`, `%m`, `%d` and separators,
    /// like `%Y-%m-%d` or `%d/%m/%Y`. With any other format the text is not masked, but it
    /// still has to parse with that format.
    #[inline]
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
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
}

impl DatePickerButton<'_> {
    fn formatted(&self) -> String {
        self.selection.strftime(&self.format).to_string()
    }

    /// Parses typed text with the format. With a mask, the text has to be complete.
    fn parse(&self, text: &str, mask: Option<&DateMask>) -> Option<Date> {
        if let Some(mask) = mask
            && text.chars().filter(char::is_ascii_digit).count() != mask.digit_count()
        {
            return None;
        }
        Date::strptime(&self.format, text).ok()
    }

    /// Paints the calendar icon and, while the popup is open, the outline of the button.
    fn paint_button_decor(
        ui: &Ui,
        button_response: &egui::Response,
        icon_rect: Option<Rect>,
        picker_visible: bool,
    ) {
        let visuals = ui.visuals().widgets.open;
        if let Some(icon_rect) = icon_rect {
            let icon_color = if picker_visible {
                visuals.text_color()
            } else {
                ui.style().interact(button_response).text_color()
            };
            Self::paint_calendar_icon(ui.painter(), icon_rect, icon_color);
        }
        if picker_visible {
            ui.painter().rect_stroke(
                button_response.rect,
                visuals.corner_radius,
                visuals.bg_stroke,
                StrokeKind::Inside,
            );
        }
    }

    /// A text field for the date (masked, and read-only unless `editable`), followed by a button
    /// that opens the popup.
    fn header(&mut self, ui: &mut Ui, id: Id, button_state: &mut DatePickerButtonState) -> Header {
        let mask = DateMask::from_format(&self.format);
        let text_id = id.with("date_text");
        let has_focus = ui.memory(|mem| mem.has_focus(text_id));
        let escape_pressed = ui.input(|input| input.key_pressed(Key::Escape));
        let enter_pressed = ui.input(|input| input.key_pressed(Key::Enter));

        let mut changed = false;
        let mut open_popup = false;
        let mut close_popup = false;
        let mut still_editing = false;

        // While the field is being edited we keep what the user typed. That has to outlive the
        // focus: Tab and Esc move the focus away before the field is shown again, and the text
        // still needs to be applied (or dropped) then. Otherwise it shows the selected date.
        let mut buffer = ui
            .data_mut(|data| data.get_temp::<String>(text_id))
            .unwrap_or_else(|| self.formatted());

        if has_focus {
            // Only let digits and separators through, so letters never show up.
            if let Some(mask) = &mask {
                ui.input_mut(|input| {
                    input.events.retain_mut(|event| match event {
                        Event::Text(text) => {
                            text.retain(|c| mask.accepts(c));
                            !text.is_empty()
                        }
                        _ => true,
                    });
                });
            }

            if ui.input_mut(|input| input.consume_key(Modifiers::ALT, Key::ArrowDown)) {
                open_popup = true;
            } else if let Some(mask) = &mask {
                for (key, delta) in [(Key::ArrowUp, 1), (Key::ArrowDown, -1)] {
                    if !ui.input_mut(|input| input.consume_key(Modifiers::NONE, key)) {
                        continue;
                    }
                    let cursor = TextEdit::load_state(ui.ctx(), text_id)
                        .and_then(|state| state.cursor.char_range())
                        .map_or(0, |range| range.primary.index.into());
                    let date = self.parse(&buffer, Some(mask)).unwrap_or(*self.selection);
                    if let Some(field) = mask.field_at(cursor)
                        && let Ok(new_date) = date.checked_add(field.span(delta))
                    {
                        *self.selection = new_date;
                        buffer = self.formatted();
                        changed = true;
                    }
                }
            }
        }

        let sample = Date::constant(2026, 12, 28)
            .strftime(&self.format)
            .to_string();
        let font = TextStyle::Body.resolve(ui.style());
        let text_width = ui
            .painter()
            .layout_no_wrap(sample, font, Color32::WHITE)
            .size()
            .x;

        let visuals = ui.visuals().widgets.open;
        let icon_id = IdSalt::new("datepicker_calendar_icon");
        let picker_visible = button_state.picker_visible;
        let inner = ui.horizontal(|ui| {
            // The icon button sits right against the text field.
            ui.spacing_mut().item_spacing.x = 0.0;
            let output = TextEdit::singleline(&mut buffer)
                .id(text_id)
                .desired_width(text_width + 24.0)
                .interactive(self.editable)
                .show(ui);

            // Apply the mask to what was just typed or pasted, and put the cursor back.
            if output.response.changed()
                && let Some(mask) = &mask
            {
                let cursor = output.state.cursor.char_range().map_or_else(
                    || buffer.chars().count(),
                    |range| range.primary.index.into(),
                );
                let (text, new_cursor) = mask.sanitize(&buffer, cursor);
                if text != buffer {
                    buffer = text;
                    let mut state = output.state;
                    state
                        .cursor
                        .set_char_range(Some(CCursorRange::one(CCursor::new(new_cursor))));
                    state.store(ui.ctx(), output.response.id);
                    ui.ctx().request_repaint();
                }
            }

            still_editing = output.response.has_focus();

            // Enter or clicking elsewhere applies the date. Anything that isn't a valid date
            // goes back to the last valid one, and so does cancelling with Esc.
            if output.response.lost_focus() {
                if !escape_pressed
                    && let Some(date) = self.parse(&buffer, mask.as_ref())
                    && date != *self.selection
                {
                    *self.selection = date;
                    changed = true;
                }
                buffer = self.formatted();

                // Enter confirms, so it also closes the calendar.
                close_popup |= enter_pressed && !escape_pressed;
            }

            if self.show_icon {
                let icon_size = Vec2::splat(ui.text_style_height(&TextStyle::Button));
                let mut button = Button::new(Atom::custom(icon_id, icon_size));
                if picker_visible {
                    button = button.fill(visuals.weak_bg_fill);
                }
                // Not editable: the calendar can't be used to change the date either.
                let atom_response = ui
                    .add_enabled_ui(self.editable, |ui| button.atom_ui(ui))
                    .inner;
                let icon_rect = atom_response.rect(icon_id);
                Self::paint_button_decor(ui, &atom_response.response, icon_rect, picker_visible);
                if atom_response.response.clicked() {
                    open_popup = true;
                }
            }
        });
        let mut response = inner.response;

        ui.data_mut(|data| {
            if still_editing {
                data.insert_temp(text_id, buffer);
            } else {
                data.remove_temp::<String>(text_id);
            }
        });

        if changed {
            response.mark_changed();
            if button_state.picker_visible {
                ui.data_mut(|data| {
                    data.get_persisted_mut_or_default::<DatePickerButtonState>(id)
                        .resync = true;
                });
            }
        }
        if open_popup {
            button_state.picker_visible = true;
            ui.data_mut(|data| data.insert_persisted(id, button_state.clone()));
        } else if close_popup && button_state.picker_visible {
            button_state.picker_visible = false;
            // The calendar is closed from the outside, so make it start from the
            // selected date the next time it opens.
            button_state.resync = true;
            ui.data_mut(|data| data.insert_persisted(id, button_state.clone()));
        }

        // Clicks in the text field shouldn't close the popup, so that it can follow the typing.
        let pressed = open_popup
            || (response.contains_pointer() && ui.input(|input| input.pointer.any_click()));
        Header { response, pressed }
    }
}

impl Widget for DatePickerButton<'_> {
    fn ui(mut self, ui: &mut Ui) -> egui::Response {
        let id = ui.make_persistent_id(self.id_salt);
        let mut button_state = ui
            .data_mut(|data| data.get_persisted::<DatePickerButtonState>(id))
            .unwrap_or_default();

        let Header {
            response: mut button_response,
            pressed,
        } = self.header(ui, id, &mut button_state);

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
                                reverse_years: self.reverse_years,
                                year_scroll_to: self.year_scroll_to,
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
            let escape = ui.input(|i| i.key_pressed(Key::Escape));
            let enter = ui.input(|i| i.key_pressed(Key::Enter));
            let elsewhere = area_response.clicked_elsewhere();
            if !pressed && !any_popup_open && (escape || elsewhere || enter) {
                // The calendar is closed from the outside: start from the date next time.
                button_state.resync = true;
                button_state.picker_visible = false;
                ui.data_mut(|data| data.insert_persisted(id, button_state));
            }
        }

        button_response
    }
}
