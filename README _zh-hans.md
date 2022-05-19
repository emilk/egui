# 🖌 egui: 一个纯 Rust 编写的易用 GUI 库

[<img alt="github" src="https://img.shields.io/badge/github-emilk/egui-8da0cb?logo=github" height="20">](https://github.com/emilk/egui)
[![Latest version](https://img.shields.io/crates/v/egui.svg)](https://crates.io/crates/egui)
[![Documentation](https://docs.rs/egui/badge.svg)](https://docs.rs/egui)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![Build Status](https://github.com/emilk/egui/workflows/CI/badge.svg)](https://github.com/emilk/egui/actions?workflow=CI)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/emilk/egui/blob/master/LICENSE-MIT)
[![Apache](https://img.shields.io/badge/license-Apache-blue.svg)](https://github.com/emilk/egui/blob/master/LICENSE-APACHE)
[![Discord](https://img.shields.io/discord/900275882684477440?label=egui%20discord)](https://discord.gg/JFcEma9bJq)

语言：
[英文](https://github.com/emilk/egui/blob/master/README.md)
|
[简体中文](https://github.com/emilk/egui/blob/master/README_zh-hans.md)

👉 [点此运行 Web 样例](https://www.egui.rs/#demo) 👈

egui 是一个简单、快速、高度可移植的 Rust 即时模式 GUI 库。egui 可运行于 Web, 原生（*Native*） 甚至 [你喜欢的的游戏引擎](#integrations) （或者很快）。

egui 旨在成为最易用的 Rust GUI 库，用最简单的方式创建Web应用程序。

egui 可以在任何可以绘制纹理三角形（textured triangles）的地方使用，这意味着你可以轻松地地将它集成到你选择的游戏引擎中。

章节:

* [示例 Example](#示例)
* [快速上手](#快速上手)
* [样例 Demo](#样例)
* [目标](#目标)
* [egui 是为谁设计的？](#egui-是为谁设计的)
* [State / features](#state)
* [Integrations](#integrations)
* [Why immediate mode](#why-immediate-mode)
* [FAQ](#faq)
* [Other](#other)
* [Credits](#credits)

## 示例

``` rust
ui.heading("My egui Application");
ui.horizontal(|ui| {
    ui.label("Your name: ");
    ui.text_edit_singleline(&mut name);
});
ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
if ui.button("Click each year").clicked() {
    age += 1;
}
ui.label(format!("Hello '{}', age {}", name, age));
```

<img src="media/demo.gif">

## 快速上手

[范例目录](https://github.com/emilk/egui/blob/master/examples/)（`examples/`）中有一些简单的范例。如果你想写一个 Web App，请按照 <https://github.com/emilk/eframe_template/>的说明操作。官方文档位于 <https://docs.rs/egui>。要获得更多灵感或范例，请查看 [egui web 样例](https://www.egui.rs/#demo) 并按照其中的链接访问源代码。

如果你想要将egui集成到现有的引擎中，请前往  [集成](#集成) 一节.

如果有疑问，请访问 [GitHub Discussions](https://github.com/emilk/egui/discussions) 或 [egui discord 服务器](https://discord.gg/JFcEma9bJq)。如果你想贡献给 egui，请阅读 [Contributing Guidelines](https://github.com/emilk/egui/blob/master/CONTRIBUTING.md).

## 样例

[点此运行 Web 样例](https://www.egui.rs/#demo) （支持任何支持WASM和WebGL的浏览器）。使用 [`eframe`](https://github.com/emilk/egui/tree/master/eframe)。

若要在本地测试样例 App，运行 `cargo run --release -p egui_demo_app`。

原生后端是 [`egui_glow`](https://github.com/emilk/egui/tree/master/egui_glow)（使用 [`glow`](https://crates.io/crates/glow))，在 Windows 和 Mac 上开箱即用，但如果要在 Linux 上使用，需要先运行：

`sudo apt-get install -y libclang-dev libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libssl-dev`

在 Fedora Rawhide 上需要运行:

`dnf install clang clang-devel clang-tools-extra speech-dispatcher-devel libxkbcommon-devel pkg-config openssl-devel libxcb-devel`

**注意**: 这只针对样例 App —— egui 本身是完全平台无关的

## 目标

* 最易用的 GUI 库
* 反应敏捷的：在 debug build 中达到 60 Hz
* 友好的: 难以犯错，不应该发生 panic
* 可移植的：同样的代码可以在不同平台上使用
* 轻松集成到任意环境中
* 用于自定义绘制的简单 2D 图形 API（[`epaint`](https://docs.rs/epaint)）.
* 没有回调
* 纯即时模式
* 可扩展的：[轻松为 egui 编写自己的 widgets](https://github.com/emilk/egui/blob/master/egui_demo_lib/src/demo/toggle_switch.rs)
* 模块化的：你应该可以使用 egui 中的一小部分，并用新的方式将它们组合起来
* 安全的：egui 中没有`unsafe`关键字
* 依赖最小化：[`ab_glyph`](https://crates.io/crates/ab_glyph) [`ahash`](https://crates.io/crates/ahash) [`nohash-hasher`](https://crates.io/crates/nohash-hasher) [`parking_lot`](https://crates.io/crates/parking_lot)

egui *不是*框架。egui 是供调用的库，而不是供编程的环境。

**注意**: egui 还没有实现所有上述目标！egui 仍在开发中。

### 非目标

* 成为最强大的 GUI 库
* 原生外观界面（*looking interface*）
* 高级灵活的布局（这与即时模式根本不兼容）

## egui 是为谁设计的？

egui 旨在成为想要以最简单的方式创建 GUI 或想要在游戏引擎中添加 GUI 的人的最佳选择。

如果你不用 Rust，egui 不适合你。如果你想要一个看起来原生的 GUI，egui 不适合你。如果你想要升级时不会损坏的东西（*something that doesn't break when you upgrade it*），egui 不适合你（暂时）。

但如果你想用 Rust 写一些交互式的东西，需要一个简单的 GUI，egui 可能会适合你。

### egui vs Dear ImGui

egui 的明显替代方案是 [`imgui-rs`](https://github.com/Gekkio/imgui-rs)，C++ 库 [Dear ImGui](https://github.com/ocornut/imgui) 的 Rust 封装。Dear ImGui 是一个很棒的库（也是 egui 的灵感来源），它有更多特性和打磨（*polish*）不过，egui为Rust用户提供了一些好处：

* egui 是纯 Rust 编写的
* egui 可以很方便的编译为 WASM
* egui 允许你使用原生Rust字符串类（`imgui-rs` 强制你对以零结尾的字符串使用恼人的宏和包装器）
* [Writing your own widgets in egui is simple](https://github.com/emilk/egui/blob/master/egui_demo_lib/src/demo/toggle_switch.rs)

egui 还尝试在一些小地方增加你的体验：

* 窗口会根据其内容自动调整大小
* 窗口会自动定位，以避免互相重叠。
* 一些微妙的动画使 egui 变得生动

综上所述:

* egui：纯 Rust、初生、激动人心、正在开发中
* Dear ImGui：特性丰富、经过良好测试、笨重的 Rust 集成

## 状态

egui 在活跃开发中。它做的不错，但缺少许多特性，接口仍在变化。新的版本会有破坏性的改变。

### 特性

*译者注：这一段个人认为不宜翻译。*

* Widgets: label, text button, hyperlink, checkbox, radio button, slider, draggable value, text editing, combo box, color picker
* Layouts: horizontal, vertical, columns, automatic wrapping
* Text editing: multiline, copy/paste, undo, emoji supports
* Windows: move, resize, name, minimize and close. Automatically sized and positioned.
* Regions: resizing, vertical scrolling, collapsing headers (sections)
* Rendering: Anti-aliased rendering of lines, circles, text and convex polygons.
* Tooltips on hover
* More

<img src="media/widget_gallery.gif" width="50%">

Light Theme:

<img src="media/light_theme.png" width="50%">

## 集成

egui 易于集成到任何你使用的游戏引擎或平台
egui 自身不知道且不关心运行它的操作系统和被渲染到屏幕的方式——这是egui集成的工作

一个集成需要在每一帧都做以下事情：

* **输入**: 采集输入（鼠标、触摸、键盘、屏幕大小……）并传递给 egui
* 运行应用程序代码
* **输出**: 处理 egui 输出 （光标变化、粘贴、纹理分配（*texture allocations*）……）

* **绘制**：渲染 egui 生成的三角形网格（参考 [OpenGL example](https://github.com/emilk/egui/blob/master/egui_glium/src/painter.rs)）

### 官方集成

*译者注：个人认为仓库列表不应该翻译。*

以下是 egui 官方集成：

* [`eframe`](https://github.com/emilk/egui/tree/master/eframe) for compiling the same app to web/wasm and desktop/native. Uses `egui_glow` and `egui-winit`.
* [`egui_glium`](https://github.com/emilk/egui/tree/master/egui_glium) for compiling native apps with [Glium](https://github.com/glium/glium).
* [`egui_glow`](https://github.com/emilk/egui/tree/master/egui_glow) for rendering egui with [glow](https://github.com/grovesNL/glow) on native and web, and for making native apps.
* [`egui-wgpu`](https://github.com/emilk/egui/tree/master/egui-wgpu) for [wgpu](https://crates.io/crates/wgpu) (WebGPU API).
* [`egui-winit`](https://github.com/emilk/egui/tree/master/egui-winit) for integrating with [winit](https://github.com/rust-windowing/winit).

### 第三方集成

*译者注：个人认为仓库列表不应该翻译。*

* [`amethyst_egui`](https://github.com/jgraef/amethyst_egui) for [the Amethyst game engine](https://amethyst.rs/).
* [`bevy_egui`](https://github.com/mvlabat/bevy_egui) for [the Bevy game engine](https://bevyengine.org/).
* [`egui_glfw_gl`](https://github.com/cohaereo/egui_glfw_gl) for [GLFW](https://crates.io/crates/glfw).
* [`egui_sdl2_gl`](https://crates.io/crates/egui_sdl2_gl) for [SDL2](https://crates.io/crates/sdl2).
* [`egui_vulkano`](https://github.com/derivator/egui_vulkano) for [Vulkano](https://github.com/vulkano-rs/vulkano).
* [`egui_winit_vulkano`](https://github.com/hakolao/egui_winit_vulkano) for [Vulkano](https://github.com/vulkano-rs/vulkano).
* [`egui-macroquad`](https://github.com/optozorax/egui-macroquad) for [macroquad](https://github.com/not-fl3/macroquad).
* [`egui-miniquad`](https://github.com/not-fl3/egui-miniquad) for [Miniquad](https://github.com/not-fl3/miniquad).
* [`egui-tetra`](https://crates.io/crates/egui-tetra) for [Tetra](https://crates.io/crates/tetra), a 2D game framework.
* [`egui-winit-ash-integration`](https://github.com/MatchaChoco010/egui-winit-ash-integration) for [winit](https://github.com/rust-windowing/winit) and [ash](https://github.com/MaikKlein/ash).
* [`fltk-egui`](https://crates.io/crates/fltk-egui) for [fltk-rs](https://github.com/fltk-rs/fltk-rs).
* [`ggez-egui`](https://github.com/NemuiSen/ggez-egui) for the [ggez](https://ggez.rs/) game framework.
* [`godot-egui`](https://github.com/setzer22/godot-egui) for [godot-rust](https://github.com/godot-rust/godot-rust).
* [`nannou_egui`](https://github.com/AlexEne/nannou_egui) for [nannou](https://nannou.cc).
* [`smithay-egui`](https://github.com/Smithay/smithay-egui) for [smithay](https://github.com/Smithay/smithay/).

没有你想要的集成？创建一个很容易！

### 编写你自己的 egui 集成

你需要采集 [`egui::RawInput`](https://docs.rs/egui/latest/egui/struct.RawInput.html) 并处理 [`egui::FullOutput`](https://docs.rs/egui/latest/egui/struct.FullOutput.html)。基本结构如下

``` rust
let mut egui_ctx = egui::CtxRef::default();

// Game loop:
loop {
    // Gather input (mouse, touches, keyboard, screen size, etc):
    let raw_input: egui::RawInput = my_integration.gather_input();
    let full_output = egui_ctx.run(raw_input, |egui_ctx| {
        my_app.ui(egui_ctx); // add panels, windows and widgets to `egui_ctx` here
    });
    let clipped_primitives = egui_ctx.tessellate(full_output.shapes); // creates triangles to paint

    my_integration.paint(&full_output.textures_delta, clipped_primitives);

    let platform_output = full_output.platform_output;
    my_integration.set_cursor_icon(platform_output.cursor_icon);
    if !platform_output.copied_text.is_empty() {
        my_integration.set_clipboard_text(platform_output.copied_text);
    }
    // See `egui::FullOutput` and `egui::PlatformOutput` for more
}
```

关于 OpenGl 后端请参考 [the `egui_glium` painter](https://github.com/emilk/egui/blob/master/egui_glium/src/painter.rs) 或 [the `egui_glow` painter](https://github.com/emilk/egui/blob/master/egui_glow/src/painter.rs).

### 调试你的集成

#### Things look jagged

* Turn off backface culling.

#### 文字看起来很模糊

* 确保在 egui 输入中设置了正确的 `pixels_per_point`。
* 确保纹理采样器未关闭半像素。尝试使用最邻近采样器来检查。

#### 窗口太透明或太暗

* egui 使用预乘 alpha，因此，请确保您的混合函数是 `(ONE, ONE_MINUS_SRC_ALPHA)`。
* 确保纹理采样器已钳制（`GL_CLAMP_TO_EDGE`）。
* egui 对所有混合使用线性颜色空间，因此
  * 使用sRGBA-aware纹理（如果可用）（例如 `GL_SRGB8_ALPHA8`).
    * Otherwise: remember to decode gamma in the fragment shader.
  * Decode the gamma of the incoming vertex colors in your vertex shader.
  * Turn on sRGBA/linear framebuffer if available (`GL_FRAMEBUFFER_SRGB`).
    * Otherwise: gamma-encode the colors before you write them again.


## Why immediate mode

`egui` is an [immediate mode GUI library](https://en.wikipedia.org/wiki/Immediate_mode_GUI), as opposed to a *retained mode* GUI library. The difference between retained mode and immediate mode is best illustrated with the example of a button: In a retained GUI you create a button, add it to some UI and install some on-click handler (callback). The button is retained in the UI, and to change the text on it you need to store some sort of reference to it. By contrast, in immediate mode you show the button and interact with it immediately, and you do so every frame (e.g. 60 times per second). This means there is no need for any on-click handler, nor to store any reference to it. In `egui` this looks like this: `if ui.button("Save file").clicked() { save(file); }`.

A more detailed description of immediate mode can be found [in the `egui` docs](https://docs.rs/egui/latest/egui/#understanding-immediate-mode).

There are advantages and disadvantages to both systems.

The short of it is this: immediate mode GUI libraries are easier to use, but less powerful.

### Advantages of immediate mode
#### Usability
The main advantage of immediate mode is that the application code becomes vastly simpler:

* You never need to have any on-click handlers and callbacks that disrupts your code flow.
* You don't have to worry about a lingering callback calling something that is gone.
* Your GUI code can easily live in a simple function (no need for an object just for the UI).
* You don't have to worry about app state and GUI state being out-of-sync (i.e. the GUI showing something outdated), because the GUI isn't storing any state - it is showing the latest state *immediately*.

In other words, a whole lot of code, complexity and bugs are gone, and you can focus your time on something more interesting than writing GUI code.

### Disadvantages of immediate mode

#### Layout
The main disadvantage of immediate mode is it makes layout more difficult. Say you want to show a small dialog window in the center of the screen. To position the window correctly the GUI library must first know the size of it. To know the size of the window the GUI library must first layout the contents of the window. In retained mode this is easy: the GUI library does the window layout, positions the window, then checks for interaction ("was the OK button clicked?").

In immediate mode you run into a paradox: to know the size of the window, we must do the layout, but the layout code also checks for interaction ("was the OK button clicked?") and so it needs to know the window position *before* showing the window contents. This means we must decide where to show the window *before* we know its size!

This is a fundamental shortcoming of immediate mode GUIs, and any attempt to resolve it comes with its own downsides.

One workaround is to store the size and use it the next frame. This produces a frame-delay for the correct layout, producing occasional flickering the first frame something shows up. `egui` does this for some things such as windows and grid layouts.

You can also call the layout code twice (once to get the size, once to do the interaction), but that is not only more expensive, it's also complex to implement, and in some cases twice is not enough. `egui` never does this.

For "atomic" widgets (e.g. a button) `egui` knows the size before showing it, so centering buttons, labels etc is possible in `egui` without any special workarounds.

#### CPU usage
Since an immediate mode GUI does a full layout each frame, the layout code needs to be quick. If you have a very complex GUI this can tax the CPU. In particular, having a very large UI in a scroll area (with very long scrollback) can be slow, as the content needs to be layed out each frame.

If you design the GUI with this in mind and refrain from huge scroll areas (or only lay out the part that is in view) then the performance hit is generally pretty small. For most cases you can expect `egui` to take up 1-2 ms per frame, but `egui` still has a lot of room for optimization (it's not something I've focused on yet). You can also set up `egui` to only repaint when there is interaction (e.g. mouse movement).

If your GUI is highly interactive, then immediate mode may actually be more performant compared to retained mode. Go to any web page and resize the browser window, and you'll notice that the browser is very slow to do the layout and eats a lot of CPU doing it. Resize a window in `egui` by contrast, and you'll get smooth 60 FPS at no extra CPU cost.


#### IDs
There are some GUI state that you want the GUI library to retain, even in an immediate mode library such as `egui`. This includes position and sizes of windows and how far the user has scrolled in some UI. In these cases you need to provide `egui` with a seed of a unique identifier (unique within the parent UI). For instance: by default `egui` uses the window titles as unique IDs to store window positions. If you want two windows with the same name (or one window with a dynamic name) you must provide some other ID source to `egui` (some unique integer or string).

`egui` also needs to track which widget is being interacted with (e.g. which slider is being dragged). `egui` uses unique id:s for this awell, but in this case the IDs are automatically generated, so there is no need for the user to worry about it. In particular, having two buttons with the same name is no problem (this is in contrast with [`Dear ImGui`](https://github.com/ocornut/imgui)).

Overall, ID handling is a rare inconvenience, and not a big disadvantage.


## FAQ

Also see [GitHub Discussions](https://github.com/emilk/egui/discussions/categories/q-a).

### Can I use `egui` with non-latin characters?
Yes! But you need to install your own font (`.ttf` or `.otf`) using `Context::set_fonts`.

### Can I customize the look of egui?
Yes! You can customize the colors, spacing, fonts and sizes of everything using `Context::set_style`.

Here is an example (from https://github.com/AlexxxRu/TinyPomodoro):

<img src="media/pompodoro-skin.png" width="50%">

### How do I use egui with `async`?
If you call `.await` in your GUI code, the UI will freeze, which is very bad UX. Instead, keep the GUI thread non-blocking and communicate with any concurrent tasks (`async` tasks or other threads) with something like:
* Channels (e.g. [`std::sync::mpsc::channel`](https://doc.rust-lang.org/std/sync/mpsc/fn.channel.html)). Make sure to use [`try_recv`](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.try_recv) so you don't block the gui thread!
* `Arc<Mutex<Value>>` (background thread sets a value; GUI thread reads it)
* [`poll_promise::Promise`](https://docs.rs/poll-promise) (example: [`examples/download_image/`](https://github.com/emilk/egui/blob/master/examples/download_image/))
* [`eventuals::Eventual`](https://docs.rs/eventuals/latest/eventuals/struct.Eventual.html)
* [`tokio::sync::watch::channel`](https://docs.rs/tokio/latest/tokio/sync/watch/fn.channel.html)

### What about accessibility, such as screen readers?
There is experimental support for a screen reader. In [the web demo](https://www.egui.rs/#demo) you can enable it in the "Backend" tab.

Read more at <https://github.com/emilk/egui/issues/167>.

### What is the difference between [egui](https://docs.rs/egui) and [eframe](https://github.com/emilk/egui/tree/master/eframe)?

`egui` is a 2D user interface library for laying out and interacting with buttons, sliders, etc.
`egui` has no idea if it is running on the web or natively, and does not know how to collect input or show things on screen.
That is the job of *the integration* or *backend*.

It is common to use `egui` from a game engine (using e.g. [`bevy_egui`](https://docs.rs/bevy_egui)),
but you can also use `egui` stand-alone using `eframe`. `eframe` has integration for web and native, and handles input and rendering.
The _frame_ in `eframe` stands both for the frame in which your egui app resides and also for "framework" (`frame` is a framework, `egui` is a library).

### How do I render 3D stuff in an egui area?
There are multiple ways to combine egui with 3D. The simplest way is to use a 3D library and have egui sit on top of the 3D view. See for instance [`bevy_egui`](https://github.com/mvlabat/bevy_egui) or [`three-d`](https://github.com/asny/three-d).

If you want to embed 3D into an egui view there are two options.

#### `Shape::Callback`
Examples:
* <https://github.com/emilk/egui/blob/master/examples/custom_3d_three-d.rs>
* <https://github.com/emilk/egui/blob/master/examples/custom_3d_glow.rs>

`Shape::Callback` will call your code when egui gets painted, to show anything using whatever the background rendering context is. When using [`eframe`](https://github.com/emilk/egui/tree/master/eframe) this will be [`glow`](https://github.com/grovesNL/glow). Other integrations will give you other rendering contexts, if they support `Shape::Callback` at all.

#### Render-to-texture
You can also render your 3D scene to a texture and display it using [`ui.image(…)`](https://docs.rs/egui/latest/egui/struct.Ui.html#method.image). You first need to convert the native texture to an [`egui::TextureId`](https://docs.rs/egui/latest/egui/enum.TextureId.html), and how to do this depends on the integration you use.

Examples:
* Using [`egui-miniquad`]( https://github.com/not-fl3/egui-miniquad): https://github.com/not-fl3/egui-miniquad/blob/master/examples/render_to_egui_image.rs
* Using [`egui_glium`](https://github.com/emilk/egui/tree/master/egui_glium): <https://github.com/emilk/egui/blob/master/egui_glium/examples/native_texture.rs>.


## Other

### Conventions and design choices

All coordinates are in screen space coordinates, with (0, 0) in the top left corner

All coordinates are in "points" which may consist of many physical pixels.

All colors have premultiplied alpha.

egui uses the builder pattern for construction widgets. For instance: `ui.add(Label::new("Hello").text_color(RED));` I am not a big fan of the builder pattern (it is quite verbose both in implementation and in use) but until Rust has named, default arguments it is the best we can do. To alleviate some of the verbosity there are common-case helper functions, like `ui.label("Hello");`.

Instead of using matching `begin/end` style function calls (which can be error prone) egui prefers to use `FnOnce` closures passed to a wrapping function. Lambdas are a bit ugly though, so I'd like to find a nicer solution to this. More discussion of this at <https://github.com/emilk/egui/issues/1004#issuecomment-1001650754>.

### Inspiration

The one and only [Dear ImGui](https://github.com/ocornut/imgui) is a great Immediate Mode GUI for C++ which works with many backends. That library revolutionized how I think about GUI code and turned GUI programming from something I hated to do to something I now enjoy.

### Name

The name of the library and the project is "egui" and pronounced as "e-gooey". Please don't write it as "EGUI".

The library was originally called "Emigui", but was renamed to "egui" in 2020.

## Credits

egui author and maintainer: Emil Ernerfeldt [(@emilk](https://github.com/emilk)).

Notable contributions by:

* [@n2](https://github.com/n2): [Mobile web input and IME support](https://github.com/emilk/egui/pull/253).
* [@optozorax](https://github.com/optozorax): [Arbitrary widget data storage](https://github.com/emilk/egui/pull/257).
* [@quadruple-output](https://github.com/quadruple-output): [Multitouch](https://github.com/emilk/egui/pull/306).
* [@EmbersArc](https://github.com/EmbersArc): [Plots](https://github.com/emilk/egui/pulls?q=+is%3Apr+author%3AEmbersArc).
* [@AsmPrgmC3](https://github.com/AsmPrgmC3): [Proper sRGBA blending for web](https://github.com/emilk/egui/pull/650).
* [@AlexApps99](https://github.com/AlexApps99): [`egui_glow`](https://github.com/emilk/egui/pull/685).
* [@mankinskin](https://github.com/mankinskin): [Context menus](https://github.com/emilk/egui/pull/543).
* [@t18b219k](https://github.com/t18b219k): [Port glow painter to web](https://github.com/emilk/egui/pull/868).
* [@danielkeller](https://github.com/danielkeller): [`Context` refactor](https://github.com/emilk/egui/pull/1050).
* And [many more](https://github.com/emilk/egui/graphs/contributors?type=a).

egui is licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

* The flattening algorithm for the cubic bezier curve and quadratic bezier curve is from [lyon_geom](https://docs.rs/lyon_geom/latest/lyon_geom/)

Default fonts:

* `emoji-icon-font.ttf`: [Copyright (c) 2014 John Slegers](https://github.com/jslegers/emoji-icon-font) , MIT License
* `Hack-Regular.ttf`: <https://github.com/source-foundry/Hack>, [MIT Licence](https://github.com/source-foundry/Hack/blob/master/LICENSE.md)
* `NotoEmoji-Regular.ttf`: [google.com/get/noto](https://google.com/get/noto), [SIL Open Font License](https://scripts.sil.org/cms/scripts/page.php?site_id=nrsi&id=OFL)
* `Ubuntu-Light.ttf` by [Dalton Maag](http://www.daltonmaag.com/): [Ubuntu font licence](https://ubuntu.com/legal/font-licence)
