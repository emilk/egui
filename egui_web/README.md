[![Latest version](https://img.shields.io/crates/v/egui_web.svg)](https://crates.io/crates/egui_web)
[![Documentation](https://docs.rs/egui_web/badge.svg)](https://docs.rs/egui_web)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

# egui_web

This crates allows you to compile GUI code written with [Egui](https://crates.io/crates/egui) to [WASM](https://en.wikipedia.org/wiki/WebAssembly) to run on a web page.

Check out [docs/index.html](https://github.com/emilk/egui/blob/master/docs/index.html), [egui_demo](https://github.com/emilk/egui/tree/master/egui_demo) and [build_web.sh](https://github.com/emilk/egui/blob/master/build_web.sh) for examples of how to set it up.

To use `egui_web`, you need to set the `RUSTFLAGS=--cfg=web_sys_unstable_apis` flag.
