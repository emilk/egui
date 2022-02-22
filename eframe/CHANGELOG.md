# Changelog for eframe
All notable changes to the `eframe` and `epi` crates.

NOTE: [`egui_web`](../egui_web/CHANGELOG.md), [`egui-winit`](../egui-winit/CHANGELOG.md), [`egui_glium`](../egui_glium/CHANGELOG.md), and [`egui_glow`](../egui_glow/CHANGELOG.md) have their own changelogs!


## Unreleased


## 0.17.0 - 2022-02-22
* Removed `Frame::alloc_texture`. Use `egui::Context::load_texture` instead ([#1110](https://github.com/emilk/egui/pull/1110)).
* The default native backend is now `egui_glow` (instead of `egui_glium`) ([#1020](https://github.com/emilk/egui/pull/1020)).
* The default web painter is now `egui_glow` (instead of WebGL) ([#1020](https://github.com/emilk/egui/pull/1020)).
* Automatically detect and apply dark or light mode from system ([#1045](https://github.com/emilk/egui/pull/1045)).
* Fix horizontal scrolling direction on Linux.
* Added `App::on_exit_event` ([#1038](https://github.com/emilk/egui/pull/1038))
* Added `NativeOptions::initial_window_pos`.
* Fixed `enable_drag` for Windows OS ([#1108](https://github.com/emilk/egui/pull/1108)).
* Shift-scroll will now result in horizontal scrolling on all platforms ([#1136](https://github.com/emilk/egui/pull/1136)).
* Log using the `tracing` crate. Log to stdout by adding `tracing_subscriber::fmt::init();` to your `main` ([#1192](https://github.com/emilk/egui/pull/1192)).
* Expose all parts of the location/url in `frame.info().web_info` ([#1258](https://github.com/emilk/egui/pull/1258)).


## 0.16.0 - 2021-12-29
* `Frame` can now be cloned, saved, and passed to background threads ([#999](https://github.com/emilk/egui/pull/999)).
* Added `Frame::request_repaint` to replace `repaint_signal` ([#999](https://github.com/emilk/egui/pull/999)).
* Added `Frame::alloc_texture/free_texture` to replace `tex_allocator` ([#999](https://github.com/emilk/egui/pull/999)).


## 0.15.0 - 2021-10-24
* `Frame` now provides `set_window_title` to set window title dynamically
* `Frame` now provides `set_decorations` to set whether to show window decorations.
* Remove "http" feature (use https://github.com/emilk/ehttp instead!).
* Increase native scroll speed.
* Add `App::persist_native_window` and `App::persist_egui_memory` to control what gets persisted.
* Add new backend `egui_glow` as an alternative to `egui_glium`. Enable with `default-features = false, features = ["default_fonts", "egui_glow"]`.


## 0.14.0 - 2021-08-24
* Add dragging and dropping files into egui.
* Improve http fetch API.
* `run_native` now returns when the app is closed.


## 0.13.1 - 2021-06-24
* Fix `http` feature flag and docs


## 0.13.0 - 2021-06-24
* `App::setup` now takes a `Frame` and `Storage` by argument.
* `App::load` has been removed. Implement `App::setup` instead.


## 0.12.0 - 2021-05-10
* Moved options out of `trait App` into new `NativeOptions`.
* Add option for `always_on_top`.


## 0.11.0 - 2021-04-05
* You can now turn your window transparent with the `App::transparent` option.
* You can now disable window decorations with the `App::decorated` option.


## 0.10.0 - 2021-02-28
* [You can now set your own app icons](https://github.com/emilk/egui/pull/193).
* You can control the initial size of the native window with `App::initial_window_size`.
* You can control the maximum egui web canvas size with `App::max_size_points`.
* `Frame::tex_allocator()` no longer returns an `Option` (there is always a texture allocator).


## 0.9.0 - 2021-02-07
* [Add support for HTTP body](https://github.com/emilk/egui/pull/139).


## 0.8.0 - 2021-01-17
* Simplify `TextureAllocator` interface.


## 0.7.0 - 2021-01-04
* Initial release of `eframe`
