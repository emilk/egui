#![expect(clippy::unwrap_used)] // TODO(emilk): avoid unwraps

mod button;
mod popup;

pub use button::DatePickerButton;
use jiff::civil::{Date, ISOWeekDate, Weekday};

/// Locale data for the [`DatePickerButton`] popup: weekday and month names, the
/// first day of the week, the weekend days, the week-number column label, and
/// the reading direction.
///
/// [`DateLocale::default`] preserves the historical behavior: English names,
/// weeks starting on Monday, a Saturday+Sunday weekend, an English `"Week"`
/// label, and left-to-right layout.
///
/// Pass a custom locale to the button with [`DatePickerButton::locale`], or
/// resolve one from a BCP 47 language tag with [`DateLocale::from_language`].
///
/// Adding fields later is non-breaking for users who construct locales through
/// [`DateLocale::default`] / [`DateLocale::from_language`] and override fields,
/// but changes the [`DateLocale::new`] signature; prefer updating fields on
/// `DateLocale::default()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateLocale {
    /// Names of the weekdays, indexed Monday-first
    /// (`Weekday::to_monday_zero_offset`): `weekday_names[0]` is Monday.
    pub weekday_names: [String; 7],
    /// Names of the months; `month_names[0]` is January.
    pub month_names: [String; 12],
    /// The first day of the week: the first (reading-start) column of the
    /// month grid.
    pub first_weekday: Weekday,
    /// The weekend days, highlighted when `DatePickerButton::highlight_weekends`
    /// is on. Two days for now; a slice is the escape hatch if a locale ever
    /// needs a 1- or 3-day weekend.
    pub weekend: [Weekday; 2],
    /// Label of the week-number column. Week numbers stay ISO-numbered
    /// regardless of `first_weekday`, matching common calendar apps.
    pub week_label: String,
    /// Whether the locale reads right-to-left. Mirrors the popup anchoring and
    /// the button icon order. This is a stopgap scoped to the date picker for
    /// [`egui::Style::right_to_left`]
    /// (<https://github.com/emilk/egui/pull/8815>); once that lands, a follow-up
    /// reads the style flag instead.
    pub rtl: bool,
}

impl Default for DateLocale {
    fn default() -> Self {
        Self {
            weekday_names: ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"].map(str::to_owned),
            month_names: [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December",
            ]
            .map(str::to_owned),
            first_weekday: Weekday::Monday,
            weekend: [Weekday::Saturday, Weekday::Sunday],
            week_label: "Week".to_owned(),
            rtl: false,
        }
    }
}

impl DateLocale {
    /// Create a locale from explicit data. Prefer starting from
    /// [`DateLocale::default`] and overriding fields; see also
    /// [`DateLocale::from_language`].
    pub fn new(
        weekday_names: [String; 7],
        month_names: [String; 12],
        first_weekday: Weekday,
        weekend: [Weekday; 2],
        week_label: String,
        rtl: bool,
    ) -> Self {
        Self {
            weekday_names,
            month_names,
            first_weekday,
            weekend,
            week_label,
            rtl,
        }
    }

    /// Resolve a locale from a BCP 47 language tag, matching on the primary
    /// language subtag (so `"ar-EG"` matches `"ar"`). Unknown or empty tags
    /// fall back to [`DateLocale::default`] (English).
    ///
    /// The built-in table is intentionally minimal; this function is the
    /// extension point for richer, data-driven locale resolution (e.g. an
    /// ICU4X/CLDR-backed table, or defaulting from `Ui::language()` once that
    /// exists upstream).
    pub fn from_language(code: &str) -> Self {
        match code.split('-').next() {
            Some("ar") => Self {
                month_names: [
                    "يناير",
                    "فبراير",
                    "مارس",
                    "أبريل",
                    "مايو",
                    "يونيو",
                    "يوليو",
                    "أغسطس",
                    "سبتمبر",
                    "أكتوبر",
                    "نوفمبر",
                    "ديسمبر",
                ]
                .map(str::to_owned),
                weekday_names: [
                    "الاثنين",
                    "الثلاثاء",
                    "الأربعاء",
                    "الخميس",
                    "الجمعة",
                    "السبت",
                    "الأحد",
                ]
                .map(str::to_owned),
                first_weekday: Weekday::Saturday,
                weekend: [Weekday::Friday, Weekday::Saturday],
                week_label: "الأسبوع".to_owned(),
                rtl: true,
            },
            _ => Self::default(),
        }
    }
}

