# [`egui`](https://github.com/emilk/egui) demo library

[![Latest version](https://img.shields.io/crates/v/egui_demo_lib.svg)](https://crates.io/crates/egui_demo_lib)
[![Documentation](https://docs.rs/egui_demo_lib/badge.svg)](https://docs.rs/egui_demo_lib)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

This crate contains example code for [`egui`](https://github.com/emilk/egui).

It is in a separate crate for two reasons:

* To ensure it only uses the public `egui` api.
* To remove the amount of code in `egui` proper.
