# eframe: the [`egui`](https://github.com/emilk/egui) framework

[![Latest version](https://img.shields.io/crates/v/eframe.svg)](https://crates.io/crates/eframe)
[![Documentation](https://docs.rs/eframe/badge.svg)](https://docs.rs/eframe)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

`eframe` is the official framework library for writing apps using [`egui`](https://github.com/emilk/egui). The app can be compiled both to run natively (cross platform) or be compiled to a web app (using WASM).

To get started, see the [examples](https://github.com/emilk/egui/tree/master/examples).
To learn how to set up `eframe` for web and native, go to <https://github.com/emilk/eframe_template/> and follow the instructions there!

There is also a tutorial video at <https://www.youtube.com/watch?v=NtUkr_z7l84>.

For how to use `egui`, see [the egui docs](https://docs.rs/egui).

---

`eframe` is a very thin crate that re-exports [`egui`](https://github.com/emilk/egui) and[`epi`](https://github.com/emilk/egui/tree/master/epi) with thin wrappers over the backends.

`eframe` uses [`egui_web`](https://github.com/emilk/egui/tree/master/egui_web) for web and [`egui_glow`](https://github.com/emilk/egui/tree/master/egui_glow) for native.

To use on Linux, first run:

```
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libssl-dev
```

You need to either use `edition = "2021"`, or set `resolver = "2"` in the `[workspace`] section of your to-level `Cargo.toml`. See [this link](https://doc.rust-lang.org/edition-guide/rust-2021/default-cargo-resolver.html) for more info.


## Alternatives
`eframe` is not the only way to write an app using `egui`! You can also try [`egui-miniquad`](https://github.com/not-fl3/egui-miniquad), [`bevy_egui`](https://github.com/mvlabat/bevy_egui), [`egui_sdl2_gl`](https://github.com/ArjunNair/egui_sdl2_gl), and others.


## Companion crates
Not all rust crates work when compiled to WASM, but here are some useful crates have been designed to work well both natively and as WASM:

* Audio: [`cpal`](https://github.com/RustAudio/cpal).
* HTTP client: [`ehttp`](https://github.com/emilk/ehttp) and [`reqwest`](https://github.com/seanmonstar/reqwest).
* Time: [`chrono`](https://github.com/chronotope/chrono).
* WebSockets: [`ewebsock`](https://github.com/rerun-io/ewebsock).


## Name

The _frame_ in `eframe` stands both for the frame in which your `egui` app resides and also for "framework" (`frame` is a framework, `egui` is a library).
