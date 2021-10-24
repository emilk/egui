# epaint changelog

All notable changes to the epaint crate will be documented in this file.


## Unreleased


## 0.15.0 - 2021-10-24
* `Fonts::layout_job`: New text layout engine allowing mixing fonts, colors and styles, with underlining and strikethrough.
* New `CircleShape`, `PathShape`, `RectShape` and `TextShape` used in `enum Shape`.
* Add support for rotated text (see `TextShape`).
* Added `"convert_bytemuck"` feature.
