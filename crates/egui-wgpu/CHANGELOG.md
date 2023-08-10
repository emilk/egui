# Changelog for egui-wgpu
All notable changes to the `egui-wgpu` integration will be noted in this file.


This file is updated upon each release.
Changes since the last release can be found by running the `scripts/generate_changelog.py` script.


## 0.22.0 - 2023-05-23
* Update to wgpu 0.16 [#2884](https://github.com/emilk/egui/pull/2884) (thanks [@niklaskorz](https://github.com/niklaskorz)!)
* Device configuration is now dependent on adapter [#2951](https://github.com/emilk/egui/pull/2951) (thanks [@Wumpf](https://github.com/Wumpf)!)
* Expose `wgpu::Adapter` via `RenderState` [#2954](https://github.com/emilk/egui/pull/2954) (thanks [@Wumpf](https://github.com/Wumpf)!)
* Add `read_screen_rgba` to the egui-wgpu `Painter`, to allow for capturing the current frame when using wgpu. Used in conjunction with `Frame::request_screenshot` [#2676](https://github.com/emilk/egui/pull/2676)
* Improve performance of `update_buffers` [#2820](https://github.com/emilk/egui/pull/2820) (thanks [@Wumpf](https://github.com/Wumpf)!)
* Added support for multisampling (MSAA) [#2878](https://github.com/emilk/egui/pull/2878) (thanks [@PPakalns](https://github.com/PPakalns)!)


## 0.21.0 - 2023-02-08
* Update to `wgpu` 0.15 ([#2629](https://github.com/emilk/egui/pull/2629))
* Return `Err` instead of panic if we can't find a device ([#2428](https://github.com/emilk/egui/pull/2428)).
* `winit::Painter::set_window` is now `async` ([#2434](https://github.com/emilk/egui/pull/2434)).
* `egui-wgpu` now only depends on `epaint` instead of the entire `egui` ([#2438](https://github.com/emilk/egui/pull/2438)).
* `winit::Painter` now supports transparent backbuffer ([#2684](https://github.com/emilk/egui/pull/2684)).


## 0.20.0 - 2022-12-08 - web support
* Renamed `RenderPass` to `Renderer`.
* Renamed `RenderPass::execute` to `RenderPass::render`.
* Renamed `RenderPass::execute_with_renderpass` to `Renderer::render` (replacing existing `Renderer::render`)
* Reexported `Renderer`.
* You can now use `egui-wgpu` on web, using WebGL ([#2107](https://github.com/emilk/egui/pull/2107)).
* `Renderer` no longer handles pass creation and depth buffer creation ([#2136](https://github.com/emilk/egui/pull/2136))
* `PrepareCallback` now passes `wgpu::CommandEncoder` ([#2136](https://github.com/emilk/egui/pull/2136))
* `PrepareCallback` can now returns `wgpu::CommandBuffer` that are bundled into a single `wgpu::Queue::submit` call ([#2230](https://github.com/emilk/egui/pull/2230))
* Only a single vertex & index buffer is now created and resized when necessary (previously, vertex/index buffers were allocated for every mesh) ([#2148](https://github.com/emilk/egui/pull/2148)).
* `Renderer::update_texture` no longer creates a new `wgpu::Sampler` with every new texture ([#2198](https://github.com/emilk/egui/pull/2198))
* `Painter`'s instance/device/adapter/surface creation is now configurable via `WgpuConfiguration` ([#2207](https://github.com/emilk/egui/pull/2207))
* Fix panic on using a depth buffer ([#2316](https://github.com/emilk/egui/pull/2316))


## 0.19.0 - 2022-08-20
* Enables deferred render + surface state initialization for Android ([#1634](https://github.com/emilk/egui/pull/1634)).
* Make `RenderPass` `Send` and `Sync` ([#1883](https://github.com/emilk/egui/pull/1883)).


## 0.18.0 - 2022-05-15
First published version since moving the code into the `egui` repository from <https://github.com/LU15W1R7H/eww>.
