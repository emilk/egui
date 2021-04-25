#!/bin/bash
set -eu

# Checks all tests, lints etc.
# Basically does what the CI does.

cargo check --workspace --all-targets
cargo test --workspace --doc
cargo check --workspace --all-targets --all-features
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown
cargo check -p egui_demo_app --lib --target wasm32-unknown-unknown --all-features
CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets --all-features --  -D warnings -W clippy::all
cargo test --workspace --all-targets --all-features
cargo fmt --all -- --check

cargo doc -p emath -p epaint -p egui -p eframe -p epi -p egui_web -p egui_glium --lib --no-deps --all-features
cargo doc -p egui_web --target wasm32-unknown-unknown --lib --no-deps --all-features

# ------------------------------------------------------------
#

# For finding bloat:
# cargo bloat --release --bin demo_glium -n 200 | rg egui

# what compiles slowly?
# cargo clean; cargo +nightly build -p egui -Z timings
