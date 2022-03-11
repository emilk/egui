# Changelog for egui_web
All notable changes to the `egui_web` integration will be noted in this file.


## Unreleased
* egui code will no longer be called after panic ([#1306](https://github.com/emilk/egui/pull/1306))
* Remove the "webgl" feature. `egui_web` now always use `glow` (which in turn wraps WebGL) ([#1356](https://github.com/emilk/egui/pull/1356)).


## 0.17.0 - 2022-02-22
* The default painter is now glow instead of WebGL ([#1020](https://github.com/emilk/egui/pull/1020)).
* Made the WebGL painter opt-in ([#1020](https://github.com/emilk/egui/pull/1020)).
* Fixed glow failure on Chromium ([#1092](https://github.com/emilk/egui/pull/1092)).
* Shift-scroll will now result in horizontal scrolling ([#1136](https://github.com/emilk/egui/pull/1136)).
* Updated `epi::IntegrationInfo::web_location_hash` on `hashchange` event ([#1140](https://github.com/emilk/egui/pull/1140)).
* Parse and percent-decode the web location query string ([#1258](https://github.com/emilk/egui/pull/1258)).


## 0.16.0 - 2021-12-29
* Fixed [dark rendering in WebKitGTK](https://github.com/emilk/egui/issues/794) ([#888](https://github.com/emilk/egui/pull/888/)).
* Added feature `glow` to switch to a [`glow`](https://github.com/grovesNL/glow) based painter ([#868](https://github.com/emilk/egui/pull/868)).


## 0.15.0 - 2021-10-24
### Added
* Remove "http" feature (use https://github.com/emilk/ehttp instead!).
* Implement `epi::NativeTexture` trait for the WebGL painter.
* Deprecate `Painter::register_webgl_texture.

### Fixed 🐛
* Fix multiline paste.
* Fix painting with non-opaque backgrounds.
* Improve text input on mobile and for IME.


## 0.14.1 - 2021-08-28
### Fixed 🐛
* Fix alpha blending for WebGL2 and WebGL1 with sRGB support backends, now having identical results as egui_glium.
* Fix use of egui on devices with both touch and mouse.


## 0.14.0 - 2021-08-24
### Added ⭐
* Added support for dragging and dropping files into the browser window.

### Fixed 🐛
* Made text thicker and less pixelated.


## 0.13.0 - 2021-06-24
### Changed 🔧
* Default to light visuals unless the system reports a preference for dark mode.

### Fixed 🐛
* Improve alpha blending, making fonts look much better (especially in light mode)
* Fix double-paste bug


## 0.12.0 - 2021-05-10
### Fixed 🐛
* Scroll faster when scrolling with mouse wheel.


## 0.11.0 - 2021-04-05
### Added ⭐
* [Fix mobile and IME text input](https://github.com/emilk/egui/pull/253)
* Hold down a modifier key when clicking a link to open it in a new tab.

Contributors: [n2](https://github.com/n2)


## 0.10.0 - 2021-02-28
### Added ⭐
* You can control the maximum egui canvas size with `App::max_size_points`.


## 0.9.0 - 2021-02-07
### Added ⭐
* Right-clicks will no longer open browser context menu.

### Fixed 🐛
* Fix a bug where one couldn't select items in a combo box on a touch screen.


## 0.8.0 - 2021-01-17
### Added ⭐
* WebGL2 is now supported, with improved texture sampler. WebGL1 will be used as a fallback.

### Changed 🔧
* Slightly improved alpha-blending (work-around for non-existing linear-space blending).

### Fixed 🐛
* Call prevent_default for arrow keys when entering text


## 0.7.0 - 2021-01-04
### Changed 🔧
* `http` and `persistence` are now optional (and opt-in) features.

### Fixed 🐛
* egui_web now compiled without `RUSTFLAGS=--cfg=web_sys_unstable_apis`, but copy/paste won't work.


## 0.6.0 - 2020-12-26
### Added ⭐
* Auto-save of app state to local storage

### Changed 🔧
* Set a maximum canvas size to alleviate performance issues on some machines
* Simplify `egui_web::start` arguments


## 0.4.0 - 2020-11-28
### Added ⭐
* A simple HTTP fetch API (wraps `web_sys`).
* Add ability to request a repaint
* Copy/cut/paste suppoert

### Changed 🔧
* Automatic repaint every second

### Fixed 🐛
* Web browser zooming should now work as expected
* A bunch of bug fixes related to keyboard events
