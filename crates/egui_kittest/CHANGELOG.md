# Changelog for egui_kittest
All notable changes to the `egui_kittest` crate will be noted in this file.


This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.31.0 - 2025-02-04
### ⭐ Added
* Add `Harness::new_eframe` and `TestRenderer` trait [#5539](https://github.com/emilk/egui/pull/5539) by [@lucasmerlin](https://github.com/lucasmerlin)
* Change `Harness::run` to run until no more repaints are requested [#5580](https://github.com/emilk/egui/pull/5580) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add `SnapshotResults` struct to `egui_kittest` [#5672](https://github.com/emilk/egui/pull/5672) by [@lucasmerlin](https://github.com/lucasmerlin)

### 🔧 Changed
* Extend `WgpuSetup`, `egui_kittest` now prefers software rasterizers for testing [#5506](https://github.com/emilk/egui/pull/5506) by [@Wumpf](https://github.com/Wumpf)
* Write `.old.png` files when updating images [#5578](https://github.com/emilk/egui/pull/5578) by [@emilk](https://github.com/emilk)
* Succeed and keep going when `UPDATE_SNAPSHOTS` is set [#5649](https://github.com/emilk/egui/pull/5649) by [@emilk](https://github.com/emilk)


## 0.30.0 - 2024-12-16 - Initial relrease
* Support for egui 0.30.0
* Automate clicks and text input
* Automatic screenshot testing with wgpu
