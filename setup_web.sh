#!/bin/bash
set -eu

# Pre-requisites:
rustup target add wasm32-unknown-unknown
cargo install -f wasm-bindgen-cli
cargo update -p wasm-bindgen
