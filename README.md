# Emigui
(Experimental, Modularized Immediate mode Graphical User Interface)

An immediate mode GUI library written in Rust. For web apps or native apps.

## Goals:
* Easy to use
* Platform independent (the same code works on the web and as a native app)
* Responsive

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

The same application code can thus be compiled to either into a native app or a web app.

## Demos
[Emigui feature demo](https://emilk.github.io/emigui/index.html), (partial) source: https://github.com/emilk/emigui/blob/master/emigui/src/example_app.rs

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
* Turn the [Glium](https://github.com/glium/glium) backend into a library
* Some examples and documentation
* Text input

## Inspiration
[Dear ImGui](https://github.com/ocornut/imgui) is a great Immediate Mode GUI for C++ which works with many backends.

## Credits / Licenses
Fonts:
* Comfortaa: Open Font License, see OFT.txt
* ProggyClean.ttf, Copyright (c) 2004, 2005 Tristan Grimmer. MIT License. http://www.proggyfonts.net/
* Roboto-Regular.ttf: Apache License, Version 2.0
