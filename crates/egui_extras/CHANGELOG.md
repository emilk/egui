# Changelog for egui_extras
All notable changes to the `egui_extras` integration will be noted in this file.

This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.27.2 - 2024-04-02
* Nothing new


## 0.27.1 - 2024-03-29
* Nothing new


## 0.27.0 - 2024-03-26
* Add scroll bar visibility option to `Table` widget [#3981](https://github.com/emilk/egui/pull/3981) (thanks [@richardhozak](https://github.com/richardhozak)!)
* Update `ehttp` to 0.5 [#4055](https://github.com/emilk/egui/pull/4055)
* Fix: assign a different id to each table cell, avoiding id clashes [#4076](https://github.com/emilk/egui/pull/4076)
* Fix interaction with widgets inside selectable rows of `Table` [#4077](https://github.com/emilk/egui/pull/4077)
* Fixed handling of `file://` protocol for images [#4107](https://github.com/emilk/egui/pull/4107) (thanks [@varphone](https://github.com/varphone)!)
* Option to change date picker format [#4180](https://github.com/emilk/egui/pull/4180) (thanks [@zaaarf](https://github.com/zaaarf)!)
* Added ability to disable highlighting of weekend days in `DatePickerPopup`. [#4151](https://github.com/emilk/egui/pull/4151) (thanks [@hiyosilver](https://github.com/hiyosilver)!)


## 0.26.2 - 2024-02-14
* Nothing new


## 0.26.1 - 2024-02-11
* Nothing new


## 0.26.0 - 2024-02-05
* Remove `unwrap`s in SVG scaling [#3826](https://github.com/emilk/egui/pull/3826) (thanks [@amPerl](https://github.com/amPerl)!)
* Update to ehttp 0.4 [#3834](https://github.com/emilk/egui/pull/3834)
* Fix `StripBuilder` not allocating its used space [#3957](https://github.com/emilk/egui/pull/3957) (thanks [@IVAN-MK7](https://github.com/IVAN-MK7)!)
* Override text color with stroke selection color for selected cells [#3968](https://github.com/emilk/egui/pull/3968) (thanks [@njust](https://github.com/njust)!)


## 0.25.0 - 2024-01-08
* Implement table row selection and hover highlighting [#3347](https://github.com/emilk/egui/pull/3347) (thanks [@laurooyen](https://github.com/laurooyen)!)
* Fix `egui_extras::Table` scrolling bug [#3690](https://github.com/emilk/egui/pull/3690) (thanks [@abey79](https://github.com/abey79)!)
* Fix crash due to assertion during image loading from http [#3750](https://github.com/emilk/egui/pull/3750)
* Update resvg dependency of egui_extras [#3719](https://github.com/emilk/egui/pull/3719) (thanks [@PingPongun](https://github.com/PingPongun)!)


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
