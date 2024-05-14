# Changelog for eframe
All notable changes to the `eframe` crate.

NOTE: [`egui-winit`](../egui-winit/CHANGELOG.md), [`egui_glium`](../egui_glium/CHANGELOG.md), [`egui_glow`](../egui_glow/CHANGELOG.md),and [`egui-wgpu`](../egui-wgpu/CHANGELOG.md) have their own changelogs!

This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.27.2 - 2024-04-02
#### Desktop/Native
* Fix continuous repaint on Wayland when TextEdit is focused or IME output is set [#4269](https://github.com/emilk/egui/pull/4269) (thanks [@white-axe](https://github.com/white-axe)!)
* Remove a bunch of `unwrap()` [#4285](https://github.com/emilk/egui/pull/4285)

#### Web
* Fix blurry rendering in some browsers [#4299](https://github.com/emilk/egui/pull/4299)
* Correctly identify if the browser tab has focus [#4280](https://github.com/emilk/egui/pull/4280)


## 0.27.1 - 2024-03-29
* Web: repaint if the `#hash` in the URL changes [#4261](https://github.com/emilk/egui/pull/4261)
* Add web support for `zoom_factor` [#4260](https://github.com/emilk/egui/pull/4260) (thanks [@justusdieckmann](https://github.com/justusdieckmann)!)


## 0.27.0 - 2024-03-26
* Update to document-features 0.2.8 [#4003](https://github.com/emilk/egui/pull/4003)
* Added `App::raw_input_hook` allows for the manipulation or filtering of raw input events [#4008](https://github.com/emilk/egui/pull/4008) (thanks [@varphone](https://github.com/varphone)!)

#### Desktop/Native
* Add with_taskbar to viewport builder [#3958](https://github.com/emilk/egui/pull/3958) (thanks [@AnotherNathan](https://github.com/AnotherNathan)!)
* Add `winuser` feature to `winapi` to fix unresolved import [#4037](https://github.com/emilk/egui/pull/4037) (thanks [@varphone](https://github.com/varphone)!)
* Add `get_proc_address` in CreationContext [#4145](https://github.com/emilk/egui/pull/4145) (thanks [@Chaojimengnan](https://github.com/Chaojimengnan)!)
* Don't clear modifier state on focus change [#4157](https://github.com/emilk/egui/pull/4157) (thanks [@ming08108](https://github.com/ming08108)!)
* Add x11 window type settings to viewport builder [#4175](https://github.com/emilk/egui/pull/4175) (thanks [@psethwick](https://github.com/psethwick)!)

#### Web
* Add `webgpu` feature by default to wgpu [#4124](https://github.com/emilk/egui/pull/4124) (thanks [@ctaggart](https://github.com/ctaggart)!)
* Update kb modifiers from web mouse events [#4156](https://github.com/emilk/egui/pull/4156) (thanks [@ming08108](https://github.com/ming08108)!)
* Fix crash on `request_animation_frame` when destroying web runner [#4169](https://github.com/emilk/egui/pull/4169) (thanks [@jprochazk](https://github.com/jprochazk)!)
* Fix bug parsing url query with escaped & or = [#4172](https://github.com/emilk/egui/pull/4172)
* `Location::query_map`: support repeated key [#4183](https://github.com/emilk/egui/pull/4183)


## 0.26.2 - 2024-02-14
* Add `winuser` feature to `winapi` to fix unresolved import [#4037](https://github.com/emilk/egui/pull/4037) (thanks [@varphone](https://github.com/varphone)!)


## 0.26.1 - 2024-02-11
* Fix high CPU usage on Windows when app is minimized [#3985](https://github.com/emilk/egui/pull/3985) (thanks [@rustbasic](https://github.com/rustbasic)!)
* Update to document-features 0.2.8 [#4003](https://github.com/emilk/egui/pull/4003)


## 0.26.0 - 2024-02-05
* Update `wgpu` to 0.19 [#3824](https://github.com/emilk/egui/pull/3824)
* Disable the default features of `wgpu` [#3875](https://github.com/emilk/egui/pull/3875)
* Much more accurate `cpu_usage` timing [#3913](https://github.com/emilk/egui/pull/3913)
* Update to puffin 0.19 [#3940](https://github.com/emilk/egui/pull/3940)

#### Desktop/Native
* Keep `ViewportInfo::maximized` and `minimized` up-to-date on Windows [#3831](https://github.com/emilk/egui/pull/3831) (thanks [@rustbasic](https://github.com/rustbasic)!)
* Handle `IconData::default()` without crashing [#3842](https://github.com/emilk/egui/pull/3842)
* Fix Android crash on resume [#3847](https://github.com/emilk/egui/pull/3847) [#3867](https://github.com/emilk/egui/pull/3867) (thanks [@Garoven](https://github.com/Garoven)!)
* Add `WgpuConfiguration::desired_maximum_frame_latency` [#3874](https://github.com/emilk/egui/pull/3874)
* Don't call `App::update` on minimized windows [#3877](https://github.com/emilk/egui/pull/3877) (thanks [@rustbasic](https://github.com/rustbasic)!)

#### Web
* When using `wgpu` on web, `eframe` will try to use WebGPU if available, then fall back to WebGL [#3824](https://github.com/emilk/egui/pull/3824) [#3895](https://github.com/emilk/egui/pull/3895) (thanks [@Wumpf](https://github.com/Wumpf)!)


## 0.25.0 - 2024-01-08
* If both `glow` and `wgpu` features are enabled, default to `wgpu` [#3717](https://github.com/emilk/egui/pull/3717)

#### Desktop/Native
* Update to winit 0.29 [#3649](https://github.com/emilk/egui/pull/3649) (thanks [@fornwall](https://github.com/fornwall)!)
* Make glow `Send + Sync` again [#3646](https://github.com/emilk/egui/pull/3646) (thanks [@surban](https://github.com/surban)!)
* Bug fix: framebuffer clear when using glow with multi-viewports [#3713](https://github.com/emilk/egui/pull/3713)
* Fix: Let `accesskit` process window events [#3733](https://github.com/emilk/egui/pull/3733) (thanks [@DataTriny](https://github.com/DataTriny)!)

#### Web
* Fix building the `wasm32` docs for `docs.rs` [#3757](https://github.com/emilk/egui/pull/3757)


## 0.24.1 - 2023-11-30
#### Desktop/Native
* Fix window flashing white on launch [#3631](https://github.com/emilk/egui/pull/3631) (thanks [@zeozeozeo](https://github.com/zeozeozeo)!)
* Fix windowing problems when using the `x11` feature on Linux [#3643](https://github.com/emilk/egui/pull/3643)
* Fix bugs when there are multiple monitors with different scales [#3663](https://github.com/emilk/egui/pull/3663)
* `glow` backend: clear framebuffer color before calling `App::update` [#3665](https://github.com/emilk/egui/pull/3665)

#### Web
* Fix click-to-copy on Safari [#3621](https://github.com/emilk/egui/pull/3621)
* Don't throw away frames on click/copy/cut [#3623](https://github.com/emilk/egui/pull/3623)
* Remove dependency on `tts` [#3651](https://github.com/emilk/egui/pull/3651)


## 0.24.0 - 2023-11-23
* Multiple viewports/windows [#3172](https://github.com/emilk/egui/pull/3172) (thanks [@konkitoman](https://github.com/konkitoman)!)
* Replace `eframe::Frame` commands and `WindowInfo` with egui [#3564](https://github.com/emilk/egui/pull/3564)
* Use `egui::ViewportBuilder` in `eframe::NativeOptions` [#3572](https://github.com/emilk/egui/pull/3572)
* Remove warm-starting [#3574](https://github.com/emilk/egui/pull/3574)
* Fix copy and cut on Safari [#3513](https://github.com/emilk/egui/pull/3513) (thanks [@lunixbochs](https://github.com/lunixbochs)!)
* Update puffin to 0.18 [#3600](https://github.com/emilk/egui/pull/3600)
* Update MSRV to Rust 1.72 [#3595](https://github.com/emilk/egui/pull/3595)

### Breaking changes:
Most settings in `NativeOptions` have been moved to `NativeOptions::viewport`, which uses the new `egui::ViewportBuilder`:

```diff
 let native_options = eframe::nativeOptions {
-    initial_window_size: Some(egui::vec2(320.0, 240.0)),
-    drag_and_drop_support: true,
+    viewport: egui::ViewportBuilder::default()
+        .with_inner_size([320.0, 240.0])
+        .with_drag_and_drop(true),
     ..Default::default()
 };
```

`NativeOptions::fullsize_content` has been replaced with four settings: `ViewportBuilder::with_fullsize_content_view`, `with_title_shown`, `with_titlebar_shown`, `with_titlebar_buttons_shown`

`frame.info().window_info` is gone, replaced with `ctx.input(|i| i.viewport())`.

`frame.info().native_pixels_per_point` is replaced with `ctx.input(|i| i.raw.native_pixels_per_point)`.

Most commands in `eframe::Frame` has been replaced with `egui::ViewportCommand`, so So `frame.close()` becomes `ctx.send_viewport_cmd(ViewportCommand::Close)`, etc.

`App::on_close_event` has been replaced with `ctx.input(|i| i.viewport().close_requested())` and `ctx.send_viewport_cmd(ViewportCommand::CancelClose)`.

`eframe::IconData` is now `egui::IconData`.

`eframe::IconData::try_from_png_bytes` is now `eframe::icon_data::from_png_bytes`.

`App::post_rendering` is gone. Screenshots are taken with `ctx.send_viewport_cmd(ViewportCommand::Screenshots)` and are returned in `egui::Event` which you can check with:
``` rust
ui.input(|i| {
    for event in &i.raw.events {
        if let egui::Event::Screenshot { viewport_id, image } = event {
            // handle it here
        }
    }
});
```


## 0.23.0 - 2023-09-27
* Update MSRV to Rust 1.70.0 [#3310](https://github.com/emilk/egui/pull/3310)
* Update to puffin 0.16 [#3144](https://github.com/emilk/egui/pull/3144)
* Update to `wgpu` 0.17.0 [#3170](https://github.com/emilk/egui/pull/3170) (thanks [@Aaron1011](https://github.com/Aaron1011)!)
* Improved wgpu callbacks [#3253](https://github.com/emilk/egui/pull/3253) (thanks [@Wumpf](https://github.com/Wumpf)!)
* Improve documentation of `eframe`, especially for wasm32 [#3295](https://github.com/emilk/egui/pull/3295)
* `eframe::Frame::info` returns a reference [#3301](https://github.com/emilk/egui/pull/3301) (thanks [@Barugon](https://github.com/Barugon)!)
* Move `App::persist_window` to `NativeOptions` and `App::max_size_points` to `WebOptions` [#3397](https://github.com/emilk/egui/pull/3397)

#### Desktop/Native
* Only show on-screen-keyboard and IME when editing text [#3362](https://github.com/emilk/egui/pull/3362) (thanks [@Barugon](https://github.com/Barugon)!)
* Add `eframe::storage_dir` [#3286](https://github.com/emilk/egui/pull/3286)
* Add `NativeOptions::window_builder` for more customization [#3390](https://github.com/emilk/egui/pull/3390) (thanks [@twop](https://github.com/twop)!)
* Better restore Window position on Mac when on secondary monitor [#3239](https://github.com/emilk/egui/pull/3239)
* Fix iOS support in `eframe` [#3241](https://github.com/emilk/egui/pull/3241) (thanks [@lucasmerlin](https://github.com/lucasmerlin)!)
* Speed up `eframe` state storage [#3353](https://github.com/emilk/egui/pull/3353) (thanks [@sebbert](https://github.com/sebbert)!)
* Allow users to opt-out of default `winit` features [#3228](https://github.com/emilk/egui/pull/3228)
* Expose Raw Window and Display Handles [#3073](https://github.com/emilk/egui/pull/3073) (thanks [@bash](https://github.com/bash)!)
* Use window title as fallback when app_id is not set [#3107](https://github.com/emilk/egui/pull/3107) (thanks [@jacekpoz](https://github.com/jacekpoz)!)
* Sleep a bit only when minimized [#3139](https://github.com/emilk/egui/pull/3139) (thanks [@icedrocket](https://github.com/icedrocket)!)
* Prevent text from being cleared when selected due to winit IME [#3376](https://github.com/emilk/egui/pull/3376) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Fix android app quit on resume with glow backend [#3080](https://github.com/emilk/egui/pull/3080) (thanks [@tkkcc](https://github.com/tkkcc)!)
* Fix panic with persistence without window [#3167](https://github.com/emilk/egui/pull/3167) (thanks [@sagebind](https://github.com/sagebind)!)
* Only call `run_return` twice on Windows [#3053](https://github.com/emilk/egui/pull/3053) (thanks [@pan93412](https://github.com/pan93412)!)
* Gracefully catch error saving state to disk [#3230](https://github.com/emilk/egui/pull/3230)
* Recognize numpad enter/plus/minus [#3285](https://github.com/emilk/egui/pull/3285)
* Add more puffin profile scopes to `eframe` [#3330](https://github.com/emilk/egui/pull/3330) [#3332](https://github.com/emilk/egui/pull/3332)

#### Web
* Update to wasm-bindgen 0.2.87 [#3237](https://github.com/emilk/egui/pull/3237)
* Remove `Function()` invocation from eframe text_agent to bypass "unsafe-eval" restrictions in Chrome browser extensions. [#3349](https://github.com/emilk/egui/pull/3349) (thanks [@aspect](https://github.com/aspect)!)
* Fix docs about web [#3026](https://github.com/emilk/egui/pull/3026) (thanks [@kerryeon](https://github.com/kerryeon)!)


## 0.22.0 - 2023-05-23
* Fix: `request_repaint_after` works even when called from background thread [#2939](https://github.com/emilk/egui/pull/2939)
* Clear all keys and modifies on focus change [#2857](https://github.com/emilk/egui/pull/2857) [#2933](https://github.com/emilk/egui/pull/2933)
* Remove dark-light dependency [#2929](https://github.com/emilk/egui/pull/2929)
* Replace `tracing` with `log` [#2928](https://github.com/emilk/egui/pull/2928)
* Update accesskit to 0.11 [#3012](https://github.com/emilk/egui/pull/3012)

#### Desktop/Native
* Automatically change theme when system dark/light mode changes [#2750](https://github.com/emilk/egui/pull/2750) (thanks [@bash](https://github.com/bash)!)
* Enabled wayland feature for winit when running native [#2751](https://github.com/emilk/egui/pull/2751) (thanks [@ItsEthra](https://github.com/ItsEthra)!)
* Fix eframe window position bug (pixels vs points) [#2763](https://github.com/emilk/egui/pull/2763) (thanks [@get200](https://github.com/get200)!)
* Add `Frame::request_screenshot` and `Frame::screenshot` to communicate to the backend that a screenshot of the current frame should be exposed by `Frame` during `App::post_rendering` ([#2676](https://github.com/emilk/egui/pull/2676)).
* Add `eframe::run_simple_native` * a simple API for simple apps ([#2453](https://github.com/emilk/egui/pull/2453)).
* Add `NativeOptions::app_id` which allows to set the Wayland application ID under Linux ([#1600](https://github.com/emilk/egui/issues/1600)).
* Add `NativeOptions::active` [#2813](https://github.com/emilk/egui/pull/2813) (thanks [@Dixeran](https://github.com/Dixeran)!)
* Remove `android-activity` dependency + add `Activity` backend features [#2863](https://github.com/emilk/egui/pull/2863) (thanks [@rib](https://github.com/rib)!)
* Fix bug where the eframe window is never destroyed on Linux when using `run_and_return` ([#2892](https://github.com/emilk/egui/issues/2892))
* Fix state persisting when exiting on Linux [#2895](https://github.com/emilk/egui/pull/2895) (thanks [@flukejones](https://github.com/flukejones)!)
* Allow for requesting the user's attention to the window [#2905](https://github.com/emilk/egui/pull/2905) (thanks [@TicClick](https://github.com/TicClick)!)
* Read and request window focus [#2900](https://github.com/emilk/egui/pull/2900) (thanks [@TicClick](https://github.com/TicClick)!)
* Set app icon on Mac and Windows [#2940](https://github.com/emilk/egui/pull/2940)
* Set a default icon for all eframe apps: a white `e` on black background [#2996](https://github.com/emilk/egui/pull/2996)
* Add `NativeOptions::app_id` for the persistence location [#3014](https://github.com/emilk/egui/pull/3014) and for Wayland [#3007](https://github.com/emilk/egui/pull/3007) (thanks [@thomaskrause](https://github.com/thomaskrause)!)
* capture a screenshot using `Frame::request_screenshot` [870264b](https://github.com/emilk/egui/commit/870264b00577a95d3fd9bdf36efaf87fd351de62)


#### Web
* ⚠️ BREAKING: `eframe::start_web` has been replaced with `eframe::WebRunner`, which also installs a nice panic hook (no need for `console_error_panic_hook`).
* ⚠️ BREAKING: WebGPU is now the default web renderer when using the `wgpu` feature of `eframe`. To use WebGL with `wgpu`, you need to add `wgpu = { version = "0.16.0", features = ["webgl"] }` to your own `Cargo.toml`. ([#2945](https://github.com/emilk/egui/pull/2945))
* Add `eframe::WebLogger` for redirecting `log` calls to the web console (`console.log`).
* Prefer the client width/height for the canvas parent [#2804](https://github.com/emilk/egui/pull/2804) (thanks [@samitbasu](https://github.com/samitbasu)!)
* eframe web: Persist app state to local storage when leaving site [#2927](https://github.com/emilk/egui/pull/2927)
* Better panic handling [#2942](https://github.com/emilk/egui/pull/2942) [#2992](https://github.com/emilk/egui/pull/2992)
* Update wasm-bindgen to 0.2.86 [#2995](https://github.com/emilk/egui/pull/2995)
* Properly unsubscribe from events on destroy [4d360f6](https://github.com/emilk/egui/commit/4d360f67a4ae2314fbc8b83b01b701ec8e9cea5b)


## 0.21.3 - 2023-02-15
* Fix typing the letter 'P' on web ([#2740](https://github.com/emilk/egui/pull/2740)).


## 0.21.2 - 2023-02-12
* Allow compiling `eframe` with `--no-default-features` ([#2728](https://github.com/emilk/egui/pull/2728)).


## 0.21.1 - 2023-02-12
* Fixed crash when native window position is in an invalid state, which could happen e.g. due to changes in monitor size or DPI ([#2722](https://github.com/emilk/egui/issues/2722)).


## 0.21.0 - 2023-02-08 - Update to `winit` 0.28
* ⚠️ BREAKING: `App::clear_color` now expects you to return a raw float array ([#2666](https://github.com/emilk/egui/pull/2666)).
* The `screen_reader` feature has now been renamed `web_screen_reader` and only work on web. On other platforms, use the `accesskit` feature flag instead ([#2669](https://github.com/emilk/egui/pull/2669)).

#### Desktop/Native
* `eframe::run_native` now returns a `Result` ([#2433](https://github.com/emilk/egui/pull/2433)).
* Update to `winit` 0.28, adding support for mac trackpad zoom ([#2654](https://github.com/emilk/egui/pull/2654)).
* Fix bug where the cursor could get stuck using the wrong icon.
* `NativeOptions::transparent` now works with the wgpu backend ([#2684](https://github.com/emilk/egui/pull/2684)).
* Add `Frame::set_minimized` and `set_maximized` ([#2292](https://github.com/emilk/egui/pull/2292), [#2672](https://github.com/emilk/egui/pull/2672)).
* Fixed persistence of native window position on Windows OS ([#2583](https://github.com/emilk/egui/issues/2583)).

#### Web
* Prevent ctrl-P/cmd-P from opening the print dialog ([#2598](https://github.com/emilk/egui/pull/2598)).


## 0.20.1 - 2022-12-11
* Fix [docs.rs](https://docs.rs/eframe) build ([#2420](https://github.com/emilk/egui/pull/2420)).


## 0.20.0 - 2022-12-08 - AccessKit integration and `wgpu` web support
* MSRV (Minimum Supported Rust Version) is now `1.65.0` ([#2314](https://github.com/emilk/egui/pull/2314)).
* Allow empty textures with the glow renderer.

#### Desktop/Native
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

#### Web
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

#### Desktop/Native
* Fixed clipboard on Wayland ([#1613](https://github.com/emilk/egui/pull/1613)).
* Added ability to read window position and size with `frame.info().window_info` ([#1617](https://github.com/emilk/egui/pull/1617)).
* Allow running on native without hardware accelerated rendering. Change with `NativeOptions::hardware_acceleration` ([#1681](https://github.com/emilk/egui/pull/1681), [#1693](https://github.com/emilk/egui/pull/1693)).
* Fixed window position persistence ([#1745](https://github.com/emilk/egui/pull/1745)).
* Fixed mouse cursor change on Linux ([#1747](https://github.com/emilk/egui/pull/1747)).
* Added `Frame::set_visible` ([#1808](https://github.com/emilk/egui/pull/1808)).
* Added fullscreen support ([#1866](https://github.com/emilk/egui/pull/1866)).
* You can now continue execution after closing the native desktop window ([#1889](https://github.com/emilk/egui/pull/1889)).
* `Frame::quit` has been renamed to `Frame::close` and `App::on_exit_event` is now `App::on_close_event` ([#1943](https://github.com/emilk/egui/pull/1943)).

#### Web
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

#### Desktop/Native
* Remove the `egui_glium` feature. `eframe` will now always use `egui_glow` as the native backend ([#1357](https://github.com/emilk/egui/pull/1357)).
* Change default for `NativeOptions::drag_and_drop_support` to `true` ([#1329](https://github.com/emilk/egui/pull/1329)).
* Added new `NativeOptions`: `vsync`, `multisampling`, `depth_buffer`, `stencil_buffer`.
* `dark-light` (dark mode detection) is now an opt-in feature ([#1437](https://github.com/emilk/egui/pull/1437)).
* Fixed potential scale bug when DPI scaling changes (e.g. when dragging a  window between different displays) ([#1441](https://github.com/emilk/egui/pull/1441)).
* Added new feature `puffin` to add [`puffin profiler`](https://github.com/EmbarkStudios/puffin) scopes ([#1483](https://github.com/emilk/egui/pull/1483)).
* Moved app persistence to a background thread, allowing for smoother frame rates (on native).
* Added `Frame::set_window_pos` ([#1505](https://github.com/emilk/egui/pull/1505)).

#### Web
* Use full browser width by default ([#1378](https://github.com/emilk/egui/pull/1378)).
* egui code will no longer be called after panic ([#1306](https://github.com/emilk/egui/pull/1306)).


## 0.17.0 - 2022-02-22
* Removed `Frame::alloc_texture`. Use `egui::Context::load_texture` instead ([#1110](https://github.com/emilk/egui/pull/1110)).
* Shift-scroll will now result in horizontal scrolling on all platforms ([#1136](https://github.com/emilk/egui/pull/1136)).
* Log using the `tracing` crate. Log to stdout by adding `tracing_subscriber::fmt::init();` to your `main` ([#1192](https://github.com/emilk/egui/pull/1192)).

#### Desktop/Native
* The default native backend is now `egui_glow` (instead of `egui_glium`) ([#1020](https://github.com/emilk/egui/pull/1020)).
* Automatically detect and apply dark or light mode from system ([#1045](https://github.com/emilk/egui/pull/1045)).
* Fixed horizontal scrolling direction on Linux.
* Added `App::on_exit_event` ([#1038](https://github.com/emilk/egui/pull/1038))
* Added `NativeOptions::initial_window_pos`.
* Fixed `enable_drag` for Windows OS ([#1108](https://github.com/emilk/egui/pull/1108)).

#### Web
* The default web painter is now `egui_glow` (instead of WebGL) ([#1020](https://github.com/emilk/egui/pull/1020)).
* Fixed glow failure on Chromium ([#1092](https://github.com/emilk/egui/pull/1092)).
* Updated `eframe::IntegrationInfo::web_location_hash` on `hashchange` event ([#1140](https://github.com/emilk/egui/pull/1140)).
* Expose all parts of the location/url in `frame.info().web_info` ([#1258](https://github.com/emilk/egui/pull/1258)).


## 0.16.0 - 2021-12-29
* `Frame` can now be cloned, saved, and passed to background threads ([#999](https://github.com/emilk/egui/pull/999)).
* Added `Frame::request_repaint` to replace `repaint_signal` ([#999](https://github.com/emilk/egui/pull/999)).
* Added `Frame::alloc_texture/free_texture` to replace `tex_allocator` ([#999](https://github.com/emilk/egui/pull/999)).

#### Web
* Fixed [dark rendering in WebKitGTK](https://github.com/emilk/egui/issues/794) ([#888](https://github.com/emilk/egui/pull/888/)).
* Added feature `glow` to switch to a [`glow`](https://github.com/grovesNL/glow) based painter ([#868](https://github.com/emilk/egui/pull/868)).


## 0.15.0 - 2021-10-24
* `Frame` now provides `set_window_title` to set window title dynamically ([#828](https://github.com/emilk/egui/pull/828)).
* `Frame` now provides `set_decorations` to set whether to show window decorations.
* Remove "http" feature (use https://github.com/emilk/ehttp instead!).
* Added `App::persist_native_window` and `App::persist_egui_memory` to control what gets persisted.

#### Desktop/Native
* Increase native scroll speed.
* Added new backend `egui_glow` as an alternative to `egui_glium`. Enable with `default-features = false, features = ["default_fonts", "egui_glow"]`.

#### Web
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
