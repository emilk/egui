# Egui Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased

### Fixed 🐛

* Fixed bug where a lost widget could still retain keyboard focus.

## 0.3.0 - 2020-11-07

### Added ⭐

* Panels: you can now create panels using `SidePanel`, `TopPanel` and `CentralPanel`.
* You can now override the default Egui fonts.
* Add ability to override text color with `visuals.override_text_color`.
* The demo now includes a simple drag-and-drop example.
* The demo app now has a slider to scale all of Egui.

### Changed 🔧

* `ui.horizontal(...)` etc returns `Response`.
* Refactored the interface for `egui::app::App`.
* Windows are now constrained to the screen.
* `Context::begin_frame()` no longer returns a `Ui`. Instead put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
* `Context::end_frame()` now returns paint commands that need to be converted to triangles with `Context::tesselate()`.
* Anti-aliasing is now off by default in debug builds.

### Removed 🔥

* You can no longer throw windows.

### Fixed 🐛

* Fix a bug where some regions would slowly grow for non-integral scales (`pixels_per_point`).

## 0.2.0 - 2020-10-10

* Color picker
* Unicode characters in labels (limited by [what the default font supports](https://fonts.google.com/specimen/Comfortaa#glyphs))
* Simple drop-down combo box menu
* Logarithmic sliders
* Optimization: coarse culling in the tesselator
* CHANGED: switch argument order of `ui.checkbox` and `ui.radio`

## 0.1.4 - 2020-09-08

This is when I started the CHANGELOG.md, after almost two years of development. Better late than never.

* Widgets: label, text button, hyperlink, checkbox, radio button, slider, draggable value, text editing
* Layouts: horizontal, vertical, columns
* Text input: very basic, multiline, copy/paste
* Windows: move, resize, name, minimize and close. Automatically sized and positioned.
* Regions: resizing, vertical scrolling, collapsing headers (sections)
* Rendering: Anti-aliased rendering of lines, circles, text and convex polygons.
* Tooltips on hover
