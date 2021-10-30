# Changelog for egui_glow
All notable changes to the `egui_glow` integration will be noted in this file.


## Unreleased
* painter separated to new crate egui_glow_painter that support web and native.

## 0.15.0 - 2021-10-24
`egui_glow` has been newly created, with feature parity to `egui_glium`.

As `glow` is a set of lower-level bindings to OpenGL, this crate is potentially less stable than `egui_glium`,
but hopefully this will one day replace `egui_glium` as the default backend for `eframe`.
