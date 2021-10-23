# `epi`: the [`egui`](https://github.com/emilk/egui) application programming interface

[![Latest version](https://img.shields.io/crates/v/epi.svg)](https://crates.io/crates/epi)
[![Documentation](https://docs.rs/epi/badge.svg)](https://docs.rs/epi)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

`epi` is a backend-agnostic interface for writing apps using `egui` (a platform agnostic GUI library).

This crate provides a common interface for programming an app using egui, which can then be easily plugged into [`eframe`](https://github.com/emilk/egui/tree/master/eframe) (which is a wrapper over [`egui_web`](https://github.com/emilk/egui/tree/master/egui_web), [`egui_glium`](https://github.com/emilk/egui/tree/master/egui_glium) and [`egui_glow`](https://github.com/emilk/egui/tree/master/egui_glow)).

This crate is only for those that want to write an app that can be compiled both natively and for the web.
