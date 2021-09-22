# egui_fro_winit

[![Latest version](https://img.shields.io/crates/v/egui_fro_winit.svg)](https://crates.io/crates/egui_fro_winit)
[![Documentation](https://docs.rs/egui_fro_winit/badge.svg)](https://docs.rs/egui_fro_winit)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

This crates provides bindings between [`egui`](https://github.com/emilk/egui) and [glium](https://crates.io/crates/winit).

The library translates winit events to egui, handled copy/paste, updates the cursor, open links clicked in egui, etc.
