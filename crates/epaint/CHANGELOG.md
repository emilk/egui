# epaint changelog
All notable changes to the epaint crate will be documented in this file.


## Unreleased


## 0.20.0 - 2022-12-08
* ⚠️ BREAKING: Fix text being too small ([#2069](https://github.com/emilk/egui/pull/2069)).
* ⚠️ BREAKING: epaint now expects integrations to do all color blending in gamma space ([#2071](https://github.com/emilk/egui/pull/2071)).
* Improve mixed CJK/Latin line-breaking ([#1986](https://github.com/emilk/egui/pull/1986)).
* Added `Fonts::has_glyph(s)` for querying if a glyph is supported ([#2202](https://github.com/emilk/egui/pull/2202)).
* Added support for [thin space](https://en.wikipedia.org/wiki/Thin_space).
* Split out color into its own crate, `ecolor` ([#2399](https://github.com/emilk/egui/pull/2399)).


## 0.19.0 - 2022-08-20
* MSRV (Minimum Supported Rust Version) is now `1.61.0` ([#1846](https://github.com/emilk/egui/pull/1846)).
* Added `epaint::hex_color!` to create `Color32`'s from hex strings under the `color-hex` feature ([#1596](https://github.com/emilk/egui/pull/1596)).
* Optimize tessellation of filled circles by 10x or more ([#1616](https://github.com/emilk/egui/pull/1616)).
* Added opt-in feature `deadlock_detection` to detect double-lock of mutexes on the same thread ([#1619](https://github.com/emilk/egui/pull/1619)).
* Texture loading now takes a `TexureOptions` with minification and magnification filters ([#2224](https://github.com/emilk/egui/pull/2224)).


## 0.18.1 - 2022-05-01
* Change `Shape::Callback` from `&dyn Any` to `&mut dyn Any` to support more backends.


## 0.18.0 - 2022-04-30
* MSRV (Minimum Supported Rust Version) is now `1.60.0` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Added `Shape::Callback` for backend-specific painting ([#1351](https://github.com/emilk/egui/pull/1351)).
* Added more text wrapping options ([#1291](https://github.com/emilk/egui/pull/1291)):
  * Added `TextWrapping` struct containing all wrapping options.
  * Added `LayoutJob::wrap` field containing these options.
  * Moved `LayoutJob::wrap_width` to `TextWrapping::max_width`.
  * Added `TextWrapping::max_rows` to limit amount of rows the text should have.
  * Added `TextWrapping::break_anywhere` to control should the text break at appropriate places or not.
  * Added `TextWrapping::overflow_character` to specify what character should be used to represent clipped text.
* Removed the `single_threaded/multi_threaded` flags - epaint is now always thread-safe ([#1390](https://github.com/emilk/egui/pull/1390)).
* `Tessellator::from_options` is now `Tessellator::new` ([#1408](https://github.com/emilk/egui/pull/1408)).
* Renamed `TessellationOptions::anti_alias` to `feathering` ([#1408](https://github.com/emilk/egui/pull/1408)).
* Renamed `AlphaImage` to `FontImage` to discourage any other use for it ([#1412](https://github.com/emilk/egui/pull/1412)).
* Dark text is darker and more readable on bright backgrounds ([#1412](https://github.com/emilk/egui/pull/1412)).
* Fixed panic when tessellating a `Shape::Vec` containing meshes with differing `TextureId`s ([#1445](https://github.com/emilk/egui/pull/1445)).
* Added `Shape::galley_with_color` which adds the functionality of `Painter::galley_with_color` into the Shape enum ([#1461](https://github.com/emilk/egui/pull/1461)).
* Renamed the feature `convert_bytemuck` to `bytemuck` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Renamed the feature `serialize` to `serde` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Added line breaking rules for Japanese text ([#1498](https://github.com/emilk/egui/pull/1498)).
* Optimize tessellation of circles and boxes with rounded corners ([#1547](https://github.com/emilk/egui/pull/1547)).


## 0.17.0 - 2022-02-22
* Much improved font selection ([#1154](https://github.com/emilk/egui/pull/1154)):
  * Replaced `TextStyle` with `FontId` which lets you pick any font size and font family.
  * Replaced `Fonts::font_image` with `font_image_delta` for partial font atlas updates.
* Made the v-align and scale of user fonts tweakable ([#1241](https://github.com/emilk/egui/pull/1027)).
* Added `ImageData` and `TextureManager` for loading images into textures ([#1110](https://github.com/emilk/egui/pull/1110)).
* Added `Shape::dashed_line_many` ([#1027](https://github.com/emilk/egui/pull/1027)).
* Replaced `corner_radius: f32` with `rounding: Rounding`, allowing per-corner rounding settings ([#1206](https://github.com/emilk/egui/pull/1206)).
* Fixed anti-aliasing of filled paths with counter-clockwise winding order.
* Improve the anti-aliasing of filled paths with sharp corners, at the cost of these corners sometimes becoming badly extruded instead (see https://github.com/emilk/egui/issues/1226).


## 0.16.0 - 2021-12-29
* Anti-alias path ends ([#893](https://github.com/emilk/egui/pull/893)).
* `Rgba` now implements `Hash` ([#886](https://github.com/emilk/egui/pull/886)).
* Renamed `Texture` to `FontImage`.


## 0.15.0 - 2021-10-24
* `Fonts::layout_job`: New text layout engine allowing mixing fonts, colors and styles, with underlining and strikethrough.
* New `CircleShape`, `PathShape`, `RectShape` and `TextShape` used in `enum Shape`.
* Added support for rotated text (see `TextShape`).
* Added `"convert_bytemuck"` feature.
