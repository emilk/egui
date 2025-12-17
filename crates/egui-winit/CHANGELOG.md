# Changelog for egui-winit
All notable changes to the `egui-winit` integration will be noted in this file.

This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.33.3 - 2025-12-11
Nothing new


## 0.33.2 - 2025-11-13
* Don't enable `arboard` on iOS [#7663](https://github.com/emilk/egui/pull/7663) by [@irh](https://github.com/irh)


## 0.33.0 - 2025-10-09
### ⭐ Added
* Add rotation gesture support for trackpad sources [#7453](https://github.com/emilk/egui/pull/7453) by [@thatcomputerguy0101](https://github.com/thatcomputerguy0101)
* Add support for the safe area on iOS [#7578](https://github.com/emilk/egui/pull/7578) by [@irh](https://github.com/irh)

### 🔧 Changed
* Update MSRV from 1.86 to 1.88 [#7579](https://github.com/emilk/egui/pull/7579) by [@Wumpf](https://github.com/Wumpf)
* Create `egui_wgpu::RendererOptions` [#7601](https://github.com/emilk/egui/pull/7601) by [@emilk](https://github.com/emilk)

### 🐛 Fixed
* Fix build error in egui-winit with profiling enabled [#7557](https://github.com/emilk/egui/pull/7557) by [@torokati44](https://github.com/torokati44)
* Properly end winit event loop [#7565](https://github.com/emilk/egui/pull/7565) by [@tye-exe](https://github.com/tye-exe)
* Fix eframe window not being focused on mac on startup [#7593](https://github.com/emilk/egui/pull/7593) by [@emilk](https://github.com/emilk)


## 0.32.3 - 2025-09-12
Nothing new


## 0.32.2 - 2025-09-04
Nothing new


## 0.32.1 - 2025-08-15
* Update to winit 0.30.12 [#7420](https://github.com/emilk/egui/pull/7420) by [@emilk](https://github.com/emilk)


## 0.32.0 - 2025-07-10
* Mark all keys as released if the app loses focus [#5743](https://github.com/emilk/egui/pull/5743) by [@emilk](https://github.com/emilk)
* Fix text input on Android [#5759](https://github.com/emilk/egui/pull/5759) by [@StratusFearMe21](https://github.com/StratusFearMe21)
* Add macOS-specific `has_shadow` and `with_has_shadow` to ViewportBuilder [#6850](https://github.com/emilk/egui/pull/6850) by [@gaelanmcmillan](https://github.com/gaelanmcmillan)
* Support for back-button on Android [#7073](https://github.com/emilk/egui/pull/7073) by [@ardocrat](https://github.com/ardocrat)
* Fix incorrect window sizes for non-resizable windows on Wayland [#7103](https://github.com/emilk/egui/pull/7103) by [@GoldsteinE](https://github.com/GoldsteinE)


## 0.31.1 - 2025-03-05
Nothing new


## 0.31.0 - 2025-02-04
* Re-enable IME support on Linux [#5198](https://github.com/emilk/egui/pull/5198) by [@YgorSouza](https://github.com/YgorSouza)
* Update to winit 0.30.7 [#5516](https://github.com/emilk/egui/pull/5516) by [@emilk](https://github.com/emilk)


## 0.30.0 - 2024-12-16
* iOS: Support putting UI next to the dynamic island [#5211](https://github.com/emilk/egui/pull/5211) by [@frederik-uni](https://github.com/frederik-uni)
* Remove implicit `accesskit_winit` feature [#5316](https://github.com/emilk/egui/pull/5316) by [@waywardmonkeys](https://github.com/waywardmonkeys)


## 0.29.1 - 2024-10-01 - Fix backspace/arrow keys on X11
* Linux: Disable IME to fix backspace/arrow keys [#5188](https://github.com/emilk/egui/pull/5188) by [@emilk](https://github.com/emilk)


## 0.29.0 - 2024-09-26 - `winit` 0.30
* Upgrade to `winit` 0.30 [#4849](https://github.com/emilk/egui/pull/4849) [#4939](https://github.com/emilk/egui/pull/4939) by [@ArthurBrussee](https://github.com/ArthurBrussee)
* Fix: Backspace not working after IME input [#4912](https://github.com/emilk/egui/pull/4912) by [@rustbasic](https://github.com/rustbasic)


## 0.28.1 - 2024-07-05
Nothing new


## 0.28.0 - 2024-07-03
* Update `webbrowser` to `v1.0.0` [#4394](https://github.com/emilk/egui/pull/4394) by [@torokati44](https://github.com/torokati44)
* Emit physical key presses when a non-Latin layout is active [#4461](https://github.com/emilk/egui/pull/4461) by [@TicClick](https://github.com/TicClick)
* IME for chinese [#4436](https://github.com/emilk/egui/pull/4436) by [@rustbasic](https://github.com/rustbasic)
* Fix: Window position creeps between executions on scaled monitors [#4443](https://github.com/emilk/egui/pull/4443) by [@avery-radmacher](https://github.com/avery-radmacher)
* Ignore synthetic key presses [#4514](https://github.com/emilk/egui/pull/4514) by [@hut](https://github.com/hut)


## 0.27.2 - 2024-04-02
* Fix continuous repaint on Wayland when TextEdit is focused or IME output is set [#4269](https://github.com/emilk/egui/pull/4269) (thanks [@white-axe](https://github.com/white-axe)!)


## 0.27.1 - 2024-03-29
* Nothing new


## 0.27.0 - 2024-03-26
* Update memoffset to 0.9.0, arboard to 3.3.1, and remove egui_glow's needless dependency on pure_glow's deps  [#4036](https://github.com/emilk/egui/pull/4036) (thanks [@Nopey](https://github.com/Nopey)!)
* Don't clear modifier state on focus change [#4157](https://github.com/emilk/egui/pull/4157) (thanks [@ming08108](https://github.com/ming08108)!)


## 0.26.2 - 2024-02-14
* Update memoffset to 0.9.0, arboard to 3.3.1, and remove egui_glow's needless dependency on pure_glow's deps  [#4036](https://github.com/emilk/egui/pull/4036) (thanks [@Nopey](https://github.com/Nopey)!)


## 0.26.1 - 2024-02-11
* Nothing new


## 0.26.0 - 2024-02-05
* Don't consume clipboard shortcuts [#3812](https://github.com/emilk/egui/pull/3812) (thanks [@Dinnerbone](https://github.com/Dinnerbone)!)
* Make the `clipboard_text` and `allow_ime` state public [#3724](https://github.com/emilk/egui/pull/3724) (thanks [@tosti007](https://github.com/tosti007)!)


## 0.25.0 - 2024-01-08
* Update to winit 0.29 [#3649](https://github.com/emilk/egui/pull/3649) (thanks [@fornwall](https://github.com/fornwall)!)
* Fix: Let `accesskit` process window events [#3733](https://github.com/emilk/egui/pull/3733) (thanks [@DataTriny](https://github.com/DataTriny)!)
* Simplify `egui_winit::State` [#3678](https://github.com/emilk/egui/pull/3678)


## 0.24.1 - 2023-11-30
* Don't treat `WindowEvent::CloseRequested` as consumed [#3627](https://github.com/emilk/egui/pull/3627) (thanks [@Aaron1011](https://github.com/Aaron1011)!)
* Fix windowing problems when using the `x11` feature on Linux [#3643](https://github.com/emilk/egui/pull/3643)


## 0.24.0 - 2023-11-23
* Update MSRV to Rust 1.72 [#3595](https://github.com/emilk/egui/pull/3595)
* Some breaking changes required for multi-viewport support


## 0.23.0 - 2023-09-27
* Only show on-screen-keyboard and IME when editing text [#3362](https://github.com/emilk/egui/pull/3362) (thanks [@Barugon](https://github.com/Barugon)!)
* Replace `instant` with `web_time` [#3296](https://github.com/emilk/egui/pull/3296)
* Allow users to opt-out of default `winit` features [#3228](https://github.com/emilk/egui/pull/3228)
* Recognize numpad enter/plus/minus [#3285](https://github.com/emilk/egui/pull/3285)


## 0.22.0 - 2023-05-23
* Only use `wasm-bindgen` feature for `instant` when building for wasm32 [#2808](https://github.com/emilk/egui/pull/2808) (thanks [@gferon](https://github.com/gferon)!)
* Fix unsafe API of `Clipboard::new` [#2765](https://github.com/emilk/egui/pull/2765) (thanks [@dhardy](https://github.com/dhardy)!)
* Remove `android-activity` dependency + add `Activity` backend features [#2863](https://github.com/emilk/egui/pull/2863) (thanks [@rib](https://github.com/rib)!)
* Use `RawDisplayHandle` for smithay clipboard init [#2914](https://github.com/emilk/egui/pull/2914) (thanks [@lunixbochs](https://github.com/lunixbochs)!)
* Clear all keys and modifies on focus change [#2933](https://github.com/emilk/egui/pull/2933)
* Support Wasm target [#2949](https://github.com/emilk/egui/pull/2949) (thanks [@jinleili](https://github.com/jinleili)!)
* Fix unsafe API: remove `State::new_with_wayland_display`; change `Clipboard::new` to take `&EventLoopWindowTarget<T>`


## 0.21.1 - 2023-02-12
* Fixed crash when window position is in an invalid state, which could happen e.g. due to changes in monitor size or DPI ([#2722](https://github.com/emilk/egui/issues/2722)).


## 0.21.0 - 2023-02-08
* Fixed persistence of native window position on Windows OS ([#2583](https://github.com/emilk/egui/issues/2583)).
* Update to `winit` 0.28, adding support for mac trackpad zoom ([#2654](https://github.com/emilk/egui/pull/2654)).
* Remove the `screen_reader` feature. Use the `accesskit` feature flag instead ([#2669](https://github.com/emilk/egui/pull/2669)).
* Fix bug where the cursor could get stuck using the wrong icon.


## 0.20.1 - 2022-12-11
* Fix [docs.rs](https://docs.rs/egui-winit) build ([#2420](https://github.com/emilk/egui/pull/2420)).


## 0.20.0 - 2022-12-08
* The default features of the `winit` crate are not enabled if the default features of `egui-winit` are disabled too ([#1971](https://github.com/emilk/egui/pull/1971)).
* Added new feature `wayland` which enables Wayland support ([#1971](https://github.com/emilk/egui/pull/1971)).
* Don't repaint when just moving window ([#1980](https://github.com/emilk/egui/pull/1980)).
* Added optional integration with [AccessKit](https://accesskit.dev/) for implementing platform accessibility APIs ([#2294](https://github.com/emilk/egui/pull/2294)).

## 0.19.0 - 2022-08-20
* MSRV (Minimum Supported Rust Version) is now `1.61.0` ([#1846](https://github.com/emilk/egui/pull/1846)).
* Fixed clipboard on Wayland ([#1613](https://github.com/emilk/egui/pull/1613)).
* Allow deferred render + surface state initialization for Android ([#1634](https://github.com/emilk/egui/pull/1634)).
* Fixed window position persistence ([#1745](https://github.com/emilk/egui/pull/1745)).
* Fixed mouse cursor change on Linux ([#1747](https://github.com/emilk/egui/pull/1747)).
* Use the new `RawInput::has_focus` field to indicate whether the window has the keyboard focus ([#1859](https://github.com/emilk/egui/pull/1859)).


## 0.18.0 - 2022-04-30
* Reexport `egui` crate
* MSRV (Minimum Supported Rust Version) is now `1.60.0` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Added new feature `puffin` to add [`puffin profiler`](https://github.com/EmbarkStudios/puffin) scopes ([#1483](https://github.com/emilk/egui/pull/1483)).
* Renamed the feature `convert_bytemuck` to `bytemuck` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Renamed the feature `serialize` to `serde` ([#1467](https://github.com/emilk/egui/pull/1467)).
* Removed the features `dark-light` and `persistence` ([#1542](https://github.com/emilk/egui/pull/1542)).


## 0.17.0 - 2022-02-22
* Fixed horizontal scrolling direction on Linux.
* Replaced `std::time::Instant` with `instant::Instant` for WebAssembly compatibility ([#1023](https://github.com/emilk/egui/pull/1023))
* Automatically detect and apply dark or light mode from system ([#1045](https://github.com/emilk/egui/pull/1045)).
* Fixed `enable_drag` on Windows OS ([#1108](https://github.com/emilk/egui/pull/1108)).
* Shift-scroll will now result in horizontal scrolling on all platforms ([#1136](https://github.com/emilk/egui/pull/1136)).
* Require knowledge about max texture side (e.g. `GL_MAX_TEXTURE_SIZE`)) ([#1154](https://github.com/emilk/egui/pull/1154)).


## 0.16.0 - 2021-12-29
* Added helper `EpiIntegration` ([#871](https://github.com/emilk/egui/pull/871)).
* Fixed shift key getting stuck enabled with the X11 option `shift:both_capslock` enabled ([#849](https://github.com/emilk/egui/pull/849)).
* Removed `State::is_quit_event` and `State::is_quit_shortcut` ([#881](https://github.com/emilk/egui/pull/881)).
* Updated `winit` to 0.26 ([#930](https://github.com/emilk/egui/pull/930)).


## 0.15.0 - 2021-10-24
First stand-alone release. Previously part of `egui_glium`.
