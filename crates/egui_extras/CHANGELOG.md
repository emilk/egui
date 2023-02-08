# Changelog for egui_extras
All notable changes to the `egui_extras` integration will be noted in this file.


## Unreleased


## 0.21.0 - 2023-02-08
* Update to egui 0.21


## 0.20.0 - 2022-12-08
* Added `RetainedImage::from_svg_bytes_with_size` to be able to specify a size for SVGs to be rasterized at.
* Lots of `Table` improvements ([#2369](https://github.com/emilk/egui/pull/2369)):
    * Double-click column separators to auto-size the column.
    * All `Table` now store state. You may see warnings about reused table ids. Use `ui.push_id` to fix this.
    * `TableBuilder::column` takes a `Column` instead of a `Size`.
    * `Column` controls default size, size range, resizing, and clipping of columns.
    * `Column::auto` will pick a size automatically
    * Added `Table::scroll_to_row`.
    * Added `Table::min_scrolled_height` and `Table::max_scroll_height`.
    * Added `TableBody::max_size`.
    * `Table::scroll` renamed to `Table::vscroll`.
    * `egui_extras::Strip` now has `clip: false` by default.
    * Fix bugs when putting `Table` inside of a horizontal `ScrollArea`.
    * Many other bug fixes.
* Add `Table::auto_shrink` - set to `false` to expand table to fit its containing `Ui` ([#2371](https://github.com/emilk/egui/pull/2371)).
* Added `TableBuilder::vertical_scroll_offset`: method to set vertical scroll offset position for a table ([#1946](https://github.com/emilk/egui/pull/1946)).


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
