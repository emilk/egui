# Changelog for egui_web

All notable changes to the `egui_web` integration will be noted in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).


## Unreleased

## 0.6.0 - 2020-12-26

### Added ⭐

* Auto-save of app state to local storage

### Changed ⭐

* Set a maximum canvas size to alleviate performance issues on some machines
* Simplify `egui_web::start` arguments


## 0.4.0 - 2020-11-28

### Added ⭐

* A simple HTTP fetch API (wraps `web_sys`).
* Add ability to request a repaint
* Copy/cut/paste suppoert

### Changed ⭐

* Automatic repaint every second

### Fixed ⭐

* Web browser zooming should now work as expected
* A bunch of bug fixes related to keyboard events
