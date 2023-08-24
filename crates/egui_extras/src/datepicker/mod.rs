mod button;
#[cfg(feature = "chrono")]
pub mod chrono;
#[cfg(feature = "hifitime")]
pub mod hifitime;
mod popup;

pub use button::DatePickerButton;

#[cfg(feature = "chrono")]
pub type DatePickerButtonChrono<'a> = DatePickerButton<'a, self::chrono::Date>;
#[cfg(feature = "hifitime")]
pub type DatePickerButtonHifi<'a> = DatePickerButton<'a, self::hifitime::Date>;

#[derive(Debug)]
pub struct Week<T> {
    number: u8,
    days: Vec<T>,
}

impl<T> Week<T> {
    pub fn new(number: u8, days: Vec<T>) -> Self {
        Self { number, days }
    }
}

pub trait DateImpl: Eq + Sized {
    fn now() -> Self;
    fn from_ymd(year: i32, month: u8, day: u8) -> Self;
    fn month_data(year: i32, month: u8) -> Vec<Week<Self>>;
    fn last_day_of_month(year: i32, month: u8) -> u8;
    fn format(&self) -> String;
    fn year_month_day(&self) -> (i32, u8, u8);
    fn is_weekend(&self) -> bool;
}
