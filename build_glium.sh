#!/bin/bash
set -eu

cargo fmt --all -- --check
cargo check --all-features
cargo clippy

cargo run --bin demo_glium --release
