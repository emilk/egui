# eframe: the [`egui`](https://github.com/emilk/egui) framework

[![Latest version](https://img.shields.io/crates/v/eframe.svg)](https://crates.io/crates/eframe)
[![Documentation](https://docs.rs/eframe/badge.svg)](https://docs.rs/eframe)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)


This aims to be the entry-level crate if you want to write an `egui` app.

`eframe` calls into your code (it is a framework) and supports web apps (via [`egui_web`](https://crates.io/crates/egui_web)) and native apps (via [`egui_glium`](https://crates.io/crates/egui_glium)).

`eframe` is a very thin crate that re-exports [`egui`](https://github.com/emilk/egui), [`epi`](https://github.com/emilk/egui/tree/master/epi) and thin wrappers over the backends.

To use on Linux, first run:

```
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libssl-dev
```

## Name

The _frame_ in `eframe` stands both for the frame in which your `egui` app resides and also for "framework" (`frame` is a framework, `egui` is a library).
