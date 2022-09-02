# Changelog for egui_glium
All notable changes to the `egui_glium` integration will be noted in this file.


## Unreleased


## 0.19.0 - 2022-08-20
* MSRV (Minimum Supported Rust Version) is now `1.61.0` ([#1846](https://github.com/emilk/egui/pull/1846)).


## 0.18.0 - 2022-04-30
* Remove "epi" feature ([#1361](https://github.com/emilk/egui/pull/1361)).
* Remove need for `trait epi::NativeTexture` to use the `fn register_native_texture/replace_native_texture` ([#1361](https://github.com/emilk/egui/pull/1361)).
* MSRV (Minimum Supported Rust Version) is now `1.60.0` ([#1467](https://github.com/emilk/egui/pull/1467)).


## 0.17.0 - 2022-02-22
* `EguiGlium::run` no longer returns the shapes to paint, but stores them internally until you call `EguiGlium::paint` ([#1110](https://github.com/emilk/egui/pull/1110)).
* Optimize the painter and texture uploading ([#1110](https://github.com/emilk/egui/pull/1110)).
* Automatically detect and apply dark or light mode from system ([#1045](https://github.com/emilk/egui/pull/1045)).


## 0.16.0 - 2021-12-29
* Simplified `EguiGlium` interface ([#871](https://github.com/emilk/egui/pull/871)).
* Removed `EguiGlium::is_quit_event` ([#881](https://github.com/emilk/egui/pull/881)).
* Updated `glium` to 0.31 ([#930](https://github.com/emilk/egui/pull/930)).
* Changed the `Painter` interface slightly ([#999](https://github.com/emilk/egui/pull/999)).


## 0.15.0 - 2021-10-24
* Remove "http" feature (use https://github.com/emilk/ehttp instead!).
* Implement `epi::NativeTexture` trait for the glium painter.
* Deprecate 'Painter::register_glium_texture'.
* Increase scroll speed.
* Restore window position on startup without flickering.
* A lot of the code has been moved to the new library [`egui-winit`](https://github.com/emilk/egui/tree/master/crates/egui-winit).
* Fixed reactive mode on windows.


## 0.14.0 - 2021-08-24
* Fixed native file dialogs hanging (eg. when using [`rfd`](https://github.com/PolyMeilex/rfd)).
* Implement drag-and-dropping files into the application.
* [Fix minimize on Windows](https://github.com/emilk/egui/issues/518).
* Change `drag_and_drop_support` to `false` by default (Windows only). See <https://github.com/emilk/egui/issues/598>.
* Don't restore window position on Windows, because the position would sometimes be invalid.


## 0.13.1 - 2021-06-24
* Fixed `http` feature flag and docs


## 0.13.0 - 2021-06-24
* Added `EguiGlium::is_quit_event` to replace `control_flow` arguemnt to `EguiGlium::on_event`.
* [Fix modifier key for zoom with mouse wheel on Mac](https://github.com/emilk/egui/issues/401)
* [Fix stuck modifier keys](https://github.com/emilk/egui/pull/479)


## 0.12.0 - 2021-05-10
* Simplify usage with a new `EguiGlium` wrapper type.


## 0.11.0 - 2021-04-05
* [Position IME candidate window next to text cursor](https://github.com/emilk/egui/pull/258).
* [Register your own glium textures](https://github.com/emilk/egui/pull/226).
* [Fix cursor icon flickering on Windows(https://github.com/emilk/egui/pull/218).


## 0.10.0 - 2021-02-28
* [Add shaders for GLSL 1.2, GLSL ES 1.0 and 3.0](https://github.com/emilk/egui/pull/187)
  - now `egui` works well on old hardware which supports OpenGL 2.1 only like Raspberry Pi 1 and Zero.


## 0.9.0 - 2021-02-07
* Nothing new


## 0.8.0 - 2021-01-17
* Fixed a bug where key releases weren't sent to egui
* Fixed `set_window_size` for non-native `pixels_per_point`.


## 0.7.0 - 2021-01-04
* `http` `persistence` and `time` are now optional (and opt-in) features.


## 0.6.0 - 2020-12-26
### Added ⭐
* `egui_glium` will auto-save your app state every 30 seconds.
* `egui_glium` can now set windows as fixed size (e.g. the user can't resize the window). See `egui::App::is_resizable()`.

### Changed 🔧
* `egui_glium` will now save you app state to [a better directory](https://docs.rs/directories-next/2.0.0/directories_next/struct.ProjectDirs.html#method.data_dir).
* `egui_glium::run`: the parameter `app` now has signature `Box<dyn App>` (you need to add `Box::new(app)` to your code).
* Window title is now passed via the `trait` function `egui::App::name()`.

### Fixed 🐛
* Serialize window size in logical points instead of physical pixels.
* Window position is now restored on restart.


## 0.5.0 - 2020-12-13
* FileStorage::from_path now takes `Into<Path>` instead of `String`


## 0.4.0 - 2020-11-28
Started changelog. Features:

* Input
* Painting
* Clipboard handling
* Open URL:s
* Simple JSON-backed storage
