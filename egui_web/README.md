# egui_web

[![Latest version](https://img.shields.io/crates/v/egui_web.svg)](https://crates.io/crates/egui_web)
[![Documentation](https://docs.rs/egui_web/badge.svg)](https://docs.rs/egui_web)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Apache](https://img.shields.io/badge/license-Apache-blue.svg)

This crates allows you to compile GUI code written with [egui](https://crates.io/crates/egui) to [WASM](https://en.wikipedia.org/wiki/WebAssembly) to run on a web page.

[Run the web demo](https://emilk.github.io/egui/index.html) to try it now.

Check out [egui_template](https://github.com/emilk/egui_template) for an example of how to set it up.

## Downsides with using egui on the web

`egui_web` uses WebGL and WASM, and almost nothing else from the web tech stack. This has some benefits, but also produces some challanges and serious downsides.

* Rendering: Getting pixel-perfect rendering right on the web is very difficult, leading to text that is hard to read on low-DPI screens (https://github.com/emilk/egui/issues/516). Additonally, WebGL does not support linear framebuffer blending.
* Search: you cannot search a egui web page like you would a normal web page.
* Bringing up an on-screen keyboard on mobile: there is no JS function to do this, so `egui_web` fakes it by adding some invisible DOM elements. It doesn't always work.
* Mobile text editing is not as good as for a normal web app.
* Accessibility: There is an experimental screen reader for `egui_web`, but it has to be enabled explicitly. There is no JS function to ask "Does the user want a screen reader?" (and there should probably not be such a function, due to user tracking/integrity conserns).
* No integration with browser settings for colors and fonts.
* On Linux and Mac, Firefox will copy the WebGL render target from GPU, to CPU and then back again (https://bugzilla.mozilla.org/show_bug.cgi?id=1010527#c0), slowing down egui.

The suggested use for `egui_web` is for experiments, personal projects and web games. Using egui for a serious web page is probably a bad idea.

In many ways, `egui_web` is trying to make the browser do something it wasn't designed to do (though there are many things browser vendors could do to improve how well libraries like egui work).
