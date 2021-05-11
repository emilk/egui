#!/bin/bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

cargo doc -p egui_web --target wasm32-unknown-unknown --lib --no-deps --all-features
cargo doc -p emath -p epaint -p egui -p eframe -p epi -p egui_web -p egui_glium --lib --no-deps --all-features --open

# cargo watch -c -x 'doc -p emath -p epaint -p egui --lib --no-deps --all-features'
