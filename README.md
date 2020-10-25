# Egui

[![Latest version](https://img.shields.io/crates/v/egui.svg)](https://crates.io/crates/egui)
[![Documentation](https://docs.rs/egui/badge.svg)](https://docs.rs/egui)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

Highly portable immediate mode GUI library for Rust.

Simple, fast, work in progress

Made for games or for anyone who want to make their own GUI and share it easily on a web page or compile it natively.

Egui can be used anywhere you can draw textured triangles.

Sections:

* [Demo](#demos)
* [Goals](#goals)
* [State / features](#state)
* [How it works](#how-it-works)
* [Integrations](#integrations)
* [Other](#other)

## Demo

[Click to run Egui web demo](https://emilk.github.io/egui/index.html). Partial demo source: <https://github.com/emilk/egui/blob/master/egui/src/demos/app.rs>

[Hobogo](https://emilk.github.io/hobogo/index.html): A small game I made using Egui. Source: <https://github.com/emilk/hobogo>

### Example

``` rust
ui.heading("My Egui Application");
ui.horizontal(|ui| {
    ui.label("Your name: ");
    ui.text_edit(&mut name);
});
ui.add(egui::Slider::u32(&mut age, 0..=120).text("age"));
if ui.button("Click each year").clicked {
    age += 1;
}
ui.label(format!("Hello '{}', age {}", name, age));
```

<img src="media/demo-2020-10-24.png" width="40%">

## Goals

* API: Simple and convenient (e.g. no lifetime arguments for [`Ui`](https://docs.rs/egui/latest/egui/struct.Ui.html)).
* Responsive: target 60 Hz in debug build
* Friendly: difficult to make mistakes
* Portable: the same code works on the web and as a native app
* Easy to integrate into any environment
* A simple 2D graphics API for custom painting
* No callbacks
* Pure immediate mode
* Extensible: [easy to write your own widgets for Egui](https://github.com/emilk/egui/blob/master/egui/src/demos/toggle_switch.rs)
* Modular: You should be able to use small parts of Egui and combine them in new ways
* Safe: there is no `unsafe` code in Egui
* Minimal dependencies
  * Egui uses [`rusttype`](https://crates.io/crates/rusttype) to render text and [`ahash`](https://crates.io/crates/ahash) + [`parking_lot`](https://crates.io/crates/parking_lot) for a speed boost.

Egui is *not* a framework. Egui is a library you call into, not an environment you program for.

**NOTE**: Egui does not claim to have reached all these goals yet! Egui is still work in progress.

### Why Egui?

Egui is written for Rust game engines. If you are not using Rust, Egui is not for you. If you want a GUI that looks native, Egui is not for you. If you want something stable that doesn't break when you upgrade it, Egui isn't for you (yet).

But if you are writing something interactive in Rust that needs a simple GUI, Egui may be for you.

The obvious alternative to Egui is [`imgui-rs`](https://github.com/Gekkio/imgui-rs), the Rust wrapper around the C++ library [Dear ImGui](https://github.com/ocornut/imgui). Dear ImGui is a great library, which a lot more features and polish compared to Egui. However, Egui provides some benefits for Rust users:

* Egui is pure Rust
* Egui is easily compiled to WASM
* Egui lets you use native Rust String types (`imgui-rs` forces you to use annoying macros and wrappers for zero-terminated strings)
* [Writing your own widgets in Egui is simple](https://github.com/emilk/egui/blob/master/egui/src/demos/toggle_switch.rs)

Egui also tries to improve your experience in other small ways:

* Windows are automatically sized based on their contents
* Windows are automatically positioned to not overlap with each other
* Some subtle animations make Egui come alive

So in summary:

* Egui: pure Rust, new, exciting, work in progress
* Dear ImGui: feature rich, well tested, cumbersome Rust integration

## State

Alpha state. It works well for what it does, but it lacks many features and the interfaces are still in flux. New releases will have breaking changes.

### Features

* Widgets: label, text button, hyperlink, checkbox, radio button, slider, draggable value, text editing, combo box, color picker
* Layouts: horizontal, vertical, columns
* Text input: very basic, multiline, copy/paste
* Windows: move, resize, name, minimize and close. Automatically sized and positioned.
* Regions: resizing, vertical scrolling, collapsing headers (sections)
* Rendering: Anti-aliased rendering of lines, circles, text and convex polygons.
* Tooltips on hover

## How it works

Loop:

* Gather input (mouse, touches, keyboard, screen size, etc) and give it to Egui
* Run application code (Immediate Mode GUI)
* Tell Egui to tesselate the frame graphics to a triangle mesh
* Render the triangle mesh with your favorite graphics API (see [OpenGL example](https://github.com/emilk/egui/blob/master/egui_glium/src/painter.rs))

## Integrations

Egui is build to be easy to integrate into any existing game engine or platform you are working on.
Egui itself doesn't know or care on what OS it is running or how to render things to the screen - that is the job of the egui integration.
The integration needs to do two things:

* **IO**: Supply Egui with input (mouse position, keyboard presses, ...) and handle Egui output (cursor changes, copy-paste integration, ...).
* **Painting**: Render the textured triangles that Egui outputs.

### Official

I maintain two official Egui integrations:

* [egui_web](https://crates.io/crates/egui_web) for making a web app. Compiles to WASM, renders with WebGL. [Click to run the Egui demo](https://emilk.github.io/egui/index.html).
* [egui_glium](https://crates.io/crates/egui_glium) for compiling native apps with [Glium](https://github.com/glium/glium).

The same code can be compiled to a native app or a web app.

### 3rd party

* [`wgpu`](https://crates.io/crates/wgpu) WebGPU API wrapper:
  * [egui_wgpu_backend](https://crates.io/crates/egui_wgpu_backend) with [example code](https://github.com/hasenbanck/egui_example)
  * Alternative: [egui_winit_wgpu](https://github.com/Gonkalbell/egui_winit_wgpu) (not available to crates.io)
* [emigui-miniquad](https://github.com/not-fl3/emigui-miniquad): backend for [Miniquad](https://github.com/not-fl3/miniquad). [Web demo](https://not-fl3.github.io/miniquad-samples/emigui.html) and [demo source](https://github.com/not-fl3/good-web-game/blob/master/examples/emigui.rs).
* [egui_winit_platform](https://github.com/hasenbanck/egui_winit_platform) provides bindings between [winit](https://crates.io/crates/winit) and Egui. It only provides the first half of an Egui integration (IO). Painting can be done with e.g. [egui_wgpu_backend](https://crates.io/crates/egui_wgpu_backend).

### Writing your own Egui integration

You need to collect [`egui::RawInput`](https://docs.rs/egui/latest/egui/struct.RawInput.html), paint [`egui::PaintJobs`](https://docs.rs/egui/latest/egui/paint/tessellator/type.PaintJobs.html) and handle [`egui::Output`](https://docs.rs/egui/latest/egui/struct.Output.html). The basic structure is this:

``` rust
let mut egui_ctx = egui::Context::new();

// Game loop:
loop {
    let raw_input: egui::RawInput = my_integration.gather_input();
    egui_ctx.begin_frame(raw_input);
    my_app.ui(&mut egui_ctx); // add panels, windows and widgets to `egui_ctx` here
    let (output, paint_jobs) = egui_ctx.end_frame();
    my_integration.paint(paint_jobs);
    my_integration.set_cursor_icon(output.cursor_icon);
    // Also see `egui::Output` for more
}
```

For a reference OpenGL backend, [see the `egui_glium` painter](https://github.com/emilk/egui/blob/master/egui_glium/src/painter.rs).

#### Debugging your integration

#### My text is blurry

* Make sure you set the proper `pixels_per_point` in the input to Egui.
* Make sure the texture sampler is not off by half a pixel. Try nearest-neighbor sampler to check.

#### My windows are too transparent or too dark

* Make sure your texture sampler is clamped.
* Make sure you consider sRGB (gamma) in your shaders.
* Egui uses premultiplied alpha, so make sure your blending function is `(ONE, ONE_MINUS_SRC_ALPHA)`

## Other

### Conventions and design choices

All coordinates are in screen space coordinates, with (0, 0) in the top left corner

All coordinates are in locial "points" which may consist of many physical pixels.

All colors have premultiplied alpha.

Egui uses the builder pattern for construction widgets. For instance: `ui.add(Label::new("Hello").text_color(RED));` I am not a big fan of the builder pattern (it is quite verbose both in implementation and in use) but until Rust has named, default arguments it is the best we can do. To alleviate some of the verbosity there are common-case helper functions, like `ui.label("Hello");`.

Instead of using matching `begin/end` style function calls (which can be error prone) Egui prefers to use `FnOnce` closures passed to a wrapping function. Lambdas are a bit ugly though, so I'd like to find a nicer solution to this.

### Inspiration

The one and only [Dear ImGui](https://github.com/ocornut/imgui) is a great Immediate Mode GUI for C++ which works with many backends. That library revolutionized how I think about GUI code and turned GUI programming from something I hated to do to something I now enjoy.

### Name

The name of the library and the project is "Egui" and pronounced as "e-gooey".

The library was originally called "Emigui", but was renamed to Egui in 2020.

### Credits / Licenses

Egui author: Emil Ernerfeldt

Egui is under MIT OR Apache-2.0 license.

Fonts:

* Comfortaa: Open Font License, see OFT.txt
* ProggyClean.ttf, Copyright (c) 2004, 2005 Tristan Grimmer. MIT License. <http://www.proggyfonts.net/>
* Roboto-Regular.ttf: Apache License, Version 2.0
