# egui_glium

[![Latest version](https://img.shields.io/crates/v/egui_glium.svg)](https://crates.io/crates/egui_glium)
[![Documentation](https://docs.rs/egui_glium/badge.svg)](https://docs.rs/egui_glium)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

This crates provides bindings between [`egui`](https://github.com/emilk/egui) and [glium](https://crates.io/crates/glium) which allows you to write GUI code using egui and compile it and run it natively, cross platform.

To use on Linux, first run:

```
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libssl-dev
```

This crate depends on [`egui-winit`](https://github.com/emilk/egui/tree/master/crates/egui-winit).


## DEPRECATED - Looking for new maintainer
This crate is no longer being updated. If you are interested in keeping `egui_glium` updated, then fork it to its own repository, make a PR to the egui repo removing it, and then I will give you access to it on crates.io so you can publish new `egui_glium` crates.
