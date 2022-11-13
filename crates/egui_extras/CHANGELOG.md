# Changelog for egui_extras
All notable changes to the `egui_extras` integration will be noted in this file.


## Unreleased
* Added `TableBuilder::vertical_scroll_offset`: method to set vertical scroll offset position for a table ([#1946](https://github.com/emilk/egui/pull/1946)).
* Added `RetainedImage::from_svg_bytes_with_size` to be able to specify a size for SVGs to be rasterized at.

## 0.19.0 - 2022-08-20
* MSRV (Minimum Supported Rust Version) is now `1.61.0` ([#1846](https://github.com/emilk/egui/pull/1846)).
* You can now specify a texture filter for `RetainedImage` ([#1636](https://github.com/emilk/egui/pull/1636)).
* Fixed uneven `Table` striping ([#1680](https://github.com/emilk/egui/pull/1680)).


## 0.18.0 - 2022-04-30
* Added `Strip`, `Table` and `DatePicker` ([#963](https://github.com/emilk/egui/pull/963)).
* MSRV (Minimum Supported Rust Version) is now `1.60.0` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Renamed feature "persistence" to "serde" ([#1540](https://github.com/emilk/egui/pull/1540)).


## 0.17.0 - 2022-02-22
* `RetainedImage`: convenience for loading svg, png, jpeg etc and showing them in egui.
