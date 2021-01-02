# Egui Framework

This aims to be the entry-level crate if you want to write an Egui App.

`eframe` calls into your code (it is a framework) and supports web apps (via [`egui_web`](https://crates.io/crates/egui_web)) and native apps (via [`egui_glium`](https://crates.io/crates/egui_glium)).

`eframe` is a very thin crate that re-exports [`egui`](https://crates.io/crates/egui), [`epi`](https://crates.io/crates/epi) and thin wrappers over the backends.
