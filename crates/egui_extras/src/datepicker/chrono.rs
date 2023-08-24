use chrono::{Datelike, Duration, NaiveDate, Weekday};

use super::{DateImpl, Week};

#[derive(Debug, PartialEq, Eq)]
pub struct Date(pub NaiveDate);

impl Date {
    pub fn month_data(year: i32, month: u8) -> Vec<Week<Self>> {
        let first =
            NaiveDate::from_ymd_opt(year, month as u32, 1).expect("Could not create NaiveDate");
        let mut start = first;
        while start.weekday() != Weekday::Mon {
            start = start.checked_sub_signed(Duration::days(1)).unwrap();
        }
        let mut weeks = vec![];
        let mut week = vec![];
        while start < first || start.month() == first.month() || start.weekday() != Weekday::Mon {
            week.push(start);

            if start.weekday() == Weekday::Sun {
                weeks.push(Week::new(
                    start.iso_week().week() as u8,
                    week.drain(..).map(Date).collect(),
                ));
            }
            start = start.checked_add_signed(Duration::days(1)).unwrap();
        }

        weeks
    }

    fn last_day_of_month(year: i32, month: u8) -> u8 {
        let date: NaiveDate =
            NaiveDate::from_ymd_opt(year, month as u32, 1).expect("Could not create NaiveDate");
        date.with_day(31)
            .map(|_| 31)
            .or_else(|| date.with_day(30).map(|_| 30))
            .or_else(|| date.with_day(29).map(|_| 29))
            .unwrap_or(28)
    }
}

impl DateImpl for Date {
    fn now() -> Self {
        Date(chrono::offset::Utc::now().date_naive())
    }

    fn from_ymd(year: i32, month: u8, day: u8) -> Self {
        Date(
            NaiveDate::from_ymd_opt(year, month as u32, day as u32)
                .expect("Could not create NaiveDate"),
        )
    }

    fn month_data(year: i32, month: u8) -> Vec<Week<Self>> {
        Date::month_data(year, month)
    }

    fn last_day_of_month(year: i32, month: u8) -> u8 {
        Date::last_day_of_month(year, month)
    }

    fn format(&self) -> String {
        format!("{}", self.0.format("%Y-%m-%d"))
    }

    fn year_month_day(&self) -> (i32, u8, u8) {
        (self.0.year(), self.0.month() as u8, self.0.day() as u8)
    }

    fn is_weekend(&self) -> bool {
        self.0.weekday() == Weekday::Sat || self.0.weekday() == Weekday::Sun
    }
}
