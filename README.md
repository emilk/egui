# Emigui
Experimental, Modularized Immediate mode Graphical User Interface

A GUI library written in Rust, compiled to WASM. Inspired by game tech.

## How it works:

Loop:
* Gather input: mouse, touches, screen size, ...
* Run app code (Immediate Mode GUI)
* Output is a triangle mesh
* Render with WebGL

## Demos
[Emigui feature demo](https://emilk.github.io/emigui/index.html)

[Hobogo: A small game using Emigui](https://emilk.github.io/hobogo/index.html)

## State
More of a tech demo than anything else. Features:

* Buttons
* Sliders
* Text
* Horizontal or vertical layout
* Columns
* Collapsible headers

## Inspiration
[Dear ImGui](https://github.com/ocornut/imgui)

## Credits / Licenses
ProggyClean.ttf, Copyright (c) 2004, 2005 Tristan Grimmer. MIT License. http://www.proggyfonts.net/
Roboto-Regular.ttf: Apache License, Version 2.0
