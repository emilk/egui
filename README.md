# Egui
Highly portable immediate mode GUI library for Rust.

Simple, fast, work in progress

Made for games or for anyone who want to make their own GUI and share it easily on a web page or compile it natively.

Egui can be used anywhere you can draw textured triangles.


[![Latest version](https://img.shields.io/crates/v/egui.svg)](https://crates.io/crates/egui)
[![Documentation](https://docs.rs/egui/badge.svg)](https://docs.rs/egui)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)


Sections:
* [Demo](#demos)
* [Goals](#goals)
* [State / features](#state)
* [How it works](#how-it-works)
* [Backends](#backends)
* [Other](#other)


## Demo
[Click to run Egui web demo](https://emilk.github.io/egui/index.html). Partial demo source: https://github.com/emilk/egui/blob/master/egui/src/demos/app.rs

[Hobogo](https://emilk.github.io/hobogo/index.html): A small game I made using Egui. Source: https://github.com/emilk/hobogo

#### Example:

``` rust
Window::new("Debug").show(ui.ctx(), |ui| {
    ui.label(format!("Hello, world {}", 123));
    if ui.button("Save").clicked {
        my_save_function();
    }
    ui.text_edit(&mut my_string);
    ui.add(Slider::f32(&mut value, 0.0..=1.0).text("float"));
});
```

<img src="media/demo-2020-08-21.png" width="50%">


## Goals
* API: Simple and convenient
* Responsive: target 60 Hz in debug build
* Portable: the same code works on the web and as a native app
* Friendly: difficult to make mistakes
* Easy to integrate into a any environment
* A simple 2D graphics API for custom painting
* Simple: no callbacks, minimal dependencies, avoid unnecessary monomorphization

Egui is *not* a framework. Egui is a library you call into, not an environment you program for.


## State
Alpha state. It works well for what it does, but it lacks many features and the interfaces are still in flux. New releases will have breaking changes.

### Features:

* Widgets: label, text button, hyperlink, checkbox, radio button, slider, draggable value, text editing
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
* Render the triangle mesh with your favorite graphics API (see OpenGL examples)


## Backends
Wherever you can render textured triangles you can use Egui.

### Official
I maintain two official Egui backends:

* [egui_web](crates.io/crates/egui_web) for making a web app. Compiles to WASM, renders with WebGL. [Click to run the Egui demo](https://emilk.github.io/egui/index.html).
* [egui_glium](crates.io/crates/egui_glium) for compiling native apps with [Glium](https://github.com/glium/glium) backend.

The same code can be compiled to a native app or a web app.

### 3rd party
* [emigui-miniquad](https://github.com/not-fl3/emigui-miniquad): backend for [Miniquad](https://github.com/not-fl3/miniquad). [Web demo](https://not-fl3.github.io/miniquad-samples/emigui.html) and [demo source](https://github.com/not-fl3/good-web-game/blob/master/examples/emigui.rs).

### Writing your own Egui backend
You need to collect `egui::RawInput`, paint `egui::PaintJobs` and handle `egui::Output`. The basic structure is this:

``` rust
let mut egui_ctx = egui::Context::new();

// game loop:
loop {
    let raw_input: egui::RawInput = my_backend.gather_input();
    let mut ui = egui_ctx.begin_frame(raw_input);
    my_app.ui(&mut ui); // add windows and widgets to `ui` here
    let (output, paint_jobs) = egui_ctx.end_frame();
    my_backend.paint(paint_jobs);
    my_backend.set_cursor_icon(output.cursor_icon);
    // Also see `egui::Output` for more
}
```


## Other
### Conventions
* All coordinates are screen space coordinates, in logical "points" (which may consist of many physical pixels). Origin (0, 0) is top left.
* All colors have premultiplied alpha

### Inspiration
The one and only [Dear ImGui](https://github.com/ocornut/imgui) is a great Immediate Mode GUI for C++ which works with many backends. That library revolutionized how I think about GUI code and turned GUI programming from something I hated to do to something I now enjoy.

#### Differences between Egui and Dear ImGui
Dear ImGui has has many years of development and so of course has more features. It has also been heavily optimized for speed, which Egui has not yet been.

Where Dear ImGui uses matching `Begin/End` style function calls, which can be error prone. Egui prefers to use lambdas passed to a wrapping function. Lambdas are a bit ugly though, so I'd like to find a nicer solution to this.

Egui uses the builder pattern for construction widgets. For instance: `ui.add(Label::new("Hello").text_color(RED));` I am not a big fan of the builder pattern (it is quite verbose both in implementation and in use) but until we have named, default arguments it is the best we can do. To alleviate some of the verbosity there are common case helper functions, like `ui.label("Hello");`.

### Name
The name of the library and the project is "Egui" and pronounced as "e-gooey".

The library was originally called "Emigui", but was renamed to Egui in 2020.

### Credits / Licenses
Egui author: Emil Ernerfeldt

Egui is under MIT OR Apache-2.0 license.

Fonts:
* Comfortaa: Open Font License, see OFT.txt
* ProggyClean.ttf, Copyright (c) 2004, 2005 Tristan Grimmer. MIT License. http://www.proggyfonts.net/
* Roboto-Regular.ttf: Apache License, Version 2.0
