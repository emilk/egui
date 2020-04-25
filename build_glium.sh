#!/bin/bash
set -eu

cargo fmt --all -- --check
cargo check --all-features
cargo clippy

# ./build_wasm.sh
# open "docs/index.html"

cargo run --bin example_glium