#[derive(Debug)]
struct Week {
    number: u8,
    days: Vec<Date>,
}

fn month_data(year: i16, month: i8, first_weekday: Weekday) -> Vec<Week> {
    let first = Date::new(year, month, 1).expect("Could not create Date");
    let mut start = first;
    while start.weekday() != first_weekday {
        start = start.yesterday().unwrap();
    }
    let last_weekday = first_weekday.previous();
    let mut weeks = vec![];
    let mut week = vec![];
    while start < first || start.month() == first.month() || start.weekday() != first_weekday {
        week.push(start);

        if start.weekday() == last_weekday {
            weeks.push(Week {
                number: ISOWeekDate::from(start).week() as u8,
                days: core::mem::take(&mut week),
            });
        }
        start = start.tomorrow().unwrap();
    }

    weeks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_locale_matches_the_previous_hardcoded_behavior() {
        let locale = DateLocale::default();
        assert_eq!(
            locale
                .weekday_names
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
        );
        assert_eq!(
            locale
                .month_names
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December"
            ]
        );
        assert_eq!(locale.first_weekday, Weekday::Monday);
        assert_eq!(locale.weekend, [Weekday::Saturday, Weekday::Sunday]);
        assert_eq!(locale.week_label, "Week");
        assert!(!locale.rtl);
    }

    #[test]
    fn constructed_locale_round_trips_its_fields() {
        let locale = DateLocale {
            weekday_names: [
                "الاثنين",
                "الثلاثاء",
                "الأربعاء",
                "الخميس",
                "الجمعة",
                "السبت",
                "الأحد",
            ]
            .map(str::to_owned),
            month_names: [
                "يناير",
                "فبراير",
                "مارس",
                "أبريل",
                "مايو",
                "يونيو",
                "يوليو",
                "أغسطس",
                "سبتمبر",
                "أكتوبر",
                "نوفمبر",
                "ديسمبر",
            ]
            .map(str::to_owned),
            first_weekday: Weekday::Saturday,
            weekend: [Weekday::Friday, Weekday::Saturday],
            week_label: "الأسبوع".to_owned(),
            rtl: true,
        };
        assert_eq!(locale.first_weekday, Weekday::Saturday);
        assert_eq!(locale.weekend, [Weekday::Friday, Weekday::Saturday]);
        assert!(locale.rtl);
        assert_eq!(locale.weekday_names[6], "الأحد");
        assert_eq!(locale.month_names[10], "نوفمبر");
    }

    #[test]
    fn month_data_monday_start_matches_the_previous_behavior() {
        // 2026-11-01 is a Sunday: the first Monday-anchored row starts 2026-10-26.
        let weeks = month_data(2026, 11, Weekday::Monday);
        assert_eq!(weeks.len(), 6);
        assert_eq!(weeks[0].days[0], Date::constant(2026, 10, 26));
        assert_eq!(weeks[0].days[0].weekday(), Weekday::Monday);
        assert_eq!(weeks[5].days[6], Date::constant(2026, 12, 6));
    }

    #[test]
    fn month_data_honors_the_first_weekday() {
        let sunday = month_data(2026, 11, Weekday::Sunday);
        assert_eq!(sunday[0].days[0], Date::constant(2026, 11, 1));
        assert_eq!(sunday[0].days[0].weekday(), Weekday::Sunday);

        let saturday = month_data(2026, 11, Weekday::Saturday);
        assert_eq!(saturday[0].days[0], Date::constant(2026, 10, 31));
        assert_eq!(saturday[0].days[0].weekday(), Weekday::Saturday);
        assert_eq!(saturday[0].days[6].weekday(), Weekday::Friday);
    }

    #[test]
    fn month_data_weeks_tile_without_gaps() {
        for first_weekday in [
            Weekday::Monday,
            Weekday::Sunday,
            Weekday::Saturday,
            Weekday::Friday,
        ] {
            let weeks = month_data(2026, 11, first_weekday);
            let days: Vec<Date> = weeks
                .iter()
                .flat_map(|week| week.days.iter().copied())
                .collect();
            assert_eq!(days.len() % 7, 0, "every row is a full week");
            for pair in days.windows(2) {
                assert_eq!(pair[1], pair[0].tomorrow().unwrap(), "days are consecutive");
            }
            assert!(
                days.iter().any(|day| day.month() == 11),
                "the month is covered"
            );
        }
    }
}
