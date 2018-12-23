# Emgui – An Experimental, Modularized immediate mode Graphical User Interface

Here are the steps, in chronological order of execution:

CODE: Input bindings, i.e. gathering GuiInput data from the system (web browser, Mac Window, iPhone App, ...)
DATA: GuiInput: mouse and keyboard state + window size
DATA: GuiSizes: this is a configuration of the ImLayout system, sets sizes of e.g. a slider.
CODE: ImLayout: Immediate mode layout Gui elements. THIS IS WHAT YOUR APP CODE CALLS!
DATA: GuiPaint: High-level commands to render e.g. a checked box with a hover-effect at a certain position.
DATA: GuiStyle: The colors/shading of the gui.
CODE: GuiPainter: Renders GuiPaint + GuiStyle into DrawCommands
DATA: PaintCommands: low-level commands (e.g. "Draw a rectangle with this color here")
CODE: Painter: paints the the PaintCommands to the screen (HTML canvas, OpenGL, ...)

This is similar to Dear ImGui but separates the layout from the rendering, and adds another step to the rendering.

# Implementation

Input is gathered in TypeScript.
PaintCommands rendered to a HTML canvas.
Everything else is written in Rust, compiled to WASM.
