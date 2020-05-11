# Emigui
(Experimental, Modularized Immediate mode Graphical User Interface)

An immediate mode GUI library written in Rust. For web apps or native apps.

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
Wherever you can render textured triangles you can use Emigui.

Currently two backends have been tested:
* WebAssembly (emigui_wasm) for making a web app
* [Glium](https://github.com/glium/glium) for native apps (see example_glium).
* [miniquad](https://github.com/not-fl3/emigui-miniquad) [web demo](https://not-fl3.github.io/miniquad-samples/emigui.html) [demo source](https://github.com/not-fl3/good-web-game/blob/master/examples/emigui.rs)

The same application code can thus be compiled to either into a native app or a web app.

## Demos
[Emigui feature demo](https://emilk.github.io/emigui/index.html), (partial) source: https://github.com/emilk/emigui/blob/master/emigui/src/example_app.rs

[Hobogo: A small game using Emigui](https://emilk.github.io/hobogo/index.html), source: https://github.com/emilk/hobogo

## State
Mostly a tech demo at this point. I hope to find time to work more on this in the future.

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

## Credits / Licenses
Fonts:
* Comfortaa: Open Font License, see OFT.txt
* ProggyClean.ttf, Copyright (c) 2004, 2005 Tristan Grimmer. MIT License. http://www.proggyfonts.net/
* Roboto-Regular.ttf: Apache License, Version 2.0
