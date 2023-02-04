use std::collections::VecDeque;

/// This struct tracks recent values of some time series.
///
/// It can be used as a smoothing filter for e.g. latency, fps etc,
/// or to show a log or graph of recent events.
///
/// It has a minimum and maximum length, as well as a maximum storage time.
/// * The minimum length is to ensure you have enough data for an estimate.
/// * The maximum length is to make sure the history doesn't take up too much space.
/// * The maximum age is to make sure the estimate isn't outdated.
///
/// Time difference between values can be zero, but never negative.
///
/// This can be used for things like smoothed averages (for e.g. FPS)
/// or for smoothed velocity (e.g. mouse pointer speed).
/// All times are in seconds.
#[derive(Clone, Debug)]
pub struct History<T> {
    /// In elements, i.e. of `values.len()`.
    /// The length is initially zero, but once past `min_len` will not shrink below it.
    min_len: usize,

    /// In elements, i.e. of `values.len()`.
    max_len: usize,

    /// In seconds.
    max_age: f32,

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
    /// Example:
    /// ```
    /// # use emath::History;
    /// # fn now() -> f64 { 0.0 }
    /// // Drop events that are older than one second,
    /// // as long we keep at least two events. Never keep more than a hundred events.
    /// let mut history = History::new(2..100, 1.0);
    /// assert_eq!(history.average(), None);
    /// history.add(now(), 40.0_f32);
    /// history.add(now(), 44.0_f32);
    /// assert_eq!(history.average(), Some(42.0));
    /// ```
    pub fn new(length_range: std::ops::Range<usize>, max_age: f32) -> Self {
        Self {
            min_len: length_range.start,
            max_len: length_range.end,
            max_age,
            total_count: 0,
            values: Default::default(),
        }
    }

    #[inline]
    pub fn max_len(&self) -> usize {
        self.max_len
    }

    #[inline]
    pub fn max_age(&self) -> f32 {
        self.max_age
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Current number of values kept in history
    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Total number of values seen.
    /// Includes those that have been discarded due to `max_len` or `max_age`.
    #[inline]
    pub fn total_count(&self) -> u64 {
        self.total_count
    }

    pub fn latest(&self) -> Option<T> {
        self.values.back().map(|(_, value)| *value)
    }

    pub fn latest_mut(&mut self) -> Option<&mut T> {
        self.values.back_mut().map(|(_, value)| value)
    }

    /// Amount of time contained from start to end in this [`History`].
    pub fn duration(&self) -> f32 {
        if let (Some(front), Some(back)) = (self.values.front(), self.values.back()) {
            (back.0 - front.0) as f32
        } else {
            0.0
        }
    }

    /// `(time, value)` pairs
    /// Time difference between values can be zero, but never negative.
    // TODO(emilk): impl IntoIter
    pub fn iter(&'_ self) -> impl ExactSizeIterator<Item = (f64, T)> + '_ {
        self.values.iter().map(|(time, value)| (*time, *value))
    }

    pub fn values(&'_ self) -> impl ExactSizeIterator<Item = T> + '_ {
        self.values.iter().map(|(_time, value)| *value)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// Values must be added with a monotonically increasing time, or at least not decreasing.
    pub fn add(&mut self, now: f64, value: T) {
        if let Some((last_time, _)) = self.values.back() {
            crate::emath_assert!(now >= *last_time, "Time shouldn't move backwards");
        }
        self.total_count += 1;
        self.values.push_back((now, value));
        self.flush(now);
    }

    /// Mean time difference between values in this [`History`].
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

    // Mean number of events per second.
    pub fn rate(&self) -> Option<f32> {
        self.mean_time_interval().map(|time| 1.0 / time)
    }

    /// Remove samples that are too old.
    pub fn flush(&mut self, now: f64) {
        while self.values.len() > self.max_len {
            self.values.pop_front();
        }
        while self.values.len() > self.min_len {
            if let Some((front_time, _)) = self.values.front() {
                if *front_time < now - (self.max_age as f64) {
                    self.values.pop_front();
                } else {
                    break;
                }
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
    #[inline]
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

impl<T> History<T>
where
    T: Copy,
    T: std::iter::Sum,
    T: std::ops::Div<f32, Output = T>,
    T: std::ops::Mul<f32, Output = T>,
{
    /// Average times rate.
    /// If you are keeping track of individual sizes of things (e.g. bytes),
    /// this will estimate the bandwidth (bytes per second).
    pub fn bandwidth(&self) -> Option<T> {
        Some(self.average()? * self.rate()?)
    }
}

impl<T, Vel> History<T>
where
    T: Copy,
    T: std::ops::Sub<Output = Vel>,
    Vel: std::ops::Div<f32, Output = Vel>,
{
    /// Calculate a smooth velocity (per second) over the entire time span.
    /// Calculated as the last value minus the first value over the elapsed time between them.
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
