# eframe: the [`egui`](https://github.com/emilk/egui) framework

[![Latest version](https://img.shields.io/crates/v/eframe.svg)](https://crates.io/crates/eframe)
[![Documentation](https://docs.rs/eframe/badge.svg)](https://docs.rs/eframe)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

`eframe` is the official framework library for writing apps using [`egui`](https://github.com/emilk/egui). The app can be compiled both to run natively (cross platform) or be compiled to a web app (using WASM).

To get started, see the [examples](https://github.com/emilk/egui/tree/master/examples).
To learn how to set up `eframe` for web and native, go to <https://github.com/emilk/eframe_template/> and follow the instructions there!

There is also a tutorial video at <https://www.youtube.com/watch?v=NtUkr_z7l84>.

For how to use `egui`, see [the egui docs](https://docs.rs/egui).

---

`eframe` uses [`egui_glow`](https://github.com/emilk/egui/tree/master/crates/egui_glow) for rendering, and on native it uses [`egui-winit`](https://github.com/emilk/egui/tree/master/crates/egui-winit).

To use on Linux, first run:

```
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

You need to either use `edition = "2021"`, or set `resolver = "2"` in the `[workspace]` section of your to-level `Cargo.toml`. See [this link](https://doc.rust-lang.org/edition-guide/rust-2021/default-cargo-resolver.html) for more info.

You can opt-in to the using [`egui_wgpu`](https://github.com/emilk/egui/tree/master/crates/egui_wgpu) for rendering by enabling the `wgpu` feature and setting `NativeOptions::renderer` to `Renderer::Wgpu`.


## Alternatives
`eframe` is not the only way to write an app using `egui`! You can also try [`egui-miniquad`](https://github.com/not-fl3/egui-miniquad), [`bevy_egui`](https://github.com/mvlabat/bevy_egui), [`egui_sdl2_gl`](https://github.com/ArjunNair/egui_sdl2_gl), and others.

You can also use `egui_glow` and [`winit`](https://github.com/rust-windowing/winit) to build your own app as demonstrated in <https://github.com/emilk/egui/blob/master/crates/egui_glow/examples/pure_glow.rs>.


## Problems with running egui on the web
`eframe` uses WebGL (via [`glow`](https://crates.io/crates/glow)) and WASM, and almost nothing else from the web tech stack. This has some benefits, but also produces some challenges and serious downsides.

* Rendering: Getting pixel-perfect rendering right on the web is very difficult.
* Search: you cannot search an egui web page like you would a normal web page.
* Bringing up an on-screen keyboard on mobile: there is no JS function to do this, so `eframe` fakes it by adding some invisible DOM elements. It doesn't always work.
* Mobile text editing is not as good as for a normal web app.
* Accessibility: There is an experimental screen reader for `eframe`, but it has to be enabled explicitly. There is no JS function to ask "Does the user want a screen reader?" (and there should probably not be such a function, due to user tracking/integrity concerns).
* No integration with browser settings for colors and fonts.

In many ways, `eframe` is trying to make the browser do something it wasn't designed to do (though there are many things browser vendors could do to improve how well libraries like egui work).

The suggested use for `eframe` are for web apps where performance and responsiveness are more important than accessibility and mobile text editing.


## Companion crates
Not all rust crates work when compiled to WASM, but here are some useful crates have been designed to work well both natively and as WASM:

* Audio: [`cpal`](https://github.com/RustAudio/cpal).
* HTTP client: [`ehttp`](https://github.com/emilk/ehttp) and [`reqwest`](https://github.com/seanmonstar/reqwest).
* Time: [`chrono`](https://github.com/chronotope/chrono).
* WebSockets: [`ewebsock`](https://github.com/rerun-io/ewebsock).


## Name
The _frame_ in `eframe` stands both for the frame in which your `egui` app resides and also for "framework" (`frame` is a framework, `egui` is a library).
