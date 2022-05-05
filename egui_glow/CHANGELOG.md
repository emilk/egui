# Changelog for egui_glow
All notable changes to the `egui_glow` integration will be noted in this file.


## Unreleased
* Remove calls to `gl.get_error` in release builds to speed up rendering ([#1583](https://github.com/emilk/egui/pull/1583)).


## 0.18.0 - 2022-04-30
* Improved logging on rendering failures.
* Add new `NativeOptions`: `vsync`, `multisampling`, `depth_buffer`, `stencil_buffer`.
* Fixed potential scale bug when DPI scaling changes (e.g. when dragging a  window between different displays) ([#1441](https://github.com/emilk/egui/pull/1441)).
* MSRV (Minimum Supported Rust Version) is now `1.60.0` ([#1467](https://github.com/emilk/egui/pull/1467)).
* `clipboard`, `links`, `winit` are now all opt-in features ([#1467](https://github.com/emilk/egui/pull/1467)).
* Added new feature `puffin` to add [`puffin profiler`](https://github.com/EmbarkStudios/puffin) scopes ([#1483](https://github.com/emilk/egui/pull/1483)).
* Removed the features `dark-light`, `default_fonts` and `persistence` ([#1542](https://github.com/emilk/egui/pull/1542)).


## 0.17.0 - 2022-02-22
* `EguiGlow::run` no longer returns the shapes to paint, but stores them internally until you call `EguiGlow::paint` ([#1110](https://github.com/emilk/egui/pull/1110)).
* Added `set_texture_filter` method to `Painter` ([#1041](https://github.com/emilk/egui/pull/1041)).
* Fix failure to run in Chrome ([#1092](https://github.com/emilk/egui/pull/1092)).
* `EguiGlow::new` and `EguiGlow::paint` now takes `&winit::Window` ([#1151](https://github.com/emilk/egui/pull/1151)).
* Automatically detect and apply dark or light mode from system ([#1045](https://github.com/emilk/egui/pull/1045)).


## 0.16.0 - 2021-12-29
* Made winit/glutin an optional dependency ([#868](https://github.com/emilk/egui/pull/868)).
* Simplified `EguiGlow` interface ([#871](https://github.com/emilk/egui/pull/871)).
* Removed `EguiGlow::is_quit_event` ([#881](https://github.com/emilk/egui/pull/881)).
* Updated `glutin` to 0.28 ([#930](https://github.com/emilk/egui/pull/930)).
* Changed the `Painter` interface slightly ([#999](https://github.com/emilk/egui/pull/999)).


## 0.15.0 - 2021-10-24
`egui_glow` has been newly created, with feature parity to `egui_glium`.

As `glow` is a set of lower-level bindings to OpenGL, this crate is potentially less stable than `egui_glium`,
but hopefully this will one day replace `egui_glium` as the default backend for `eframe`.
