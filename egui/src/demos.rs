//! Demo-code for showing how Egui is used.
//!
//! The demo-code is also used in benchmarks and tests.
mod app;
mod fractal_clock;
pub mod toggle_switch;

pub use {
    app::{DemoApp, DemoWindow},
    fractal_clock::FractalClock,
};
