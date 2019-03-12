# Emigui
(Experimental, Modularized Immediate mode Graphical User Interface)

An immediate mode GUI library written in Rust. Compiles to WASM.

## Goals:
* Easy to use
* Platform independent (the same code should run on web and native)
* Responsive

## How it works:
Loop:
* Gather input: mouse, touches, screen size, ...
* Run application code (Immediate Mode GUI)
* Output is a triangle mesh
* Render with WebGL

## Demos
[Emigui feature demo](https://emilk.github.io/emigui/index.html), source: https://github.com/emilk/emigui/blob/master/example/src/app.rs

[Hobogo: A small game using Emigui](https://emilk.github.io/hobogo/index.html), source: https://github.com/emilk/hobogo

## State
Mostly a tech demo at this point. I hope to find time to work more on this in the future.

Features:

* Text
* Buttons, checkboxes, radio buttons and sliders
* Horizontal or vertical layout
* Column layout
* Collapsible headers (sections)
* Anti-aliased rendering of circles, rounded rectangles and lines.

## Roadmap:
* Native backend
* Some examples / documentation
* Text input

## Inspiration
[Dear ImGui](https://github.com/ocornut/imgui) is a great Immediate Mode GUI for C++ which works with many backends.

## Credits / Licenses
Fonts:
* ProggyClean.ttf, Copyright (c) 2004, 2005 Tristan Grimmer. MIT License. http://www.proggyfonts.net/
* Roboto-Regular.ttf: Apache License, Version 2.0
