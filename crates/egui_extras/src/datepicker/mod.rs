#![expect(clippy::unwrap_used)] // TODO(emilk): avoid unwraps

mod button;
mod popup;

pub use button::DatePickerButton;
use jiff::civil::{Date, ISOWeekDate, Weekday};

#[derive(Debug)]
struct Week {
    number: u8,
    days: Vec<Date>,
}

fn month_data(year: i16, month: i8) -> Vec<Week> {
    let first = Date::new(year, month, 1).expect("Could not create Date");
    let mut start = first;
    while start.weekday() != Weekday::Monday {
        start = start.yesterday().unwrap();
    }
    let mut weeks = vec![];
    let mut week = vec![];
    while start < first || start.month() == first.month() || start.weekday() != Weekday::Monday {
        week.push(start);

        if start.weekday() == Weekday::Sunday {
            weeks.push(Week {
                number: ISOWeekDate::from(start).week() as u8,
                days: std::mem::take(&mut week),
            });
        }
        start = start.tomorrow().unwrap();
    }

    weeks
}
