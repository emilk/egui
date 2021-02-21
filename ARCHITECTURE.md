# Arcitecture
This document describes how the crates that make up egui are all connected.

Also see `CONTRIBUTING.md` for what to do before opening a PR.


## Crate overview
The crates in this repository are: `egui, emath, epaint, egui, epi, egui_web, egui_glium, egui_demo_lib, egui_demo_app`.

### `egui`: The main GUI library.
Example code: `if ui.button("Click me").clicked() { … }`
This is the crate where the bulk of the code is at. `egui` depends only on `emath` and `epaint`.

### `emath`: minimal 2D math library
Examples: `Vec2, Pos2, Rect, lerp, remap`

### `epaint`
2d shapes and text that can be turned into textured triangles.

Example: `Shape::Circle { center, radius, fill, stroke }`

Depends on `emath`, [`rusttype`](https://crates.io/crates/rusttype), [`atomic_refcell`](https://crates.io/crates/atomic_refcell), [`ahash`](https://crates.io/crates/ahash).

### `epi`
Depends only on `egui`.
Adds a thin application level wrapper around `egui` for hosting an `egui` app inside of `eframe`.

### `egui_web`
Puts an egui app inside the web browser by compiling to WASM and binding to the web browser with [`js-sys`](https://crates.io/crates/js-sys) and [`wasm-bindgen`](https://crates.io/crates/wasm-bindgen). Paints the triangles that egui outputs using WebGL.

### `egui_glium`
Puts an egui app inside a native window on your laptop. Paints the triangles that egui outputs using [glium](https://github.com/glium/glium).

### `eframe`
A wrapper around `egui_web` + `egui_glium`, so you can compile the same app for either web or native.

The demo that you can see at <https://emilk.github.io/egui/index.html> is using `eframe` to host the `egui`. The demo code is found in:

### `egui_demo_lib`
Depends on `egui` + `epi`.
This contains a bunch of uses of `egui` and looks like the ui code you would write for an `egui` app.


### `egui_demo_app`
Thin wrapper around `egui_demo_lib` so we can compile it to a web site or a native app executable.
Depends on `egui_demo_lib` + `eframe`.

### Other integrations

There are also many great integrations for game engines such as `bevy` and `miniquad` which you can find at <https://github.com/emilk/egui#integrations>.
