# egui framework

This aims to be the entry-level crate if you want to write an egui app.

`eframe` calls into your code (it is a framework) and supports web apps (via [`egui_web`](https://crates.io/crates/egui_web)) and native apps (via [`egui_glium`](https://crates.io/crates/egui_glium)).

`eframe` is a very thin crate that re-exports [`egui`](https://crates.io/crates/egui), [`epi`](https://crates.io/crates/epi) and thin wrappers over the backends.

On Linux you need to first run `sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev` to compile `eframe` natively.

## Name

The _frame_ in `eframe` stands both for the frame in which your egui app resides and also for "framework" (`frame` is a framework, `egui` is a library).
