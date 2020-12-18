# Changelog for egui_glium

All notable changes to the `egui_glium` integration will be noted in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).


## Unreleased

### Changed

* `egui_glium::run`: the parameter `app` now has signature `Box<dyn App>` (you need to add `Box::new(app)` to your code).
* Window title is now passed via the `trait` function `egui::App::name()`

### Fixed 🐛

* Serialize window size in logical points instead of physical pixels


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
