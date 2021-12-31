# eframe: the [`egui`](https://github.com/emilk/egui) framework

[![Latest version](https://img.shields.io/crates/v/eframe.svg)](https://crates.io/crates/eframe)
[![Documentation](https://docs.rs/eframe/badge.svg)](https://docs.rs/eframe)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

`eframe` is the official framework library for writing apps using [`egui`](https://github.com/emilk/egui). The app can be compiled both to run natively (cross platform) or be compiled to a web app (using WASM).

To get started, go to <https://github.com/emilk/eframe_template/> and follow the instructions there!

You can also take a look at [the `eframe` examples folder](https://github.com/emilk/egui/tree/master/eframe/examples). There is also an excellent tutorial video at <https://www.youtube.com/watch?v=NtUkr_z7l84>.

For how to use `egui`, see [the egui docs](https://docs.rs/egui).

---

`eframe` is a very thin crate that re-exports [`egui`](https://github.com/emilk/egui) and[`epi`](https://github.com/emilk/egui/tree/master/epi) with thin wrappers over the backends.

`eframe` uses [`egui_web`](https://github.com/emilk/egui/tree/master/egui_web) for web and [`egui_glium`](https://github.com/emilk/egui/tree/master/egui_glium) or [`egui_glow`](https://github.com/emilk/egui/tree/master/egui_glow) for native.

To use on Linux, first run:

```
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libssl-dev
```


## Alternatives
The default native backend for `eframe` is currently [`egui_glow`](https://github.com/emilk/egui/tree/master/egui_glow), but you can switch to the previous [`egui_glium`](https://github.com/emilk/egui/tree/master/egui_glium) backend by putting this in your `Cargo.toml`:

``` toml
eframe = { version = "*", default-features = false, features = ["default_fonts", "egui_glium"] }
```

`eframe` is not the only way to write an app using `egui`! You can also try [`egui-miniquad`](https://github.com/not-fl3/egui-miniquad) and [`egui_sdl2_gl`](https://github.com/ArjunNair/egui_sdl2_gl).


## Companion crates
Not all rust crates work when compiled to WASM, but here are some useful crates have been designed to work well both natively and as WASM:

* Audio: [`cpal`](https://github.com/RustAudio/cpal).
* HTTP client: [`ehttp`](https://github.com/emilk/ehttp).
* Time: [`chrono`](https://github.com/chronotope/chrono).


## Name

The _frame_ in `eframe` stands both for the frame in which your `egui` app resides and also for "framework" (`frame` is a framework, `egui` is a library).
