[![Latest version](https://img.shields.io/crates/v/egui.svg)](https://crates.io/crates/egui)
[![Documentation](https://docs.rs/egui/badge.svg)](https://docs.rs/egui)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

# Egui
An immediate mode GUI library written in Rust. Works anywhere you can draw textured triangles.

## Goals:
* Lightweight
* Short, conveniant syntax
* Responsive (60 Hz without breaking a sweat)
* Portable
* Platform independent (the same code works on the web and as a native app)

## How it works:
Loop:
* Gather input: mouse, touches, screen size, ...
* Run application code (Immediate Mode GUI)
* Output is a triangle mesh
* Render with e.g. OpenGL

## Available backends:
Wherever you can render textured triangles you can use Egui.

* WebAssembly (`egui_web`) for making a web app. [Click to run](https://emilk.github.io/egui/index.html).
* [Glium](https://github.com/glium/glium) for native apps (see example_glium).
* [miniquad](https://github.com/not-fl3/emigui-miniquad) [web demo](https://not-fl3.github.io/miniquad-samples/emigui.html) [demo source](https://github.com/not-fl3/good-web-game/blob/master/examples/emigui.rs)

The same application code can thus be compiled to either into a native app or a web app.

## Demos
[Egui feature demo](https://emilk.github.io/egui/index.html), (partial) source: https://github.com/emilk/egui/blob/master/egui/src/demos/app.rs

[Hobogo: A small game using Egui](https://emilk.github.io/hobogo/index.html), source: https://github.com/emilk/hobogo

## State
Alpha state. It works, but is somewhat incomplete.

Features:

* Labels
* Buttons, checkboxes, radio buttons and sliders
* Horizontal or vertical layout
* Column layout
* Collapsible headers (sections)
* Windows
* Resizable regions
* Vertical scolling
* Simple text input
* Anti-aliased rendering of circles, rounded rectangles and lines.

## Conventions
* All coordinates are screen space coordinates, in locial "points" (which may consist of many physical pixels).
* All colors have premultiplied alpha

## Inspiration
The one and only [Dear ImGui](https://github.com/ocornut/imgui) is a great Immediate Mode GUI for C++ which works with many backends. That library revolutionized how I think about GUI code from something I hated to do to something I now like to do.

## Name
The name of the gui library is "Egui", written like that in text and as `egui` in code and pronounced as "e-gooey".

The library was originally called "Emigui", but was renamed to Egui in 2020.

## Credits / Licenses
Fonts:
* Comfortaa: Open Font License, see OFT.txt
* ProggyClean.ttf, Copyright (c) 2004, 2005 Tristan Grimmer. MIT License. http://www.proggyfonts.net/
* Roboto-Regular.ttf: Apache License, Version 2.0
