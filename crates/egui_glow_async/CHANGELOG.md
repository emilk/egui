# Changelog for egui_glow
All notable changes to the `egui_glow` integration will be noted in this file.

This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.



## 0.27.2 - 2024-04-02
* Nothing new


## 0.27.1 - 2024-03-29
* Nothing new


## 0.27.0 - 2024-03-26
* Only disable sRGB framebuffer on supported platforms [#3994](https://github.com/emilk/egui/pull/3994) (thanks [@Nopey](https://github.com/Nopey)!)
* Update memoffset to 0.9.0, arboard to 3.3.1, and remove egui_glow's needless dependency on pure_glow's deps  [#4036](https://github.com/emilk/egui/pull/4036) (thanks [@Nopey](https://github.com/Nopey)!)


## 0.26.2 - 2024-02-14
* Update memoffset to 0.9.0, arboard to 3.3.1, and remove egui_glow's needless dependency on pure_glow's deps  [#4036](https://github.com/emilk/egui/pull/4036) (thanks [@Nopey](https://github.com/Nopey)!)


## 0.26.1 - 2024-02-11
* Only disable sRGB framebuffer on supported platforms [#3994](https://github.com/emilk/egui/pull/3994) (thanks [@Nopey](https://github.com/Nopey)!)


## 0.26.0 - 2024-02-05
* Add `x11` and `wayland` features [#3909](https://github.com/emilk/egui/pull/3909) (thanks [@YgorSouza](https://github.com/YgorSouza)!)


## 0.25.0 - 2024-01-08
* Update to glow 0.13 [#3715](https://github.com/emilk/egui/pull/3715)
* Make glow `Send + Sync` again [#3646](https://github.com/emilk/egui/pull/3646) (thanks [@surban](https://github.com/surban)!)


## 0.24.1 - 2023-11-30
* Improve a docstring


## 0.24.0 - 2023-11-23
* Change `Arc<glow::Context>` to `Rc<glow::Context>` [#3598](https://github.com/emilk/egui/pull/3598)
* Update MSRV to Rust 1.72 [#3595](https://github.com/emilk/egui/pull/3595)
* Clamp viewport values [#3604](https://github.com/emilk/egui/pull/3604) (thanks [@Wumpf](https://github.com/Wumpf)!)


## 0.23.0 - 2023-09-27
* Update `egui`


## 0.22.0 - 2023-05-23
* Update `egui`


## 0.21.0 - 2023-02-08
* Update to `glow` 0.12 ([#2695](https://github.com/emilk/egui/pull/2695)).
* Remove the `screen_reader` feature ([#2669](https://github.com/emilk/egui/pull/2669)).


## 0.20.1 - 2022-12-11
* Fix [docs.rs](https://docs.rs/egui_glow) build ([#2420](https://github.com/emilk/egui/pull/2420)).


## 0.20.0 - 2022-12-08
* Allow empty textures.
* Added `shader_version` variable on `EguiGlow::new` for easier cross compiling on different OpenGL | ES targets ([#1993](https://github.com/emilk/egui/pull/1993)).


## 0.19.0 - 2022-08-20
* MSRV (Minimum Supported Rust Version) is now `1.61.0` ([#1846](https://github.com/emilk/egui/pull/1846)).
* `EguiGlow::new` now takes an `EventLoopWindowTarget<E>` instead of a `winit::Window` ([#1634](https://github.com/emilk/egui/pull/1634)).
* Use `Arc` for `glow::Context` instead of `Rc` ([#1640](https://github.com/emilk/egui/pull/1640)).
* Fixed `glClear` on WebGL1 ([#1658](https://github.com/emilk/egui/pull/1658)).
* Add `Painter::intermediate_fbo` which tells callbacks where to render. This is only needed if the callbacks use their own FBO:s and need to know what to restore to.


## 0.18.1 - 2022-05-05
* Remove calls to `gl.get_error` in release builds to speed up rendering ([#1583](https://github.com/emilk/egui/pull/1583)).


## 0.18.0 - 2022-04-30
* Improved logging on rendering failures.
* Added new `NativeOptions`: `vsync`, `multisampling`, `depth_buffer`, `stencil_buffer`.
* Fixed potential scale bug when DPI scaling changes (e.g. when dragging a  window between different displays) ([#1441](https://github.com/emilk/egui/pull/1441)).
* MSRV (Minimum Supported Rust Version) is now `1.60.0` ([#1467](https://github.com/emilk/egui/pull/1467)).
* `clipboard`, `links`, `winit` are now all opt-in features ([#1467](https://github.com/emilk/egui/pull/1467)).
* Added new feature `puffin` to add [`puffin profiler`](https://github.com/EmbarkStudios/puffin) scopes ([#1483](https://github.com/emilk/egui/pull/1483)).
* Removed the features `dark-light`, `default_fonts` and `persistence` ([#1542](https://github.com/emilk/egui/pull/1542)).


## 0.17.0 - 2022-02-22
* `EguiGlow::run` no longer returns the shapes to paint, but stores them internally until you call `EguiGlow::paint` ([#1110](https://github.com/emilk/egui/pull/1110)).
* Added `set_texture_filter` method to `Painter` ([#1041](https://github.com/emilk/egui/pull/1041)).
* Fixed failure to run in Chrome ([#1092](https://github.com/emilk/egui/pull/1092)).
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
