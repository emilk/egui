# Egui Changelog

## Unreleased

* `ui.horizontal(...)` etc returns `Response`
* Add ability to override text color with `visuals.override_text_color`
* Refactored the interface for `egui::app::App`
* Demo App: Add slider to scale all of Egui
* Windows are now constrained to the screen
* Panels: you can now create panels using `SidePanel` and `TopPanel`.
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
