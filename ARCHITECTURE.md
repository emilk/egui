# Arcitecture
This document describes how the crates that make up egui are all connected.

Also see [`CONTRIBUTING.md`](https://github.com/emilk/egui/blob/master/CONTRIBUTING.md) for what to do before opening a PR.


## Crate overview
The crates in this repository are: `egui, emath, epaint, egui_extras, epi, egui-winit, egui_web, egui_glium, egui_glow, egui_demo_lib, egui_demo_app`.

### `egui`: The main GUI library.
Example code: `if ui.button("Click me").clicked() { … }`
This is the crate where the bulk of the code is at. `egui` depends only on `emath` and `epaint`.

### `emath`: minimal 2D math library
Examples: `Vec2, Pos2, Rect, lerp, remap`

### `epaint`
2d shapes and text that can be turned into textured triangles.

Example: `Shape::Circle { center, radius, fill, stroke }`

Depends on `emath`, [`ab_glyph`](https://crates.io/crates/ab_glyph), [`atomic_refcell`](https://crates.io/crates/atomic_refcell), [`ahash`](https://crates.io/crates/ahash).

### `egui_extras`
This adds additional features on top of `egui`.

### `epi`
Depends only on `egui`.
Adds a thin application level wrapper around `egui` for hosting an `egui` app inside of `eframe`.

### `egui-winit`
This crates provides bindings between [`egui`](https://github.com/emilk/egui) and [winit](https://crates.io/crates/winit).

The library translates winit events to egui, handled copy/paste, updates the cursor, open links clicked in egui, etc.

### `egui_web`
Puts an egui app inside the web browser by compiling to WASM and binding to the web browser with [`js-sys`](https://crates.io/crates/js-sys) and [`wasm-bindgen`](https://crates.io/crates/wasm-bindgen). Paints the triangles that egui outputs using WebGL.

### `egui_glium`
Puts an egui app inside a native window on your laptop. Paints the triangles that egui outputs using [glium](https://github.com/glium/glium).

### `egui_glow`
Puts an egui app inside a native window on your laptop. Paints the triangles that egui outputs using [glow](https://github.com/grovesNL/glow).

### `eframe`
A wrapper around `egui_web` + `egui_glium`, so you can compile the same app for either web or native.

The demo that you can see at <https://www.egui.rs> is using `eframe` to host the `egui`. The demo code is found in:

### `egui_demo_lib`
Depends on `egui` + `epi`.
This contains a bunch of uses of `egui` and looks like the ui code you would write for an `egui` app.

### `egui_demo_app`
Thin wrapper around `egui_demo_lib` so we can compile it to a web site or a native app executable.
Depends on `egui_demo_lib` + `eframe`.

### Other integrations

There are also many great integrations for game engines such as `bevy` and `miniquad` which you can find at <https://github.com/emilk/egui#integrations>.
