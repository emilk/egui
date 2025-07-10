#!/usr/bin/env bash

(cd crates/emath                && cargo publish --quiet)  &&  echo "✅ emath"
(cd crates/ecolor               && cargo publish --quiet)  &&  echo "✅ ecolor"
(cd crates/epaint_default_fonts && cargo publish --quiet)  &&  echo "✅ epaint_default_fonts"
(cd crates/epaint               && cargo publish --quiet)  &&  echo "✅ epaint"
(cd crates/egui                 && cargo publish --quiet)  &&  echo "✅ egui"
(cd crates/egui-winit           && cargo publish --quiet)  &&  echo "✅ egui-winit"
(cd crates/egui_glow            && cargo publish --quiet)  &&  echo "✅ egui_glow"
(cd crates/egui-wgpu            && cargo publish --quiet)  &&  echo "✅ egui-wgpu"
(cd crates/eframe               && cargo publish --quiet)  &&  echo "✅ eframe"
(cd crates/egui_kittest         && cargo publish --quiet)  &&  echo "✅ egui_kittest"
(cd crates/egui_extras          && cargo publish --quiet)  &&  echo "✅ egui_extras"
(cd crates/egui_demo_lib        && cargo publish --quiet)  &&  echo "✅ egui_demo_lib"
