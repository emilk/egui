# Changelog for egui_glow
All notable changes to the `egui_glow` integration will be noted in this file.


## Unreleased
* Make winit/glutin an optional dependency ([#868](https://github.com/emilk/egui/pull/868)).
* Simplify `EguiGlow` interface ([#871](https://github.com/emilk/egui/pull/871)).
* Remove `EguiGlow::is_quit_event` ([#881](https://github.com/emilk/egui/pull/881)).
* Updated `glutin` to 0.28 ([#930](https://github.com/emilk/egui/pull/930)).


## 0.15.0 - 2021-10-24
`egui_glow` has been newly created, with feature parity to `egui_glium`.

As `glow` is a set of lower-level bindings to OpenGL, this crate is potentially less stable than `egui_glium`,
but hopefully this will one day replace `egui_glium` as the default backend for `eframe`.
