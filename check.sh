#!/bin/bash
set -eu

export RUSTFLAGS=--cfg=web_sys_unstable_apis # required for the clipboard API

cargo check --workspace --all-targets --all-features --release
cargo fmt --all -- --check
CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets --all-features --  -D warnings -W clippy::all #-W clippy::pedantic -W clippy::restriction -W clippy::nursery
cargo test --workspace --all-targets --all-features
cargo test --workspace --doc

cargo check -p egui --lib --target wasm32-unknown-unknown
cargo check -p egui_web --lib --target wasm32-unknown-unknown
cargo check -p egui_demo --lib --target wasm32-unknown-unknown
cargo check -p example_web --lib --target wasm32-unknown-unknown

# For finding bloat:
# cargo bloat --release --bin demo_glium -n 200 | rg egui

# what compiles slowly?
# cargo clean; cargo +nightly build -p egui -Z timings
