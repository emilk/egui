# Changelog for eframe
All notable changes to the `eframe` crate.

NOTE: [`egui-winit`](../egui-winit/CHANGELOG.md), [`egui_glium`](../egui_glium/CHANGELOG.md), and [`egui_glow`](../egui_glow/CHANGELOG.md) have their own changelogs!


## Unreleased
* `egui_glow`: remove calls to `gl.get_error` in release builds to speed up rendering ([#1583](https://github.com/emilk/egui/pull/1583)).


## 0.18.0 - 2022-04-30
* MSRV (Minimum Supported Rust Version) is now `1.60.0` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Removed `eframe::epi` - everything is now in `eframe` (`eframe::App`, `eframe::Frame` etc) ([#1545](https://github.com/emilk/egui/pull/1545)).
* Removed `Frame::request_repaint` - just call `egui::Context::request_repaint` for the same effect ([#1366](https://github.com/emilk/egui/pull/1366)).
* Changed app creation/setup ([#1363](https://github.com/emilk/egui/pull/1363)):
  * Removed `App::setup` and `App::name`.
  * Provide `CreationContext` when creating app with egui context, storage, integration info and glow context.
  * Change interface of `run_native` and `start_web`.
* Added `Frame::storage()` and `Frame::storage_mut()` ([#1418](https://github.com/emilk/egui/pull/1418)).
  * You can now load/save state in `App::update`
  * Changed `App::update` to take `&mut Frame` instead of `&Frame`.
  * `Frame` is no longer `Clone` or `Sync`.
* Add `glow` (OpenGL) context to `Frame` ([#1425](https://github.com/emilk/egui/pull/1425)).

#### Desktop/Native:
* Remove the `egui_glium` feature. `eframe` will now always use `egui_glow` as the native backend ([#1357](https://github.com/emilk/egui/pull/1357)).
* Change default for `NativeOptions::drag_and_drop_support` to `true` ([#1329](https://github.com/emilk/egui/pull/1329)).
* Added new `NativeOptions`: `vsync`, `multisampling`, `depth_buffer`, `stencil_buffer`.
* `dark-light` (dark mode detection) is now an opt-in feature ([#1437](https://github.com/emilk/egui/pull/1437)).
* Fixed potential scale bug when DPI scaling changes (e.g. when dragging a  window between different displays) ([#1441](https://github.com/emilk/egui/pull/1441)).
* Added new feature `puffin` to add [`puffin profiler`](https://github.com/EmbarkStudios/puffin) scopes ([#1483](https://github.com/emilk/egui/pull/1483)).
* Moved app persistence to a background thread, allowing for smoother frame rates (on native).
* Added `Frame::set_window_pos` ([#1505](https://github.com/emilk/egui/pull/1505)).

#### Web:
* Use full browser width by default ([#1378](https://github.com/emilk/egui/pull/1378)).
* egui code will no longer be called after panic ([#1306](https://github.com/emilk/egui/pull/1306)).


## 0.17.0 - 2022-02-22
* Removed `Frame::alloc_texture`. Use `egui::Context::load_texture` instead ([#1110](https://github.com/emilk/egui/pull/1110)).
* Shift-scroll will now result in horizontal scrolling on all platforms ([#1136](https://github.com/emilk/egui/pull/1136)).
* Log using the `tracing` crate. Log to stdout by adding `tracing_subscriber::fmt::init();` to your `main` ([#1192](https://github.com/emilk/egui/pull/1192)).

#### Desktop/Native:
* The default native backend is now `egui_glow` (instead of `egui_glium`) ([#1020](https://github.com/emilk/egui/pull/1020)).
* Automatically detect and apply dark or light mode from system ([#1045](https://github.com/emilk/egui/pull/1045)).
* Fix horizontal scrolling direction on Linux.
* Added `App::on_exit_event` ([#1038](https://github.com/emilk/egui/pull/1038))
* Added `NativeOptions::initial_window_pos`.
* Fixed `enable_drag` for Windows OS ([#1108](https://github.com/emilk/egui/pull/1108)).

#### Web:
* The default web painter is now `egui_glow` (instead of WebGL) ([#1020](https://github.com/emilk/egui/pull/1020)).
* Fixed glow failure on Chromium ([#1092](https://github.com/emilk/egui/pull/1092)).
* Updated `eframe::IntegrationInfo::web_location_hash` on `hashchange` event ([#1140](https://github.com/emilk/egui/pull/1140)).
* Expose all parts of the location/url in `frame.info().web_info` ([#1258](https://github.com/emilk/egui/pull/1258)).


## 0.16.0 - 2021-12-29
* `Frame` can now be cloned, saved, and passed to background threads ([#999](https://github.com/emilk/egui/pull/999)).
* Added `Frame::request_repaint` to replace `repaint_signal` ([#999](https://github.com/emilk/egui/pull/999)).
* Added `Frame::alloc_texture/free_texture` to replace `tex_allocator` ([#999](https://github.com/emilk/egui/pull/999)).

#### Web:
* Fixed [dark rendering in WebKitGTK](https://github.com/emilk/egui/issues/794) ([#888](https://github.com/emilk/egui/pull/888/)).
* Added feature `glow` to switch to a [`glow`](https://github.com/grovesNL/glow) based painter ([#868](https://github.com/emilk/egui/pull/868)).


## 0.15.0 - 2021-10-24
* `Frame` now provides `set_window_title` to set window title dynamically
* `Frame` now provides `set_decorations` to set whether to show window decorations.
* Remove "http" feature (use https://github.com/emilk/ehttp instead!).
* Add `App::persist_native_window` and `App::persist_egui_memory` to control what gets persisted.

#### Desktop/Native:
* Increase native scroll speed.
* Add new backend `egui_glow` as an alternative to `egui_glium`. Enable with `default-features = false, features = ["default_fonts", "egui_glow"]`.

#### Web:
* Implement `eframe::NativeTexture` trait for the WebGL painter.
* Deprecate `Painter::register_webgl_texture.
* Fix multiline paste.
* Fix painting with non-opaque backgrounds.
* Improve text input on mobile and for IME.


## 0.14.0 - 2021-08-24
* Add dragging and dropping files into egui.
* Improve http fetch API.
* `run_native` now returns when the app is closed.
* Web: Made text thicker and less pixelated.


## 0.13.1 - 2021-06-24
* Fix `http` feature flag and docs


## 0.13.0 - 2021-06-24
* `App::setup` now takes a `Frame` and `Storage` by argument.
* `App::load` has been removed. Implement `App::setup` instead.
* Web: Default to light visuals unless the system reports a preference for dark mode.
* Web: Improve alpha blending, making fonts look much better (especially in light mode)
* Web: Fix double-paste bug


## 0.12.0 - 2021-05-10
* Moved options out of `trait App` into new `NativeOptions`.
* Add option for `always_on_top`.
* Web: Scroll faster when scrolling with mouse wheel.


## 0.11.0 - 2021-04-05
* You can now turn your window transparent with the `App::transparent` option.
* You can now disable window decorations with the `App::decorated` option.
* Web: [Fix mobile and IME text input](https://github.com/emilk/egui/pull/253)
* Web: Hold down a modifier key when clicking a link to open it in a new tab.

Contributors: [n2](https://github.com/n2)


## 0.10.0 - 2021-02-28
* [You can now set your own app icons](https://github.com/emilk/egui/pull/193).
* You can control the initial size of the native window with `App::initial_window_size`.
* You can control the maximum egui web canvas size with `App::max_size_points`.
* `Frame::tex_allocator()` no longer returns an `Option` (there is always a texture allocator).


## 0.9.0 - 2021-02-07
* [Add support for HTTP body](https://github.com/emilk/egui/pull/139).
* Web: Right-clicks will no longer open browser context menu.
* Web: Fix a bug where one couldn't select items in a combo box on a touch screen.


## 0.8.0 - 2021-01-17
* Simplify `TextureAllocator` interface.
* WebGL2 is now supported, with improved texture sampler. WebGL1 will be used as a fallback.
* Web: Slightly improved alpha-blending (work-around for non-existing linear-space blending).
* Web: Call `prevent_default` for arrow keys when entering text


## 0.7.0 - 2021-01-04
* Initial release of `eframe`
