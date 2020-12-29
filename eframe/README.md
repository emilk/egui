# Egui Framework

This aims to be the entry-level crate if you want to write an Egui App.

`eframe` calls into your code (it is a framework) and supports web apps (via `egui_web`) and native apps (via `egui_glium`).

`eframe` is a very thin crate that re-exports `egui`, `epi` and thin wrappers over the backends.
