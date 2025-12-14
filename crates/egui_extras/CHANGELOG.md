# Changelog for egui_extras
All notable changes to the `egui_extras` integration will be noted in this file.

This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.33.3 - 2025-12-11
* Bump `ehttp` to 0.6.0 [#7757](https://github.com/emilk/egui/pull/7757) by [@jprochazk](https://github.com/jprochazk)


## 0.33.2 - 2025-11-13
Nothing new


## 0.33.0 - 2025-10-09
* Fix: use unique id for resize columns in `Table` [#7414](https://github.com/emilk/egui/pull/7414) by [@zezic](https://github.com/zezic)
* Feat: Add serde serialization to SyntectSettings [#7506](https://github.com/emilk/egui/pull/7506) by [@bircni](https://github.com/bircni)
* Make individual egui_extras image loaders public [#7551](https://github.com/emilk/egui/pull/7551) by [@lucasmerlin](https://github.com/lucasmerlin)
* Update MSRV from 1.86 to 1.88 [#7579](https://github.com/emilk/egui/pull/7579) by [@Wumpf](https://github.com/Wumpf)


## 0.32.3 - 2025-09-12
* Fix deadlock in `FileLoader` and `EhttpLoader` [#7515](https://github.com/emilk/egui/pull/7515) by [@emilk](https://github.com/emilk)


## 0.32.2 - 2025-09-04
* Fix memory leak when `forget_image` is called while loading [#7380](https://github.com/emilk/egui/pull/7380) by [@Vanadiae](https://github.com/Vanadiae)
* Fix deadlock in `ImageLoader`, `FileLoader`, `EhttpLoader` [#7494](https://github.com/emilk/egui/pull/7494) by [@lucasmerlin](https://github.com/lucasmerlin)


## 0.32.1 - 2025-08-15
Nothing new


## 0.32.0 - 2025-07-10 - Improved SVG support
### ⭐ Added
* Allow loading multi-MIME formats using the image_loader [#5769](https://github.com/emilk/egui/pull/5769) by [@MYDIH](https://github.com/MYDIH)
* Make ImageLoader use background thread [#5394](https://github.com/emilk/egui/pull/5394) by [@bircni](https://github.com/bircni)
* Add overline option for Table rows [#5637](https://github.com/emilk/egui/pull/5637) by [@akx](https://github.com/akx)
* Support text in SVGs [#5979](https://github.com/emilk/egui/pull/5979) by [@cernec1999](https://github.com/cernec1999)
* Enable setting DatePickerButton start and end year explicitly [#7061](https://github.com/emilk/egui/pull/7061) by [@zachbateman](https://github.com/zachbateman)
* Support custom syntect settings in syntax highlighter [#7084](https://github.com/emilk/egui/pull/7084) by [@mkeeter](https://github.com/mkeeter)

### 🔧 Changed
* Use enum-map serde feature only when serde is enabled [#5748](https://github.com/emilk/egui/pull/5748) by [@tyssyt](https://github.com/tyssyt)
* Better define the meaning of `SizeHint` [#7079](https://github.com/emilk/egui/pull/7079) by [@emilk](https://github.com/emilk)

### 🔥 Removed
* Remove things that have been deprecated for over a year [#7099](https://github.com/emilk/egui/pull/7099) by [@emilk](https://github.com/emilk)

### 🐛 Fixed
* Refactor MIME type support detection in image loader to allow for deferred handling and appended encoding info [#5686](https://github.com/emilk/egui/pull/5686) by [@markusdd](https://github.com/markusdd)
* Fix incorrect color fringe colors on SVG:s [#7069](https://github.com/emilk/egui/pull/7069) by [@emilk](https://github.com/emilk)
* Fix sometimes blurry SVGs [#7071](https://github.com/emilk/egui/pull/7071) by [@emilk](https://github.com/emilk)
* Fix crash in `egui_extras::FileLoader` after `forget_image` [#6995](https://github.com/emilk/egui/pull/6995) by [@bircni](https://github.com/bircni)


## 0.31.1 - 2025-03-05
* Fix image_loader for animated image types [#5688](https://github.com/emilk/egui/pull/5688) by [@BSteffaniak](https://github.com/BSteffaniak)


## 0.31.0 - 2025-02-04
* Animated WebP support [#5470](https://github.com/emilk/egui/pull/5470), [#5586](https://github.com/emilk/egui/pull/5586) by [@Aely0](https://github.com/Aely0)
* Make image extension check case-insensitive [#5501](https://github.com/emilk/egui/pull/5501) by [@RyanBluth](https://github.com/RyanBluth)
* Avoid allocations for loader cache lookup [#5584](https://github.com/emilk/egui/pull/5584) by [@mineichen](https://github.com/mineichen)


## 0.30.0 - 2024-12-16
* Use `Table::id_salt` on `ScrollArea` [#5282](https://github.com/emilk/egui/pull/5282) by [@jwhear](https://github.com/jwhear)
* Use proper `image` crate URI and MIME support detection [#5324](https://github.com/emilk/egui/pull/5324) by [@xangelix](https://github.com/xangelix)
* Support loading images with weird urls and improve error message [#5431](https://github.com/emilk/egui/pull/5431) by [@lucasmerlin](https://github.com/lucasmerlin)


## 0.29.1 - 2024-10-01 - Fix table interaction
* Bug fix: click anywhere on a `Table` row to select it [#5193](https://github.com/emilk/egui/pull/5193) by [@emilk](https://github.com/emilk)


## 0.29.0 - 2024-09-26
### ⭐ Added
* Add `TableRow::set_hovered` [#4820](https://github.com/emilk/egui/pull/4820) by [@addiswebb](https://github.com/addiswebb)
* Add `TableBuilder::id_salt` [#5022](https://github.com/emilk/egui/pull/5022) by [@emilk](https://github.com/emilk)
* Add `TableBuilder::animate_scrolling` [#5159](https://github.com/emilk/egui/pull/5159) by [@ecpost](https://github.com/ecpost)

### 🔧 Changed
* Truncate text in clipped `Table` columns [#5023](https://github.com/emilk/egui/pull/5023) by [@emilk](https://github.com/emilk)
* Change default `max_scroll_height` of `egui::Table` to `f32::INFINITY` [#4817](https://github.com/emilk/egui/pull/4817) by [@abey79](https://github.com/abey79)
* Return `ScrollAreaOutput` from `Table::body` [#4829](https://github.com/emilk/egui/pull/4829) by [@frederik-uni](https://github.com/frederik-uni)
* Use `Style`'s font size in `egui_extras::syntax_highlighting` [#5090](https://github.com/emilk/egui/pull/5090) by [@lampsitter](https://github.com/lampsitter)

### 🐛 Fixed
* Make sure SVGs are crisp [#4823](https://github.com/emilk/egui/pull/4823) by [@AurevoirXavier](https://github.com/AurevoirXavier)
* Fix file mime from path (wrong feature name) [#4933](https://github.com/emilk/egui/pull/4933) by [@rustbasic](https://github.com/rustbasic)
* Fix compilation of `egui_extras` without `serde` feature [#5014](https://github.com/emilk/egui/pull/5014) by [@emilk](https://github.com/emilk)


## 0.28.1 - 2024-07-05
* Make `serde` a default (opt-out) feature of `egui_extras` [#4786](https://github.com/emilk/egui/pull/4786) by [@emilk](https://github.com/emilk)


## 0.28.0 - 2024-07-03
* Update `image` crate to 0.25 [#4160](https://github.com/emilk/egui/pull/4160) by [@emilk](https://github.com/emilk)
* Set the `sizing_pass` flag in first frame of `egui_extras::Table` [#4613](https://github.com/emilk/egui/pull/4613) by [@emilk](https://github.com/emilk)
* Make `serde` an opt-in feature [#4641](https://github.com/emilk/egui/pull/4641) by [@Dinnerbone](https://github.com/Dinnerbone)
* GIF support [#4620](https://github.com/emilk/egui/pull/4620) by [@JustFrederik](https://github.com/JustFrederik)
* Improve `egui_extras::Table` layout [#4755](https://github.com/emilk/egui/pull/4755) by [@emilk](https://github.com/emilk)
* Improve the auto-sizing of `Table` [#4756](https://github.com/emilk/egui/pull/4756) by [@emilk](https://github.com/emilk)


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
