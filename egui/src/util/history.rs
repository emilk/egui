use std::collections::VecDeque;

/// This struct tracks recent values of some time series.
///
/// One use is to show a log of recent events,
/// or show a graph over recent events.
///
/// It has both a maximum length and a maximum storage time.
/// Elements are dropped when either max length or max age is reached.
///
/// Time difference between values can be zero, but never negative.
///
/// This can be used for things like smoothed averages (for e.g. FPS)
/// or for smoothed velocity (e.g. mouse pointer speed).
/// All times are in seconds.
#[derive(Clone, Debug)]
pub struct History<T> {
    /// In elements, i.e. of `values.len()`
    max_len: usize,

    /// In seconds
    max_age: f64, // TODO: f32

    /// Total number of elements seen ever
    total_count: u64,

    /// (time, value) pairs, oldest front, newest back.
    /// Time difference between values can be zero, but never negative.
    values: VecDeque<(f64, T)>,
}

impl<T> History<T>
where
    T: Copy,
{
    pub fn new(max_len: usize, max_age: f64) -> Self {
        Self::from_max_len_age(max_len, max_age)
    }

    pub fn from_max_len_age(max_len: usize, max_age: f64) -> Self {
        Self {
            max_len,
            max_age,
            total_count: 0,
            values: Default::default(),
        }
    }

    pub fn max_len(&self) -> usize {
        self.max_len
    }

    pub fn max_age(&self) -> f32 {
        self.max_age as f32
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Current number of values kept in history
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Total number of values seen.
    /// Includes those that have been discarded due to `max_len` or `max_age`.
    pub fn total_count(&self) -> u64 {
        self.total_count
    }

    pub fn latest(&self) -> Option<T> {
        self.values.back().map(|(_, value)| *value)
    }

    pub fn latest_mut(&mut self) -> Option<&mut T> {
        self.values.back_mut().map(|(_, value)| value)
    }

    /// Amount of time contained from start to end in this `History`.
    pub fn duration(&self) -> f32 {
        if let (Some(front), Some(back)) = (self.values.front(), self.values.back()) {
            (back.0 - front.0) as f32
        } else {
            0.0
        }
    }

    /// `(time, value)` pairs
    /// Time difference between values can be zero, but never negative.
    // TODO: impl IntoIter
    pub fn iter(&'_ self) -> impl Iterator<Item = (f64, T)> + '_ {
        self.values.iter().map(|(time, value)| (*time, *value))
    }

    pub fn values(&'_ self) -> impl Iterator<Item = T> + '_ {
        self.values.iter().map(|(_time, value)| *value)
    }

    pub fn clear(&mut self) {
        self.values.clear()
    }

    /// Values must be added with a monotonically increasing time, or at least not decreasing.
    pub fn add(&mut self, now: f64, value: T) {
        if let Some((last_time, _)) = self.values.back() {
            crate::egui_assert!(now >= *last_time, "Time shouldn't move backwards");
        }
        self.total_count += 1;
        self.values.push_back((now, value));
        self.flush(now);
    }

    /// Mean time difference between values in this `History`.
    pub fn mean_time_interval(&self) -> Option<f32> {
        if let (Some(first), Some(last)) = (self.values.front(), self.values.back()) {
            let n = self.len();
            if n >= 2 {
                Some((last.0 - first.0) as f32 / ((n - 1) as f32))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Remove samples that are too old
    pub fn flush(&mut self, now: f64) {
        while self.values.len() > self.max_len {
            self.values.pop_front();
        }
        while let Some((front_time, _)) = self.values.front() {
            if *front_time < now - self.max_age {
                self.values.pop_front();
            } else {
                break;
            }
        }
    }
}

impl<T> History<T>
where
    T: Copy,
    T: std::iter::Sum,
    T: std::ops::Div<f32, Output = T>,
{
    pub fn sum(&self) -> T {
        self.values().sum()
    }

    pub fn average(&self) -> Option<T> {
        let num = self.len();
        if num > 0 {
            Some(self.sum() / (num as f32))
        } else {
            None
        }
    }
}

impl<T, Vel> History<T>
where
    T: Copy,
    T: std::ops::Sub<Output = Vel>,
    Vel: std::ops::Div<f32, Output = Vel>,
{
    /// Calculate a smooth velocity (per second) over the entire time span
    pub fn velocity(&self) -> Option<Vel> {
        if let (Some(first), Some(last)) = (self.values.front(), self.values.back()) {
            let dt = (last.0 - first.0) as f32;
            if dt > 0.0 {
                Some((last.1 - first.1) / dt)
            } else {
                None
            }
        } else {
            None
        }
    }
}
