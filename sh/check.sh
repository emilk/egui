#!/bin/bash
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."
set -eux

# Checks all tests, lints etc.
# Basically does what the CI does.

cargo check --workspace --all-targets
cargo test --workspace --doc
cargo check --workspace --all-targets --all-features
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown --all-features
cargo clippy --workspace --all-targets --all-features --  -D warnings -W clippy::all
cargo test --workspace --all-targets --all-features
cargo fmt --all -- --check

cargo doc -p emath -p epaint -p egui -p eframe -p epi -p egui_web -p egui-winit -p egui_glium -p egui_glow --lib --no-deps --all-features
cargo doc -p egui_web --target wasm32-unknown-unknown --lib --no-deps --all-features

(cd emath && cargo check --no-default-features)
(cd epaint && cargo check --no-default-features --features "single_threaded")
(cd egui && cargo check --no-default-features --features "multi_threaded")
(cd eframe && cargo check --no-default-features --features "egui_glow")
(cd epi && cargo check --no-default-features)
(cd egui_web && cargo check --no-default-features)
(cd egui-winit && cargo check --no-default-features)
(cd egui_glium && cargo check --no-default-features)

# ------------------------------------------------------------
#

# For finding bloat:
# cargo bloat --release --bin demo_glium -n 200 | rg egui

# what compiles slowly?
# cargo clean; cargo +nightly build -p egui -Z timings

# what compiles slowly?
# cargo llvm-lines --lib -p egui | head -20
