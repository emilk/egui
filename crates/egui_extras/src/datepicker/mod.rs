mod button;
mod popup;

pub use button::DatePickerButton;
use chrono::{Date, Datelike, Duration, NaiveDate, Utc, Weekday};

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
