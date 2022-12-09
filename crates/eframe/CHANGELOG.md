# Changelog for eframe
All notable changes to the `eframe` crate.

NOTE: [`egui-winit`](../egui-winit/CHANGELOG.md), [`egui_glium`](../egui_glium/CHANGELOG.md), [`egui_glow`](../egui_glow/CHANGELOG.md),and [`egui-wgpu`](../egui-wgpu/CHANGELOG.md) have their own changelogs!


## Unreleased
* Fix docs.rs build ([#2420](https://github.com/emilk/egui/pull/2420)).


## 0.20.0 - 2022-12-08 - AccessKit integration and `wgpu` web support
* MSRV (Minimum Supported Rust Version) is now `1.65.0` ([#2314](https://github.com/emilk/egui/pull/2314)).
* Allow empty textures with the glow renderer.

#### Desktop/Native:
* Don't repaint when just moving window ([#1980](https://github.com/emilk/egui/pull/1980)).
* Added `NativeOptions::event_loop_builder` hook for apps to change platform specific event loop options ([#1952](https://github.com/emilk/egui/pull/1952)).
* Enabled deferred render state initialization to support Android ([#1952](https://github.com/emilk/egui/pull/1952)).
* Added `shader_version` to `NativeOptions` for cross compiling support on different target OpenGL | ES versions (on native `glow` renderer only) ([#1993](https://github.com/emilk/egui/pull/1993)).
* Fix: app state is now saved when user presses Cmd-Q on Mac ([#2013](https://github.com/emilk/egui/pull/2013)).
* Added `center` to `NativeOptions` and `monitor_size` to `WindowInfo` on desktop ([#2035](https://github.com/emilk/egui/pull/2035)).
* Improve IME support ([#2046](https://github.com/emilk/egui/pull/2046)).
* Added mouse-passthrough option ([#2080](https://github.com/emilk/egui/pull/2080)).
* Added `NativeOptions::fullsize_content` option on Mac to build titlebar-less windows with floating window controls ([#2049](https://github.com/emilk/egui/pull/2049)).
* Wgpu device/adapter/surface creation has now various configuration options exposed via `NativeOptions/WebOptions::wgpu_options` ([#2207](https://github.com/emilk/egui/pull/2207)).
* Fix: Make sure that `native_pixels_per_point` is updated ([#2256](https://github.com/emilk/egui/pull/2256)).
* Added optional, but enabled by default, integration with [AccessKit](https://accesskit.dev/) for implementing platform accessibility APIs ([#2294](https://github.com/emilk/egui/pull/2294)).
* Fix: Less flickering on resize on Windows ([#2280](https://github.com/emilk/egui/pull/2280)).

#### Web:
* ⚠️ BREAKING: `start_web` is a now `async` ([#2107](https://github.com/emilk/egui/pull/2107)).
* Web: You can now use WebGL on top of `wgpu` by enabling the `wgpu` feature (and disabling `glow` via disabling default features) ([#2107](https://github.com/emilk/egui/pull/2107)).
* Web: Add `WebInfo::user_agent` ([#2202](https://github.com/emilk/egui/pull/2202)).
* Web: you can access your application from JS using `AppRunner::app_mut`. See `crates/egui_demo_app/src/lib.rs` ([#1886](https://github.com/emilk/egui/pull/1886)).


## 0.19.0 - 2022-08-20
* MSRV (Minimum Supported Rust Version) is now `1.61.0` ([#1846](https://github.com/emilk/egui/pull/1846)).
* Added `wgpu` rendering backed ([#1564](https://github.com/emilk/egui/pull/1564)):
  * Added features `wgpu` and `glow`.
  * Added `NativeOptions::renderer` to switch between the rendering backends.
* `egui_glow`: remove calls to `gl.get_error` in release builds to speed up rendering ([#1583](https://github.com/emilk/egui/pull/1583)).
* Added `App::post_rendering` for e.g. reading the framebuffer ([#1591](https://github.com/emilk/egui/pull/1591)).
* Use `Arc` for `glow::Context` instead of `Rc` ([#1640](https://github.com/emilk/egui/pull/1640)).
* Fixed bug where the result returned from `App::on_exit_event` would sometimes be ignored ([#1696](https://github.com/emilk/egui/pull/1696)).
* Added `NativeOptions::follow_system_theme` and `NativeOptions::default_theme` ([#1726](https://github.com/emilk/egui/pull/1726)).
* Selectively expose parts of the API based on target arch (`wasm32` or not) ([#1867](https://github.com/emilk/egui/pull/1867)).

#### Desktop/Native:
* Fixed clipboard on Wayland ([#1613](https://github.com/emilk/egui/pull/1613)).
* Added ability to read window position and size with `frame.info().window_info` ([#1617](https://github.com/emilk/egui/pull/1617)).
* Allow running on native without hardware accelerated rendering. Change with `NativeOptions::hardware_acceleration` ([#1681](https://github.com/emilk/egui/pull/1681), [#1693](https://github.com/emilk/egui/pull/1693)).
* Fixed window position persistence ([#1745](https://github.com/emilk/egui/pull/1745)).
* Fixed mouse cursor change on Linux ([#1747](https://github.com/emilk/egui/pull/1747)).
* Added `Frame::set_visible` ([#1808](https://github.com/emilk/egui/pull/1808)).
* Added fullscreen support ([#1866](https://github.com/emilk/egui/pull/1866)).
* You can now continue execution after closing the native desktop window ([#1889](https://github.com/emilk/egui/pull/1889)).
* `Frame::quit` has been renamed to `Frame::close` and `App::on_exit_event` is now `App::on_close_event` ([#1943](https://github.com/emilk/egui/pull/1943)).

#### Web:
* Added ability to stop/re-run web app from JavaScript. ⚠️ You need to update your CSS with `html, body: { height: 100%; width: 100%; }` ([#1803](https://github.com/emilk/egui/pull/1650)).
* Added `WebOptions::follow_system_theme` and `WebOptions::default_theme` ([#1726](https://github.com/emilk/egui/pull/1726)).
* Added option to select WebGL version ([#1803](https://github.com/emilk/egui/pull/1803)).


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
* Added `glow` (OpenGL) context to `Frame` ([#1425](https://github.com/emilk/egui/pull/1425)).

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
* Fixed horizontal scrolling direction on Linux.
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
* `Frame` now provides `set_window_title` to set window title dynamically ([#828](https://github.com/emilk/egui/pull/828)).
* `Frame` now provides `set_decorations` to set whether to show window decorations.
* Remove "http" feature (use https://github.com/emilk/ehttp instead!).
* Added `App::persist_native_window` and `App::persist_egui_memory` to control what gets persisted.

#### Desktop/Native:
* Increase native scroll speed.
* Added new backend `egui_glow` as an alternative to `egui_glium`. Enable with `default-features = false, features = ["default_fonts", "egui_glow"]`.

#### Web:
* Implement `eframe::NativeTexture` trait for the WebGL painter.
* Deprecate `Painter::register_webgl_texture.
* Fixed multiline paste.
* Fixed painting with non-opaque backgrounds.
* Improve text input on mobile and for IME.


## 0.14.0 - 2021-08-24
* Added dragging and dropping files into egui.
* Improve http fetch API.
* `run_native` now returns when the app is closed.
* Web: Made text thicker and less pixelated.


## 0.13.1 - 2021-06-24
* Fixed `http` feature flag and docs


## 0.13.0 - 2021-06-24
* `App::setup` now takes a `Frame` and `Storage` by argument.
* `App::load` has been removed. Implement `App::setup` instead.
* Web: Default to light visuals unless the system reports a preference for dark mode.
* Web: Improve alpha blending, making fonts look much better (especially in light mode)
* Web: Fix double-paste bug


## 0.12.0 - 2021-05-10
* Moved options out of `trait App` into new `NativeOptions`.
* Added option for `always_on_top`.
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
* [Added support for HTTP body](https://github.com/emilk/egui/pull/139).
* Web: Right-clicks will no longer open browser context menu.
* Web: Fix a bug where one couldn't select items in a combo box on a touch screen.


## 0.8.0 - 2021-01-17
* Simplify `TextureAllocator` interface.
* WebGL2 is now supported, with improved texture sampler. WebGL1 will be used as a fallback.
* Web: Slightly improved alpha-blending (work-around for non-existing linear-space blending).
* Web: Call `prevent_default` for arrow keys when entering text


## 0.7.0 - 2021-01-04
* Initial release of `eframe`
