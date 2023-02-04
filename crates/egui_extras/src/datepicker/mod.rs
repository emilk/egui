mod button;
mod popup;

pub use button::DatePickerButton;
use chrono::{Datelike, Duration, NaiveDate, Weekday};

#[derive(Debug)]
struct Week {
    number: u8,
    days: Vec<NaiveDate>,
}

fn month_data(year: i32, month: u32) -> Vec<Week> {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("Could not create NaiveDate");
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
