# Changelog for egui-winit
All notable changes to the `egui-winit` integration will be noted in this file.


## Unreleased
* Fix docs.rs build ([#2420](https://github.com/emilk/egui/pull/2420)).


## 0.20.0 - 2022-12-08
* The default features of the `winit` crate are not enabled if the default features of `egui-winit` are disabled too ([#1971](https://github.com/emilk/egui/pull/1971)).
* Added new feature `wayland` which enables Wayland support ([#1971](https://github.com/emilk/egui/pull/1971)).
* Don't repaint when just moving window ([#1980](https://github.com/emilk/egui/pull/1980)).
* Added optional integration with [AccessKit](https://accesskit.dev/) for implementing platform accessibility APIs ([#2294](https://github.com/emilk/egui/pull/2294)).

## 0.19.0 - 2022-08-20
* MSRV (Minimum Supported Rust Version) is now `1.61.0` ([#1846](https://github.com/emilk/egui/pull/1846)).
* Fixed clipboard on Wayland ([#1613](https://github.com/emilk/egui/pull/1613)).
* Allow deferred render + surface state initialization for Android ([#1634](https://github.com/emilk/egui/pull/1634)).
* Fixed window position persistence ([#1745](https://github.com/emilk/egui/pull/1745)).
* Fixed mouse cursor change on Linux ([#1747](https://github.com/emilk/egui/pull/1747)).
* Use the new `RawInput::has_focus` field to indicate whether the window has the keyboard focus ([#1859](https://github.com/emilk/egui/pull/1859)).


## 0.18.0 - 2022-04-30
* Reexport `egui` crate
* MSRV (Minimum Supported Rust Version) is now `1.60.0` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Added new feature `puffin` to add [`puffin profiler`](https://github.com/EmbarkStudios/puffin) scopes ([#1483](https://github.com/emilk/egui/pull/1483)).
* Renamed the feature `convert_bytemuck` to `bytemuck` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Renamed the feature `serialize` to `serde` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Removed the features `dark-light` and `persistence` ([#1542](https://github.com/emilk/egui/pull/1542)).


## 0.17.0 - 2022-02-22
* Fixed horizontal scrolling direction on Linux.
* Replaced `std::time::Instant` with `instant::Instant` for WebAssembly compatability ([#1023](https://github.com/emilk/egui/pull/1023))
* Automatically detect and apply dark or light mode from system ([#1045](https://github.com/emilk/egui/pull/1045)).
* Fixed `enable_drag` on Windows OS ([#1108](https://github.com/emilk/egui/pull/1108)).
* Shift-scroll will now result in horizontal scrolling on all platforms ([#1136](https://github.com/emilk/egui/pull/1136)).
* Require knowledge about max texture side (e.g. `GL_MAX_TEXTURE_SIZE`)) ([#1154](https://github.com/emilk/egui/pull/1154)).


## 0.16.0 - 2021-12-29
* Added helper `EpiIntegration` ([#871](https://github.com/emilk/egui/pull/871)).
* Fixed shift key getting stuck enabled with the X11 option `shift:both_capslock` enabled ([#849](https://github.com/emilk/egui/pull/849)).
* Removed `State::is_quit_event` and `State::is_quit_shortcut` ([#881](https://github.com/emilk/egui/pull/881)).
* Updated `winit` to 0.26 ([#930](https://github.com/emilk/egui/pull/930)).


## 0.15.0 - 2021-10-24
First stand-alone release. Previously part of `egui_glium`.
