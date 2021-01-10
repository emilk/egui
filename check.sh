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

# For finding bloat:
# cargo bloat --release --bin demo_glium -n 200 | rg egui

# what compiles slowly?
# cargo clean; cargo +nightly build -p egui -Z timings
