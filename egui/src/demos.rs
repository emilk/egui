//! Demo-code for showing how Egui is used.
//!
//! The demo-code is also used in benchmarks and tests.
mod app;
mod color_test;
mod fractal_clock;
pub mod toggle_switch;

pub use {
    app::{DemoApp, DemoWindow},
    color_test::ColorTest,
    fractal_clock::FractalClock,
};
