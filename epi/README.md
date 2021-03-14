# egui app programming interface

Backend-agnostic interface for writing apps using [`egui`](https://crates.io/crates/egui) (a platform agnostic GUI library).

This crate provides a common interface for programming an app using egui, which can then be easily plugged into [`egui_frame`](https://crates.io/crates/egui_frame) (which in a wrapper over  [`egui_web`](https://crates.io/crates/egui_web) or [`egui_glium`](https://crates.io/crates/egui_glium)).
