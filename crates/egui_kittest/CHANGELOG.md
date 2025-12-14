# Changelog for egui_kittest
All notable changes to the `egui_kittest` crate will be noted in this file.


This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.33.3 - 2025-12-11
* Enforce consistent snapshot updates [#7744](https://github.com/emilk/egui/pull/7744) by [@lucasmerlin](https://github.com/lucasmerlin)
* `kittest`: add drag-and-drop helpers [#7690](https://github.com/emilk/egui/pull/7690) by [@emilk](https://github.com/emilk)


## 0.33.2 - 2025-11-13
Nothing new


## 0.33.1 - 2025-10-15
* Add `egui_kittest::HarnessBuilder::with_options` [#7638](https://github.com/emilk/egui/pull/7638) by [@emilk](https://github.com/emilk)


## 0.33.0 - 2025-10-09
### ⭐ Added
* Kittest: Add `UPDATE_SNAPSHOTS=force` [#7508](https://github.com/emilk/egui/pull/7508) by [@emilk](https://github.com/emilk)
* Add `egui_kittest::HarnessBuilder::with_os` and set the default to `Nix` [#7493](https://github.com/emilk/egui/pull/7493) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add `Harness::debug_open_snapshot` helper [#7590](https://github.com/emilk/egui/pull/7590) by [@lucasmerlin](https://github.com/lucasmerlin)

### 🔧 Changed
* Include popups and tooltips in `Harness::fit_contents` [#7556](https://github.com/emilk/egui/pull/7556) by [@oxkitsune](https://github.com/oxkitsune)
* Adjust when we write .diff and .new snapshot images [#7571](https://github.com/emilk/egui/pull/7571) by [@emilk](https://github.com/emilk)
* Update MSRV from 1.86 to 1.88 [#7579](https://github.com/emilk/egui/pull/7579) by [@Wumpf](https://github.com/Wumpf)
* `Harness`: Add `remove_cursor`,  `event` and `event_modifiers` [#7607](https://github.com/emilk/egui/pull/7607) by [@lucasmerlin](https://github.com/lucasmerlin)
* Use software texture filtering in kittest [#7602](https://github.com/emilk/egui/pull/7602) by [@emilk](https://github.com/emilk)
* Write .new.png file if snapshot is missing [#7610](https://github.com/emilk/egui/pull/7610) by [@lucasmerlin](https://github.com/lucasmerlin)

### 🔥 Removed
* Remove deprecated `Harness::wgpu_snapshot` and related fns [#7504](https://github.com/emilk/egui/pull/7504) by [@bircni](https://github.com/bircni)


## 0.32.3 - 2025-09-12
Nothing new


## 0.32.2 - 2025-09-04
* Allow masking widgets in kittest snapshots [#7467](https://github.com/emilk/egui/pull/7467) by [@lucasmerlin](https://github.com/lucasmerlin)


## 0.32.1 - 2025-08-15
* Fix `UPDATE_SNAPSHOTS`: only update if we didn't pass the test [#7455](https://github.com/emilk/egui/pull/7455) by [@emilk](https://github.com/emilk)


## 0.32.0 - 2025-07-10
### ⭐ Added
* Add `ImageLoader::has_pending` and `wait_for_pending_images` [#7030](https://github.com/emilk/egui/pull/7030) by [@lucasmerlin](https://github.com/lucasmerlin)
* Create custom `egui_kittest::Node` [#7138](https://github.com/emilk/egui/pull/7138) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add `HarnessBuilder::theme` [#7289](https://github.com/emilk/egui/pull/7289) by [@emilk](https://github.com/emilk)
* Add support for scrolling via accesskit / kittest [#7286](https://github.com/emilk/egui/pull/7286) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add `failed_pixel_count_threshold` [#7092](https://github.com/emilk/egui/pull/7092) by [@bircni](https://github.com/bircni)

### 🔧 Changed
* More ergonomic functions taking `Impl Into<String>` [#7307](https://github.com/emilk/egui/pull/7307) by [@emlik](https://github.com/emilk)
* Update kittest to 0.2 [#7332](https://github.com/emilk/egui/pull/7332) by [@lucasmerlin](https://github.com/lucasmerlin)


## 0.31.1 - 2025-03-05
* Fix modifiers not working in kittest [#5693](https://github.com/emilk/egui/pull/5693) by [@lucasmerlin](https://github.com/lucasmerlin)
* Enable all features for egui_kittest docs [#5711](https://github.com/emilk/egui/pull/5711) by [@YgorSouza](https://github.com/YgorSouza)
* Run a frame per queued event in egui_kittest [#5704](https://github.com/emilk/egui/pull/5704) by [@lucasmerlin](https://github.com/lucasmerlin)
* Add guidelines for image comparison tests [#5714](https://github.com/emilk/egui/pull/5714) by [@Wumpf](https://github.com/Wumpf)


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
