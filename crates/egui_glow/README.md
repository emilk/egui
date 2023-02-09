# egui_glow

[![Latest version](https://img.shields.io/crates/v/egui_glow.svg)](https://crates.io/crates/egui_glow)
[![Documentation](https://docs.rs/egui_glow/badge.svg)](https://docs.rs/egui_glow)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

This crates provides bindings between [`egui`](https://github.com/emilk/egui) and [glow](https://crates.io/crates/glow) which allows you to:
* Render egui using glow on both native and web.
* Write cross platform native egui apps (with the `winit` feature).

To write web apps using `glow` you can use [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe) (which uses `egui_glow` for rendering).

To use on Linux, first run:

```
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

This crate optionally depends on [`egui-winit`](https://github.com/emilk/egui/tree/master/crates/egui-winit).

Text the example with:

``` sh
cargo run -p egui_glow --example pure_glow --features=winit,egui/default_fonts
```
