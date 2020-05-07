#!/bin/bash
set -eu

cargo fmt --all -- --check
cargo check --all-features
cargo clean -p emigui && cargo clippy

cargo run --bin example_glium --release
