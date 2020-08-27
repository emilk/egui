use std::collections::VecDeque;

/// This struct tracks recent values of some time series.
/// This can be used for things like smoothed averages (for e.g. FPS)
/// or for smoothed velocity (e.g. mouse pointer speed).
/// All times are in seconds.
#[derive(Clone, Debug)]
pub struct MovementTracker<T> {
    max_len: usize,
    max_age: f64,

    /// (time, value) pais
    values: VecDeque<(f64, T)>,
}

impl<T> MovementTracker<T>
where
    T: Copy,
{
    pub fn new(max_len: usize, max_age: f64) -> Self {
        Self {
            max_len,
            max_age,
            values: Default::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Amount of time contained from start to end in this `MovementTracker`
    pub fn dt(&self) -> f32 {
        if let (Some(front), Some(back)) = (self.values.front(), self.values.back()) {
            (back.0 - front.0) as f32
        } else {
            0.0
        }
    }

    pub fn values<'a>(&'a self) -> impl Iterator<Item = T> + 'a {
        self.values.iter().map(|(_time, value)| *value)
    }

    pub fn clear(&mut self) {
        self.values.clear()
    }

    /// Values must be added with a monotonically increasing time, or at least not decreasing.
    pub fn add(&mut self, now: f64, value: T) {
        if let Some((last_time, _)) = self.values.back() {
            debug_assert!(now >= *last_time, "Time shouldn't go backwards");
        }
        self.values.push_back((now, value));
        self.flush(now);
    }

    /// Mean time difference between values in this `MovementTracker`.
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

impl<T> MovementTracker<T>
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

impl<T, Vel> MovementTracker<T>
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
