# Changelog for egui_extras
All notable changes to the `egui_extras` integration will be noted in this file.

This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.24.2 - 2023-12-08 - `Table` scroll bug fix
* Fix `Table` scrolling bug [#3690](https://github.com/emilk/egui/pull/3690)


## 0.24.1 - 2023-11-30
* Add more years for datepicker [#3599](https://github.com/emilk/egui/pull/3599) (thanks [@vaqxai](https://github.com/vaqxai)!)


## 0.24.0 - 2023-11-23
* Fix Table stripe pattern when combining `row()` and `rows()` [#3442](https://github.com/emilk/egui/pull/3442) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Update MSRV to Rust 1.72 [#3595](https://github.com/emilk/egui/pull/3595)


## 0.23.0 - 2023-09-27
* `egui_extras::install_image_loaders` [#3297](https://github.com/emilk/egui/pull/3297) [#3315](https://github.com/emilk/egui/pull/3315) [#3328](https://github.com/emilk/egui/pull/3328) (thanks [@jprochazk](https://github.com/jprochazk)!)
* Add syntax highlighting feature to `egui_extras` [#3333](https://github.com/emilk/egui/pull/3333) [#3388](https://github.com/emilk/egui/pull/3388)
* Add `TableBuilder::drag_to_scroll` [#3100](https://github.com/emilk/egui/pull/3100) (thanks [@KYovchevski](https://github.com/KYovchevski)!)
* Add opt-in `puffin` feature to `egui-extras` [#3307](https://github.com/emilk/egui/pull/3307)
* Always depend on `log` crate [#3336](https://github.com/emilk/egui/pull/3336)
* Fix not taking clipping into account when calculating column remainder [#3357](https://github.com/emilk/egui/pull/3357) (thanks [@daxpedda](https://github.com/daxpedda)!)

## 0.22.0 - 2023-05-23
- Add option to hide datepicker button calendar icon [#2910](https://github.com/emilk/egui/pull/2910) (thanks [@Barugon](https://github.com/Barugon)!)


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
