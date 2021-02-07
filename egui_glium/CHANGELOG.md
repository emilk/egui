# Changelog for egui_glium

All notable changes to the `egui_glium` integration will be noted in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).


## Unreleased


## 0.9.0 - 2021-02-07

* Nothing new


## 0.8.0 - 2021-01-17

### Fixed 🐛

* Fix a bug where key releases weren't sent to egui
* Fix `set_window_size` for non-native `pixels_per_point`.


## 0.7.0 - 2021-01-04

### Changed

* `http` `persistence` and `time` are now optional (and opt-in) features.


## 0.6.0 - 2020-12-26

### Added

* `egui_glium` will auto-save your app state every 30 seconds.
* `egui_glium` can now set windows as fixed size (e.g. the user can't resize the window). See `egui::App::is_resizable()`.

### Changed

* `egui_glium` will now save you app state to [a better directory](https://docs.rs/directories-next/2.0.0/directories_next/struct.ProjectDirs.html#method.data_dir).
* `egui_glium::run`: the parameter `app` now has signature `Box<dyn App>` (you need to add `Box::new(app)` to your code).
* Window title is now passed via the `trait` function `egui::App::name()`.

### Fixed 🐛

* Serialize window size in logical points instead of physical pixels.
* Window position is now restored on restart.

## 0.5.0 - 2020-12-13

### Changed

* FileStorage::from_path now takes `Into<Path>` instead of `String`


## 0.4.0 - 2020-11-28

Started changelog. Features:

* Input
* Painting
* Clipboard handling
* Open URL:s
* Simple JSON-backed storage
