# 🖌 egui: an easy-to-use GUI in pure Rust

[![Latest version](https://img.shields.io/crates/v/egui.svg)](https://crates.io/crates/egui)
[![Documentation](https://docs.rs/egui/badge.svg)](https://docs.rs/egui)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Build Status](https://github.com/emilk/egui/workflows/CI/badge.svg)](https://github.com/emilk/egui/actions?workflow=CI)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)
**dependencies**: [`rusttype`](https://crates.io/crates/rusttype) [`atomic_refcell`](https://crates.io/crates/atomic_refcell) [`ahash`](https://crates.io/crates/ahash)


egui is a simple, fast, and highly portable immediate mode GUI library for Rust. egui runs on the web, natively, and in your favorite game engine (or will soon).

egui aims to be the easiest-to-use Rust GUI libary, and the simplest way to make a web app in Rust.

egui can be used anywhere you can draw textured triangles, which means you can easily integrate it into your game engine of choice.

Sections:

* [Quick start](#quick-start)
* [Demo](#demo)
* [Goals](#goals)
* [Who is egui for?](#who-is-egui-for)
* [State / features](#state)
* [How it works](#how-it-works)
* [Integrations](#integrations)
* [Other](#other)

## Quick start

If you just want to write a GUI application in Rust (for the web or for native), go to <https://github.com/emilk/egui_template/> and follow the instructions there!

If you want to integrate egui into an existing engine, go to the [Integrations](#integrations) section.

If you have questions, use [Discussions](https://github.com/emilk/egui/discussions). If you want to contribute to egui, please read the [Contributing Guidelines](https://github.com/emilk/egui/blob/master/CONTRIBUTING.md)

## Demo

[Click to run egui web demo](https://emilk.github.io/egui/index.html) (works in any browser with WASM and WebGL support).

To test the demo app locally, run `cargo run --release -p egui_demo_app`.

The native backend is currently using [`glium`](https://github.com/glium/glium) ([though there are plans to change that](https://github.com/emilk/egui/issues/93)) and should work out-of-the-box on Mac and Windows, but on Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`

**NOTE**: egui itself is completely platform agnostic.

### Example

``` rust
ui.heading("My egui Application");
ui.horizontal(|ui| {
    ui.label("Your name: ");
    ui.text_edit_singleline(&mut name);
});
ui.add(egui::Slider::u32(&mut age, 0..=120).text("age"));
if ui.button("Click each year").clicked() {
    age += 1;
}
ui.label(format!("Hello '{}', age {}", name, age));
```

<img src="media/demo-2021-01-17.gif">

## Goals

* The easiest to use GUI libary
* Responsive: target 60 Hz in debug build
* Friendly: difficult to make mistakes, and shouldn't panic
* Portable: the same code works on the web and as a native app
* Easy to integrate into any environment
* A simple 2D graphics API for custom painting ([`epaint`](https://docs.rs/epaint)).
* No callbacks
* Pure immediate mode
* Extensible: [easy to write your own widgets for egui](https://github.com/emilk/egui/blob/master/egui_demo_lib/src/apps/demo/toggle_switch.rs)
* Modular: You should be able to use small parts of egui and combine them in new ways
* Safe: there is no `unsafe` code in egui
* Minimal dependencies: [`rusttype`](https://crates.io/crates/rusttype), [`atomic_refcell`](https://crates.io/crates/atomic_refcell) and [`ahash`](https://crates.io/crates/ahash).

egui is *not* a framework. egui is a library you call into, not an environment you program for.

**NOTE**: egui does not claim to have reached all these goals yet! egui is still work in progress.

### Non-goals

* Become the most powerful GUI libary
* Native looking interface
* Advanced and flexible layouts (that's fundamentally incompatible with immediate mode)

## Who is egui for?

egui aims to be the best choice when you want a simple way to create a GUI, or you want to add a GUI to a game engine.

If you are not using Rust, egui is not for you. If you want a GUI that looks native, egui is not for you. If you want something that doesn't break when you upgrade it, egui isn't for you (yet).

But if you are writing something interactive in Rust that needs a simple GUI, egui may be for you.

### egui vs Dear ImGui

The obvious alternative to egui is [`imgui-rs`](https://github.com/Gekkio/imgui-rs), the Rust wrapper around the C++ library [Dear ImGui](https://github.com/ocornut/imgui). Dear ImGui is a great library, which a lot more features and polish compared to egui. However, egui provides some benefits for Rust users:

* egui is pure Rust
* egui is easily compiled to WASM
* egui lets you use native Rust String types (`imgui-rs` forces you to use annoying macros and wrappers for zero-terminated strings)
* [Writing your own widgets in egui is simple](https://github.com/emilk/egui/blob/master/egui_demo_lib/src/apps/demo/toggle_switch.rs)

egui also tries to improve your experience in other small ways:

* Windows are automatically sized based on their contents
* Windows are automatically positioned to not overlap with each other
* Some subtle animations make egui come alive

So in summary:

* egui: pure Rust, new, exciting, work in progress
* Dear ImGui: feature rich, well tested, cumbersome Rust integration

## State

egui is in active development. It works well for what it does, but it lacks many features and the interfaces are still in flux. New releases will have breaking changes.

### Features

* Widgets: label, text button, hyperlink, checkbox, radio button, slider, draggable value, text editing, combo box, color picker
* Layouts: horizontal, vertical, columns, automatic wrapping
* Text editing: multiline, copy/paste, undo, emoji supports
* Windows: move, resize, name, minimize and close. Automatically sized and positioned.
* Regions: resizing, vertical scrolling, collapsing headers (sections)
* Rendering: Anti-aliased rendering of lines, circles, text and convex polygons.
* Tooltips on hover
* More

<img src="media/widget_gallery_0.8.0.gif" width="50%">

## How it works

Loop:

* Gather input (mouse, touches, keyboard, screen size, etc) and give it to egui
* Run application code (Immediate Mode GUI)
* Tell egui to tessellate the frame graphics to a triangle mesh
* Render the triangle mesh with your favorite graphics API (see [OpenGL example](https://github.com/emilk/egui/blob/master/egui_glium/src/painter.rs))

## Integrations

egui is build to be easy to integrate into any existing game engine or platform you are working on.
egui itself doesn't know or care on what OS it is running or how to render things to the screen - that is the job of the egui integration.
The integration needs to do two things:

* **IO**: Supply egui with input (mouse position, keyboard presses, ...) and handle egui output (cursor changes, copy-paste integration, ...).
* **Painting**: Render the textured triangles that egui outputs.

### Official

I maintain two official egui integrations:

* [egui_web](https://crates.io/crates/egui_web) for making a web app. Compiles to WASM, renders with WebGL. [Click to run the egui demo](https://emilk.github.io/egui/index.html).
* [egui_glium](https://crates.io/crates/egui_glium) for compiling native apps with [Glium](https://github.com/glium/glium).

The same code can be compiled to a native app or a web app.

### 3rd party

* [`bevy_egui`](https://github.com/mvlabat/bevy_egui) for [the Bevy game engine](https://bevyengine.org/).
* [`egui_winit_ash_vk_mem`](https://crates.io/crates/egui_winit_ash_vk_mem) for for [winit](https://github.com/rust-windowing/winit), [ash](https://github.com/MaikKlein/ash) and [vk_mem](https://github.com/gwihlidal/vk-mem-rs).
* [`wgpu`](https://crates.io/crates/wgpu) WebGPU API wrapper:
  * [egui_wgpu_backend](https://crates.io/crates/egui_wgpu_backend) with [example code](https://github.com/hasenbanck/egui_example)
  * Alternative: [egui_winit_wgpu](https://github.com/Gonkalbell/egui_winit_wgpu) (not available to crates.io)
* [egui_sdl2_gl](https://crates.io/crates/egui_sdl2_gl) for [SDL2](https://crates.io/crates/sdl2)
* [emigui-miniquad](https://github.com/not-fl3/emigui-miniquad): backend for [Miniquad](https://github.com/not-fl3/miniquad). [Web demo](https://not-fl3.github.io/miniquad-samples/emigui.html) and [demo source](https://github.com/not-fl3/good-web-game/blob/master/examples/emigui.rs).
* [egui_winit_platform](https://github.com/hasenbanck/egui_winit_platform) provides bindings between [winit](https://crates.io/crates/winit) and egui. It only provides the first half of an egui integration (IO). Painting can be done with e.g. [egui_wgpu_backend](https://crates.io/crates/egui_wgpu_backend).

### Writing your own egui integration

You need to collect [`egui::RawInput`](https://docs.rs/egui/latest/egui/struct.RawInput.html), paint [`egui::ClippedMesh`](https://docs.rs/epaint/):es and handle [`egui::Output`](https://docs.rs/egui/latest/egui/struct.Output.html). The basic structure is this:

``` rust
let mut egui_ctx = egui::Context::new();

// Game loop:
loop {
    let raw_input: egui::RawInput = my_integration.gather_input();
    egui_ctx.begin_frame(raw_input);
    my_app.ui(&mut egui_ctx); // add panels, windows and widgets to `egui_ctx` here
    let (output, shapes) = egui_ctx.end_frame();
    let clipped_meshes = egui_ctx.tessellate(shapes); // create triangles to paint
    my_integration.paint(clipped_meshes);
    my_integration.set_cursor_icon(output.cursor_icon);
    // Also see `egui::Output` for more
}
```

For a reference OpenGL backend, see [the `egui_glium` painter](https://github.com/emilk/egui/blob/master/egui_glium/src/painter.rs) or [the `egui_web` `WebGL` painter](https://github.com/emilk/egui/blob/master/egui_web/src/webgl1.rs).

### Debugging your integration

#### Things look jagged

* Turn off backface culling.

#### My text is blurry

* Make sure you set the proper `pixels_per_point` in the input to egui.
* Make sure the texture sampler is not off by half a pixel. Try nearest-neighbor sampler to check.

#### My windows are too transparent or too dark

* egui uses premultiplied alpha, so make sure your blending function is `(ONE, ONE_MINUS_SRC_ALPHA)`.
* Make sure your texture sampler is clamped (`GL_CLAMP_TO_EDGE`).
* Use an sRGBA-aware texture if available (e.g. `GL_SRGB8_ALPHA8`).
  * Otherwise: remember to decode gamma in the fragment shader.
* Decode the gamma of the incoming vertex colors in your vertex shader.
* Turn on sRGBA/linear framebuffer if available (`GL_FRAMEBUFFER_SRGB`).
  * Otherwise: gamma-encode the colors before you write them again.

## Other

### Conventions and design choices

All coordinates are in screen space coordinates, with (0, 0) in the top left corner

All coordinates are in locial "points" which may consist of many physical pixels.

All colors have premultiplied alpha.

egui uses the builder pattern for construction widgets. For instance: `ui.add(Label::new("Hello").text_color(RED));` I am not a big fan of the builder pattern (it is quite verbose both in implementation and in use) but until Rust has named, default arguments it is the best we can do. To alleviate some of the verbosity there are common-case helper functions, like `ui.label("Hello");`.

Instead of using matching `begin/end` style function calls (which can be error prone) egui prefers to use `FnOnce` closures passed to a wrapping function. Lambdas are a bit ugly though, so I'd like to find a nicer solution to this.

### Inspiration

The one and only [Dear ImGui](https://github.com/ocornut/imgui) is a great Immediate Mode GUI for C++ which works with many backends. That library revolutionized how I think about GUI code and turned GUI programming from something I hated to do to something I now enjoy.

### Name

The name of the library and the project is "egui" and pronounced as "e-gooey".

The library was originally called "Emigui", but was renamed to "egui" in 2020.

### Credits / Licenses

egui author: Emil Ernerfeldt

egui is under MIT OR Apache-2.0 license.

Fonts:

* `emoji-icon-font.ttf`: [Copyright (c) 2014 John Slegers](https://github.com/jslegers/emoji-icon-font) , MIT License
* `NotoEmoji-Regular.ttf`: [google.com/get/noto](https://google.com/get/noto), [SIL Open Font License](https://scripts.sil.org/cms/scripts/page.php?site_id=nrsi&id=OFL)
* `ProggyClean.ttf`: Copyright (c) 2004, 2005 Tristan Grimmer. MIT License. <http://www.proggyfonts.net/>
* `Ubuntu-Light.ttf` by [Dalton Maag](http://www.daltonmaag.com/): [Ubuntu font licence](https://ubuntu.com/legal/font-licence)
