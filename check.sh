#!/bin/bash
set -eu

cargo check --workspace --all-targets --all-features --release
cargo fmt --all -- --check
CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets --all-features --  -D warnings -W clippy::all #-W clippy::pedantic -W clippy::restriction -W clippy::nursery
cargo test --workspace --all-targets --all-features


# For finding bloat:
# cargo bloat --release --bin demo_glium -n 200 | rg egui

# what compiles slowly?
# cargo clean; cargo +nightly build -p egui -Z timings
