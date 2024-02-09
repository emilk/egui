#![allow(dead_code)] // not everything is used on wasm

use web_time::Instant;

pub struct Stopwatch {
    total_time_ns: u128,

    /// None = not running
    start: Option<Instant>,
}

impl Stopwatch {
    pub fn new() -> Self {
        Self {
            total_time_ns: 0,
            start: None,
        }
    }

    pub fn start(&mut self) {
        assert!(self.start.is_none());
        self.start = Some(Instant::now());
    }

    pub fn pause(&mut self) {
        let start = self.start.take().unwrap();
        let duration = start.elapsed();
        self.total_time_ns += duration.as_nanos();
    }

    pub fn resume(&mut self) {
        assert!(self.start.is_none());
        self.start = Some(Instant::now());
    }

    pub fn total_time_ns(&self) -> u128 {
        if let Some(start) = self.start {
            // Running
            let duration = start.elapsed();
            self.total_time_ns + duration.as_nanos()
        } else {
            // Paused
            self.total_time_ns
        }
    }

    pub fn total_time_sec(&self) -> f32 {
        self.total_time_ns() as f32 * 1e-9
    }
}
