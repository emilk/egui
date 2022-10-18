# Changelog for egui-wgpu
All notable changes to the `egui-wgpu` integration will be noted in this file.


## Unreleased
* Renamed `RenderPass` to `Renderer`.
* Renamed `RenderPass::execute` to `RenderPass::render`.
* Renamed `RenderPass::execute_with_renderpass` to `Renderer::render` (replacing existing `Renderer::render`)
* Reexported `Renderer`.
* `Renderer` no longer handles pass creation and depth buffer creation ([#2136](https://github.com/emilk/egui/pull/2136))
* `PrepareCallback` now passes `wgpu::CommandEncoder` ([#2136](https://github.com/emilk/egui/pull/2136))
* Only a single vertex & index buffer is now created and resized when necessary (previously, vertex/index buffers were allocated for every mesh) ([#2148](https://github.com/emilk/egui/pull/2148)).

## 0.19.0 - 2022-08-20
* Enables deferred render + surface state initialization for Android ([#1634](https://github.com/emilk/egui/pull/1634)).
* Make `RenderPass` `Send` and `Sync` ([#1883](https://github.com/emilk/egui/pull/1883)).


## 0.18.0 - 2022-05-15
First published version since moving the code into the `egui` repository from <https://github.com/LU15W1R7H/eww>.
