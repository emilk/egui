# Changelog for egui-ios
All notable changes to the `egui-ios` crate will be noted in this file.

This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.33.3 - 2025-12-11
### ⭐ Added
* Initial release of `egui-ios` crate
* `InputEvent` - FFI type for touch, keyboard, and lifecycle events from Swift to egui
* `OutputState` - FFI type for cursor, keyboard, and IME state from egui to Swift
* `CursorIcon` - Cursor icons mapped to iOS equivalents
* Swift bindings generated via swift-bridge
