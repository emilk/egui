# Changelog for egui_web

All notable changes to the `egui_web` integration will be noted in this file.


## Unreleased

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
