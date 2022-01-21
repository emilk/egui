# Changelog for egui-winit

All notable changes to the `egui-winit` integration will be noted in this file.


## Unreleased
* Fix horizontal scrolling direction on Linux.
* Replaced `std::time::Instant` with `instant::Instant` for WebAssembly compatability ([#1023](https://github.com/emilk/egui/pull/1023))
* Shift-scroll will now result in horizontal scrolling on all platforms ((#1136)[https://github.com/emilk/egui/pull/1136]).


## 0.16.0 - 2021-12-29
* Added helper `EpiIntegration` ([#871](https://github.com/emilk/egui/pull/871)).
* Fixed shift key getting stuck enabled with the X11 option `shift:both_capslock` enabled ([#849](https://github.com/emilk/egui/pull/849)).
* Removed `State::is_quit_event` and `State::is_quit_shortcut` ([#881](https://github.com/emilk/egui/pull/881)).
* Updated `winit` to 0.26 ([#930](https://github.com/emilk/egui/pull/930)).


## 0.15.0 - 2021-10-24
First stand-alone release. Previously part of `egui_glium`.
