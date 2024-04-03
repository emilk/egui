# epaint changelog
All notable changes to the epaint crate will be documented in this file.

This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.27.2 - 2024-04-02
* Nothing new


## 0.27.1 - 2024-03-29
* Fix visual glitch on the right side of highly rounded rectangles [#4244](https://github.com/emilk/egui/pull/4244)
* Prevent visual glitch when shadow blur width is very high [#4245](https://github.com/emilk/egui/pull/4245)


## 0.27.0 - 2024-03-26
* Add `ColorImage::from_gray_iter` [#3536](https://github.com/emilk/egui/pull/3536) (thanks [@wangxiaochuTHU](https://github.com/wangxiaochuTHU)!)
* Convenience const fn for `Margin`, `Rounding` and `Shadow` [#4080](https://github.com/emilk/egui/pull/4080) (thanks [@0Qwel](https://github.com/0Qwel)!)
* Added `Shape::{scale,translate}` wrappers [#4090](https://github.com/emilk/egui/pull/4090) (thanks [@varphone](https://github.com/varphone)!)
* Add `EllipseShape` [#4122](https://github.com/emilk/egui/pull/4122) (thanks [@TheTacBanana](https://github.com/TheTacBanana)!)
* Add `Margin` to `epaint` [#4231](https://github.com/emilk/egui/pull/4231)
* CSS-like `Shadow` with offset, spread, and blur [#4232](https://github.com/emilk/egui/pull/4232)


## 0.26.2 - 2024-02-14
* Nothing new


## 0.26.1 - 2024-02-11
* Nothing new


## 0.26.0 - 2024-02-05
* Add `Align2::anchor_size` [#3863](https://github.com/emilk/egui/pull/3863)
* Add opacity factor to `TextShape` [#3916](https://github.com/emilk/egui/pull/3916) (thanks [@StratusFearMe21](https://github.com/StratusFearMe21)!)
* Parallel tessellation with opt-in `rayon` feature [#3934](https://github.com/emilk/egui/pull/3934)


## 0.25.0 - 2024-01-08
* Replace a special `Color32::PLACEHOLDER` with widget fallback color [#3727](https://github.com/emilk/egui/pull/3727)
* Add support for dashed lines with offset [#3720](https://github.com/emilk/egui/pull/3720) (thanks [@oscargus](https://github.com/oscargus)!)
* Impl `Clone` for `Fonts` [#3737](https://github.com/emilk/egui/pull/3737)
* Fix: allow using the full Private Use Area for custom fonts [#3509](https://github.com/emilk/egui/pull/3509) (thanks [@varphone](https://github.com/varphone)!)
* Add `Color32::from_hex` and `Color32::to_hex` [#3570](https://github.com/emilk/egui/pull/3570) [#3777](https://github.com/emilk/egui/pull/3777) (thanks [@YgorSouza](https://github.com/YgorSouza)!)


## 0.24.1 - 2023-11-30
* Optimize `FontImage::srgba_pixels` and reduce the initial font atlas texture size from 8MiB -> 1MiB [#3666](https://github.com/emilk/egui/pull/3666)


## 0.24.0 - 2023-11-23
* Use `impl Into<Stroke>` as argument in a few more places [#3420](https://github.com/emilk/egui/pull/3420) (thanks [@Phen-Ro](https://github.com/Phen-Ro)!)
* Update MSRV to Rust 1.72 [#3595](https://github.com/emilk/egui/pull/3595)
* Make `ViewportInPixels` use integers, and clamp to bounds [#3604](https://github.com/emilk/egui/pull/3604) (thanks [@Wumpf](https://github.com/Wumpf)!)


## 0.23.0 - 2023-09-27
* Update MSRV to Rust 1.70.0 [#3310](https://github.com/emilk/egui/pull/3310)
* Add option to truncate text at wrap width [#3244](https://github.com/emilk/egui/pull/3244) [#3366](https://github.com/emilk/egui/pull/3366)
* Add control of line height and letter spacing [#3302](https://github.com/emilk/egui/pull/3302)
* Support images with rounded corners [#3257](https://github.com/emilk/egui/pull/3257)
* Add `ColorImage::from_gray` [#3166](https://github.com/emilk/egui/pull/3166) (thanks [@thomaseliot](https://github.com/thomaseliot)!)
* Provide `into_inner()` for `egui::mutex::{Mutex, RwLock}` [#3110](https://github.com/emilk/egui/pull/3110) (thanks [@KmolYuan](https://github.com/KmolYuan)!)
* Fix problems with tabs in text [#3355](https://github.com/emilk/egui/pull/3355)
* Refactor: change `ClippedShape` from struct-enum to a normal struct [#3225](https://github.com/emilk/egui/pull/3225)
* Document when `Galley`s get invalidated [#3024](https://github.com/emilk/egui/pull/3024) (thanks [@e00E](https://github.com/e00E)!)


## 0.22.0 - 2023-05-23
* Fix compiling `epaint` without `bytemuck` dependency [#2913](https://github.com/emilk/egui/pull/2913) (thanks [@lunixbochs](https://github.com/lunixbochs)!)
* Fix documentation for `TextureId::Managed(0)` [#2998](https://github.com/emilk/egui/pull/2998) (thanks [@andersk](https://github.com/andersk)!)


## 0.21.0 - 2023-02-08
* Improve the look of thin white lines ([#2437](https://github.com/emilk/egui/pull/2437)).
* Don't render `\r` (Carriage Return) ([#2452](https://github.com/emilk/egui/pull/2452)).
* Fix bug in `Mesh::split_to_u16` ([#2459](https://github.com/emilk/egui/pull/2459)).
* Improve rendering of very thin rectangles.


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
* Texture loading now takes a `TextureOptions` with minification and magnification filters ([#2224](https://github.com/emilk/egui/pull/2224)).


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
