#!/usr/bin/env bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

cargo doc -p eframe --target wasm32-unknown-unknown --lib --no-deps
cargo doc -p emath -p epaint -p egui -p eframe -p egui-winit -p egui_extras -p egui_glow --lib --no-deps --all-features --open

# cargo watch -c -x 'doc -p emath -p epaint -p egui --lib --no-deps --all-features'
