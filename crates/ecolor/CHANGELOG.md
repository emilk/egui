# Changelog for ecolor
All notable changes to the `ecolor` crate will be noted in this file.


This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.33.3 - 2025-12-11
Nothing new


## 0.33.2 - 2025-11-13
Nothing new


## 0.33.0 - 2025-10-09
* Align `Color32` to 4 bytes [#7318](https://github.com/emilk/egui/pull/7318) by [@anti-social](https://github.com/anti-social)
* Make the `hex_color` macro `const` [#7444](https://github.com/emilk/egui/pull/7444) by [@YgorSouza](https://github.com/YgorSouza)
* Update MSRV from 1.86 to 1.88 [#7579](https://github.com/emilk/egui/pull/7579) by [@Wumpf](https://github.com/Wumpf)


## 0.32.3 - 2025-09-12
Nothing new


## 0.32.2 - 2025-09-04
Nothing new


## 0.32.1 - 2025-08-15
Nothing new


## 0.32.0 - 2025-07-10
* Fix semi-transparent colors appearing too bright [#5824](https://github.com/emilk/egui/pull/5824) by [@emilk](https://github.com/emilk)
* Remove things that have been deprecated for over a year [#7099](https://github.com/emilk/egui/pull/7099) by [@emilk](https://github.com/emilk)
* Make `Hsva` derive serde [#7132](https://github.com/emilk/egui/pull/7132) by [@bircni](https://github.com/bircni)


## 0.31.1 - 2025-03-05
Nothing new


## 0.31.0 - 2025-02-04
* Add `Color32::CYAN` and `Color32::MAGENTA` [#5663](https://github.com/emilk/egui/pull/5663) by [@juancampa](https://github.com/juancampa)


## 0.30.0 - 2024-12-16
* Use boxed slice for lookup table to avoid stack overflow [#5212](https://github.com/emilk/egui/pull/5212) by [@YgorSouza](https://github.com/YgorSouza)
* Add `Color32::mul` [#5437](https://github.com/emilk/egui/pull/5437) by [@emilk](https://github.com/emilk)


## 0.29.1 - 2024-10-01
Nothing new


## 0.29.0 - 2024-09-26
* Document the fact that the `hex_color!` macro is not `const` [#5169](https://github.com/emilk/egui/pull/5169) by [@YgorSouza](https://github.com/YgorSouza)


## 0.28.1 - 2024-07-05
Nothing new


## 0.28.0 - 2024-07-03
* Fix `hex_color!` macro by re-exporting `color_hex` crate from `ecolor` [#4372](https://github.com/emilk/egui/pull/4372) by [@dataphract](https://github.com/dataphract)
* Remove `extra_asserts` and `extra_debug_asserts` feature flags [#4478](https://github.com/emilk/egui/pull/4478) by [@emilk](https://github.com/emilk)
* Add `Color32::lerp_to_gamma` [#4627](https://github.com/emilk/egui/pull/4627) by [@abey79](https://github.com/abey79)


## 0.27.2 - 2024-04-02
* Nothing new


## 0.27.1 - 2024-03-29
* Nothing new


## 0.27.0 - 2024-03-26
* Nothing new


## 0.26.2 - 2024-02-14
* Nothing new


## 0.26.1 - 2024-02-11
* Nothing new


## 0.26.0 - 2024-02-05
* Nothing new


## 0.25.0 - 2024-01-08
* Add `Color32::from_hex` and `Color32::to_hex` [#3570](https://github.com/emilk/egui/pull/3570) [#3777](https://github.com/emilk/egui/pull/3777) (thanks [@YgorSouza](https://github.com/YgorSouza)!)


## 0.24.1 - 2023-11-30
* Optimize color conversions [#3666](https://github.com/emilk/egui/pull/3666)


## 0.24.0 - 2023-11-23
* Update MSRV to Rust 1.72 [#3595](https://github.com/emilk/egui/pull/3595)
* Add `#[inline]` to all color-related function [38b4234](https://github.com/emilk/egui/commit/38b4234c3282a7c044c18b77234ee8c204efe171)


## 0.22.0 - 2023-05-23
* Nothing new


## 0.21.0 - 2023-02-08
* Add `Color32::gamma_multiply` ([#2437](https://github.com/emilk/egui/pull/2437)).


## 0.20.0 - 2022-12-08
* Split out `ecolor` crate from `epaint`
