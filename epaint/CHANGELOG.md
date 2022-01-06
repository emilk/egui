# epaint changelog

All notable changes to the epaint crate will be documented in this file.


## Unreleased

* Added `Shape::dashed_line_many` ([#1027](https://github.com/emilk/egui/pull/1027)).

## 0.16.0 - 2021-12-29
* Anti-alias path ends  ([#893](https://github.com/emilk/egui/pull/893)).
* `Rgba` now implements `Hash` ([#886](https://github.com/emilk/egui/pull/886)).
* Renamed `Texture` to `FontImage`.


## 0.15.0 - 2021-10-24
* `Fonts::layout_job`: New text layout engine allowing mixing fonts, colors and styles, with underlining and strikethrough.
* New `CircleShape`, `PathShape`, `RectShape` and `TextShape` used in `enum Shape`.
* Add support for rotated text (see `TextShape`).
* Added `"convert_bytemuck"` feature.
