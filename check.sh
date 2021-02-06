#!/bin/bash
set -eu

cargo check --workspace --all-targets
cargo check --workspace --all-targets --all-features
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown --all-features
cargo fmt --all -- --check
CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets --all-features --  -D warnings -W clippy::all
cargo test --workspace --all-targets --all-features
cargo test --workspace --doc

# TODO: doesn't error, but at least prints a warning.
cargo doc -p emath -p epaint -p egui -p eframe -p epi -p egui_web -p egui_glium --lib --no-deps
cargo doc -p egui_web --target wasm32-unknown-unknown --lib --no-deps

# For finding bloat:
# cargo bloat --release --bin demo_glium -n 200 | rg egui

# what compiles slowly?
# cargo clean; cargo +nightly build -p egui -Z timings
