use super::{DateImpl, Week};
use hifitime::{prelude::*, Epoch, TimeSeries, Weekday};
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq)]

pub struct Date(pub Epoch);

impl Date {
    pub fn last_day_of_month(year: i32, month: u8) -> u8 {
        let date = Epoch::from_gregorian_utc_at_midnight(year, month, 1);
        for nb in (27u8..32).rev() {
            let end = date + (nb as i64).days();
            if date.month_name() == end.month_name() {
                return nb;
            }
        }
        unreachable!()
    }

    /// get the iso week number of a given date
    fn iso_week_of(date: Epoch) -> u8 {
        // iso week 1 starts the week of the 4th january
        // see: https://en.wikipedia.org/wiki/ISO_week_date
        (if date.weekday() != Weekday::Sunday {
            date.next_weekday_at_midnight(Weekday::Sunday)
        } else {
            date
        }
        .previous_weekday_at_midnight(Weekday::Thursday)
        .day_of_year()
            / 7.) as u8
            + 1
    }

    fn month_data(year: i32, month: u8) -> Vec<Week<Self>> {
        let first = Epoch::from_gregorian_utc_at_midnight(year, month, 1);
        let start = if first.weekday_utc() != Weekday::Monday {
            first.previous_weekday_at_midnight(Weekday::Monday)
        } else {
            first
        };
        let end: Epoch = first + (Self::last_day_of_month(year, month) as i64).days();
        let end = if end.weekday_utc() != Weekday::Sunday {
            end.next_weekday_at_midnight(Weekday::Sunday)
        } else {
            end
        };
        let mut weeks = vec![];
        let mut week = vec![];
        TimeSeries::inclusive(start, end, 1.days());
        for day in TimeSeries::inclusive(start, end, 1.days()) {
            week.push(day);
            if day.weekday_utc() == Weekday::Sunday {
                weeks.push(Week::new(
                    Date::iso_week_of(day),
                    week.drain(..).map(Date).collect(),
                ));
            }
        }
        weeks
    }
}

impl DateImpl for Date {
    fn now() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Date(Epoch::from_unix_seconds(
                instant::SystemTime::now()
                    .duration_since(instant::SystemTime::UNIX_EPOCH)
                    .expect("")
                    .as_secs_f64(),
            ))
        }
        #[cfg(not(target_arch = "wasm32"))]
        Date(Epoch::now().unwrap())
    }

    fn from_ymd(year: i32, month: u8, day: u8) -> Self {
        Date(Epoch::from_gregorian_utc_at_midnight(year, month, day))
    }

    fn month_data(year: i32, month: u8) -> Vec<Week<Self>> {
        Date::month_data(year, month)
    }

    fn last_day_of_month(year: i32, month: u8) -> u8 {
        Date::last_day_of_month(year, month)
    }

    fn format(&self) -> String {
        format!(
            "{}",
            Formatter::new(self.0, Format::from_str("%Y-%m-%d").unwrap())
        )
    }

    fn year_month_day(&self) -> (i32, u8, u8) {
        let (y, m, d, ..) = self.0.to_gregorian_utc();
        (y, m, d)
    }

    fn is_weekend(&self) -> bool {
        self.0.weekday() == Weekday::Saturday || self.0.weekday() == Weekday::Sunday
    }
}
