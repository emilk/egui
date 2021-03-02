# Changelog for eframe

All notable changes to the `eframe` crate.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).


## Unreleased


## 0.10.0 - 2021-02-28

* [You can now set your own app icons](https://github.com/emilk/egui/pull/193).
* You can control the initial size of the native window with `App::initial_window_size`.
* You can control the maximum egui web canvas size with `App::max_size_points`.
* `Frame::tex_allocator()` no longer returns an `Option` (there is always a texture allocator).


## 0.9.0 - 2021-02-07

* [Add support for HTTP body](https://github.com/emilk/egui/pull/139).


## 0.8.0 - 2021-01-17

* Simplify `TextureAllocator` interface.


## 0.7.0 - 2021-01-04

* Initial release of `eframe`
